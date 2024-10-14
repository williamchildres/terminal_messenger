use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};

use crate::app::{App, MessageType};
use crate::commander::command_handler::handle_command;

pub async fn websocket_task(app: Arc<Mutex<App>>, shutdown: broadcast::Sender<()>) {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("Server listening on {}", addr);

    let clients = Arc::new(Mutex::new(HashMap::<
        String,
        (Option<String>, mpsc::UnboundedSender<MessageType>),
    >::new()));
    let history = Arc::new(Mutex::new(VecDeque::<MessageType>::with_capacity(100)));

    loop {
        let mut shutdown_subscriber = shutdown.subscribe(); // Create shutdown receiver here and make it mutable
        tokio::select! {
            // Handle new WebSocket connections
            Ok((stream, _)) = listener.accept() => {
                let clients = clients.clone();
                let history = history.clone();
                let app = app.clone();
                let shutdown_subscriber = shutdown.subscribe(); // Each connection gets its own shutdown receiver

                tokio::spawn(handle_connection(stream, clients, history, app, shutdown_subscriber));
            }

            // Shutdown signal is received
            _ = shutdown_subscriber.recv() => {
                println!("Shutting down WebSocket task.");
                break;
            }
        }
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    clients: Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<MessageType>)>>>,
    history: Arc<Mutex<VecDeque<MessageType>>>,
    app: Arc<Mutex<App>>,
    mut shutdown: broadcast::Receiver<()>,
) {
    let ws_stream: WebSocketStream<_> = accept_async(stream).await.expect("Error during handshake");

    // Clone client_id and all Arc values to be used in async tasks
    let client_id = format!("{}", ws_stream.get_ref().peer_addr().unwrap());
    let client_id_clone = client_id.clone(); // Clone for use inside the async block
    let client_id_shutdown = client_id.clone(); // Separate clone for use in the shutdown message

    let clients_clone = clients.clone(); // Clone Arc to share with async tasks
    let history_clone = history.clone(); // Clone Arc for async tasks
    let app_clone = app.clone(); // Clone Arc to use within the async block

    let (tx_original, mut rx) = mpsc::unbounded_channel();

    {
        let mut clients_guard = clients.lock().await;
        clients_guard.insert(client_id.clone(), (None, tx_original.clone()));
    }

    // Send message history to the new client (using MessageType)
    for message in history.lock().await.iter() {
        tx_original.send(message.clone()).unwrap();
    }

    let (outgoing, mut incoming) = ws_stream.split();
    let outgoing = Arc::new(Mutex::new(outgoing)); // Wrap 'outgoing' in Arc<Mutex<_>>, this allows
                                                   // both ping_task and send_task to gain ownership of 'outgoing'

    // Task for sending periodic ping messages to the client
    let outgoing_clone = outgoing.clone();
    let ping_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30)); // Adjust interval as needed
        loop {
            interval.tick().await;
            let mut outgoing_lock = outgoing_clone.lock().await; // Acquire lock before sending ping
            if outgoing_lock.send(Message::Ping(vec![])).await.is_err() {
                break; // If sending fails, exit the task
            }
        }
    });

    // Task for sending messages from the server to the client
    let outgoing_clone = outgoing.clone();
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let serialized_message = serde_json::to_string(&message).unwrap();
            let mut outgoing_lock = outgoing_clone.lock().await; // Acquire lock before sending message
            if outgoing_lock
                .send(Message::Text(serialized_message))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Task for receiving messages from the client
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = incoming.next().await {
            // Deserialize incoming message into MessageType
            match serde_json::from_str::<MessageType>(&text) {
                Ok(message) => {
                    handle_incoming_message(
                        message,
                        &client_id_clone,
                        &clients_clone,
                        &app_clone,
                        &history_clone,
                    )
                    .await
                }
                Err(_) => println!("Invalid message format from client {}", client_id_clone),
            }
        }

        // Handle disconnection
        handle_disconnect(&client_id_clone, &clients_clone, app_clone).await;
    });

    // Wait for shutdown or any of the tasks to complete
    tokio::select! {
        _ = ping_task => {},
        _ = send_task => {},
        _ = recv_task => {},
        _ = shutdown.recv() => {
            println!("Shutdown received for client: {}", client_id_shutdown); // Using separate clone here
        }
    }

    // Handle disconnect after tasks are done
    handle_disconnect(&client_id, &clients, app).await;
}

async fn handle_disconnect(
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<MessageType>)>>>,
    app: Arc<Mutex<App>>,
) {
    // Remove the client from the active list
    let client_name = {
        let mut clients_guard = clients.lock().await;
        let client_name = clients_guard.remove(client_id).and_then(|(name, _)| name);
        client_name.unwrap_or_else(|| client_id.to_string()) // Use client_id if no name was set
    };

    // Update App state to remove the disconnected user
    app.lock().await.remove_connected_user(&client_id).await;

    // Send a system message about the disconnection
    let disconnect_message =
        MessageType::SystemMessage(format!("{} has disconnected.", client_name));

    // Broadcast the disconnection message to all remaining clients
    for (_, (_, tx)) in clients.lock().await.iter() {
        tx.send(disconnect_message.clone()).unwrap();
    }

    println!("{} has disconnected", client_name);
}

async fn handle_incoming_message(
    message: MessageType,
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<MessageType>)>>>,
    app: &Arc<Mutex<App>>,
    history: &Arc<Mutex<VecDeque<MessageType>>>,
) {
    match message {
        MessageType::ChatMessage { sender, content } => {
            println!("Chat message from {}: {}", sender, content);

            let client_name = clients
                .lock()
                .await
                .get(client_id)
                .and_then(|(name, _)| name.clone())
                .unwrap_or_else(|| client_id.to_string());

            let broadcast_message = MessageType::ChatMessage {
                sender: client_name.clone(),
                content: content.clone(),
            };

            // Add to history
            let mut history_guard = history.lock().await;
            if history_guard.len() == 100 {
                history_guard.pop_front();
            }
            history_guard.push_back(broadcast_message.clone());

            // Broadcast to all clients
            for (_, (_, tx)) in clients.lock().await.iter() {
                tx.send(broadcast_message.clone()).unwrap();
            }
        }

        MessageType::Command { name, args } => {
            // Handle commands (e.g., `/name`, `/list`)
            handle_command(name, args, client_id, clients, app.clone()).await;
        }

        MessageType::SystemMessage(system_message) => {
            println!("System message: {}", system_message);
        }
    }
}
