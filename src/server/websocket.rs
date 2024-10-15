//  This file contains functions related to handling WebSocket connections.
//  It includes a function for starting the WebSocket task,
//  handling individual connections, and processing incoming and outgoing messages.
//
//  Author: William Childres
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

    // Clone the necessary values
    let client_id = format!("{}", ws_stream.get_ref().peer_addr().unwrap());
    let client_id_shutdown = client_id.clone(); // Clone for use in shutdown handling

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
    let outgoing = Arc::new(Mutex::new(outgoing));

    let disconnect_handled = Arc::new(Mutex::new(false));

    // Task for sending periodic ping messages to the client
    let ping_task = {
        let outgoing_clone = Arc::clone(&outgoing);
        let client_id_clone = client_id.clone();
        let clients_clone = Arc::clone(&clients);
        let app_clone = Arc::clone(&app);
        let disconnect_handled_clone = Arc::clone(&disconnect_handled);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let mut outgoing_lock = outgoing_clone.lock().await;
                if outgoing_lock.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
            println!("Ping task ended for client: {}", client_id_clone);
            handle_disconnection(
                disconnect_handled_clone,
                &client_id_clone,
                &clients_clone,
                Arc::clone(&app_clone),
            )
            .await;
        })
    };

    // Task for sending messages from the server to the client
    let send_task = {
        let outgoing_clone = Arc::clone(&outgoing);
        let client_id_clone = client_id.clone();
        let clients_clone = Arc::clone(&clients);
        let app_clone = Arc::clone(&app);
        let disconnect_handled_clone = Arc::clone(&disconnect_handled);

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                let serialized_message = serde_json::to_string(&message).unwrap();
                let mut outgoing_lock = outgoing_clone.lock().await;
                if outgoing_lock
                    .send(Message::Text(serialized_message))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            println!("Send task ended for client: {}", client_id_clone);
            handle_disconnection(
                disconnect_handled_clone,
                &client_id_clone,
                &clients_clone,
                Arc::clone(&app_clone),
            )
            .await;
        })
    };

    // Task for receiving messages from the client
    let recv_task = {
        let client_id_clone = client_id.clone();
        let clients_clone = Arc::clone(&clients);
        let app_clone = Arc::clone(&app);
        let history_clone = Arc::clone(&history);
        let disconnect_handled_clone = Arc::clone(&disconnect_handled);

        tokio::spawn(async move {
            while let Some(result) = incoming.next().await {
                match result {
                    Ok(Message::Text(text)) => {
                        // Log the received message
                        println!("Received message from client {}: {}", client_id_clone, text);

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
                            Err(_) => {
                                println!("Invalid message format from client: {}", client_id_clone);
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => {
                        println!("Received Ping from client {}", client_id_clone);
                    }
                    Ok(Message::Pong(_)) => {
                        println!("Received Pong from client {}", client_id_clone);
                    }
                    Ok(_) => {
                        println!(
                            "Received other type of message from client {}",
                            client_id_clone
                        );
                    }
                    Err(e) => {
                        // Log any errors that occur
                        println!(
                            "Error receiving message from client {}: {}",
                            client_id_clone, e
                        );
                        break; // Break the loop on error
                    }
                }
            }
            println!("Recv task ended for client: {}", client_id_clone);
            handle_disconnection(
                disconnect_handled_clone,
                &client_id_clone,
                &clients_clone,
                Arc::clone(&app_clone),
            )
            .await;
        })
    };

    // Wait for shutdown or any task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = ping_task => {},
        _ = shutdown.recv() => {
            println!("Shutdown received for client: {}", client_id_shutdown);
        }
    }

    // Handle disconnect after tasks are done
    handle_disconnection(disconnect_handled, &client_id, &clients, app).await;
}

// Handle disconnection logic (only once)
async fn handle_disconnection(
    disconnect_handled: Arc<Mutex<bool>>,
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<MessageType>)>>>,
    app: Arc<Mutex<App>>,
) {
    let mut handled = disconnect_handled.lock().await;
    if *handled {
        return; // Disconnection already handled
    }
    *handled = true;

    let client_name = {
        let mut clients_guard = clients.lock().await;
        let client_name = clients_guard.remove(client_id).and_then(|(name, _)| name);
        client_name.unwrap_or_else(|| client_id.to_string())
    };

    app.lock().await.remove_connected_user(client_id).await;

    let disconnect_message =
        MessageType::SystemMessage(format!("{} has disconnected.", client_name));
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
