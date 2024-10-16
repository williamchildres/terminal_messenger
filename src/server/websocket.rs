//  This file contains functions related to handling WebSocket connections.
//  It includes a function for starting the WebSocket task,
//  handling individual connections, and processing incoming and outgoing messages.
//
//  Author: William Childres
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use uuid::Uuid; //  unique IDs for users

use crate::app::{App, MessageType};
use crate::commander::command_handler::handle_command;

pub async fn websocket_task(addr:SocketAddr, app: Arc<Mutex<App>>, shutdown: broadcast::Sender<()>) {
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("Server listening on {}", addr.to_string());

    let clients = Arc::new(Mutex::new(HashMap::<
        String,
        mpsc::UnboundedSender<MessageType>,
    >::new()));

    // Channel for sending messages to the batch processor
    let (batch_tx, batch_rx) = mpsc::channel(100);

    // Spawn the batch processing task
    tokio::spawn(batch_send_task(clients.clone(), batch_rx));

    loop {
        let mut shutdown_subscriber = shutdown.subscribe();
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                let clients = clients.clone();
                let app = app.clone();
                let shutdown_subscriber = shutdown.subscribe();

                tokio::spawn(handle_connection(stream, clients, app, shutdown_subscriber, batch_tx.clone())); // Pass the batch_tx to handle_connection
            }

            _ = shutdown_subscriber.recv() => {
                println!("Shutting down WebSocket task.");
                break;
            }
        }
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<MessageType>>>>,
    app: Arc<Mutex<App>>,
    mut shutdown: broadcast::Receiver<()>,
    batch_tx: mpsc::Sender<MessageType>, // Add batch_tx here
) {
    let ws_stream = accept_async(stream).await.expect("Error during handshake");

    let client_id = Uuid::new_v4().to_string();
    let (tx_original, mut rx) = mpsc::unbounded_channel();
    clients
        .lock()
        .await
        .insert(client_id.clone(), tx_original.clone());

    // Add the user to the App with default name
    app.lock()
        .await
        .add_connected_user(client_id.clone(), "Anonymous".to_string())
        .await;

    // Send message history to the new client from the App
    let history = app.lock().await.get_message_history().await;
    for message in history {
        tx_original.send(message.clone()).unwrap();
    }

    let (outgoing, mut incoming) = ws_stream.split();
    let outgoing = Arc::new(Mutex::new(outgoing));
    let disconnect_handled = Arc::new(Mutex::new(false));

    // Create a channel for ping task to detect pong responses
    let (pong_tx, mut pong_rx) = mpsc::channel(1); // Use bounded channel to ensure order

    // Ping task
    let ping_task = {
        let outgoing_clone = Arc::clone(&outgoing);
        let client_id_clone = client_id.clone();
        let clients_clone = Arc::clone(&clients);
        let app_clone = Arc::clone(&app);
        let disconnect_handled_clone = Arc::clone(&disconnect_handled);

        tokio::spawn(async move {
            let mut ping_interval = tokio::time::interval(Duration::from_secs(30)); // Ping every 30 seconds
            let pong_timeout = Duration::from_secs(10); // Wait 10 seconds for Pong

            loop {
                ping_interval.tick().await;

                let mut outgoing_lock = outgoing_clone.lock().await;

                // Send Ping message to the client
                if outgoing_lock.send(Message::Ping(vec![])).await.is_err() {
                    println!("Error sending Ping to client: {}", client_id_clone);
                    break;
                }
                drop(outgoing_lock); // Release the lock before waiting for Pong

                // Wait for Pong within the timeout period
                match timeout(pong_timeout, pong_rx.recv()).await {
                    Ok(Some(())) => {
                        println!("Pong received from client: {}", client_id_clone);
                    }
                    _ => {
                        println!(
                            "Client {} is unresponsive. Disconnecting...",
                            client_id_clone
                        );
                        handle_disconnection(
                            disconnect_handled_clone,
                            &client_id_clone,
                            &clients_clone,
                            Arc::clone(&app_clone),
                        )
                        .await;
                        break;
                    }
                }
            }
        })
    };

    // Task for sending messages
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
            handle_disconnection(
                disconnect_handled_clone,
                &client_id_clone,
                &clients_clone,
                Arc::clone(&app_clone),
            )
            .await;
        })
    };

    // Task for receiving messages and detecting Pong responses
    let recv_task = {
        let client_id_clone = client_id.clone();
        let clients_clone = Arc::clone(&clients);
        let app_clone = Arc::clone(&app);
        let disconnect_handled_clone = Arc::clone(&disconnect_handled);
        let pong_tx_clone = pong_tx.clone(); // Clone pong sender for use in task

        tokio::spawn(async move {
            while let Some(result) = incoming.next().await {
                match result {
                    Ok(Message::Text(text)) => match serde_json::from_str::<MessageType>(&text) {
                        Ok(message) => {
                            handle_incoming_message(
                                message,
                                &client_id_clone,
                                &clients_clone,
                                &app_clone, // Pass batch_tx to handle_incoming_message
                            )
                            .await;
                        }
                        Err(_) => {
                            println!("Invalid message format from client: {}", client_id_clone);
                        }
                    },
                    Ok(Message::Ping(_)) => {
                        println!("Received Ping from client {}", client_id_clone);
                    }
                    Ok(Message::Pong(_)) => {
                        // Notify ping task that Pong was received
                        let _ = pong_tx_clone.send(()).await;
                    }
                    Ok(_) => {
                        println!(
                            "Received other type of message from client {}",
                            client_id_clone
                        );
                    }
                    Err(e) => {
                        println!(
                            "Error receiving message from client {}: {}",
                            client_id_clone, e
                        );
                        break;
                    }
                }
            }
            handle_disconnection(
                disconnect_handled_clone,
                &client_id_clone,
                &clients_clone,
                Arc::clone(&app_clone),
            )
            .await;
        })
    };

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = ping_task => {},
        _ = shutdown.recv() => {
            println!("Shutdown received for client: {}", client_id);
        }
    }

    handle_disconnection(disconnect_handled, &client_id, &clients, app).await;
}

async fn handle_incoming_message(
    message: MessageType,
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<MessageType>>>>,
    app: &Arc<Mutex<App>>, // Batch processing sender
) {
    match message {
        MessageType::ChatMessage { sender: _, content } => {
            // Fetch username from App
            let client_name = app
                .lock()
                .await
                .get_connected_user(client_id)
                .await
                .unwrap()
                .lock()
                .await
                .username
                .clone();

            let broadcast_message = MessageType::ChatMessage {
                sender: client_name.clone(),
                content: content.clone(),
            };

            // Add message to history in App
            app.lock()
                .await
                .add_message_to_history(broadcast_message.clone())
                .await;

            // Broadcast to all clients
            let mut clients_lock = clients.lock().await;
            let disconnected_clients: Vec<String> = clients_lock
                .iter()
                .filter_map(|(id, tx)| {
                    if tx.send(broadcast_message.clone()).is_err() {
                        // If sending fails, mark this client as disconnected
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Remove disconnected clients
            for id in disconnected_clients {
                clients_lock.remove(&id);
                println!("Removed disconnected client: {}", id);
            }
        }

        MessageType::Command { name, args } => {
            handle_command(name, args, client_id, clients, app.clone()).await;
        }

        MessageType::SystemMessage(system_message) => {
            println!("System message: {}", system_message);
        }
    }
}

async fn batch_send_task(
    clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<MessageType>>>>,
    mut rx: mpsc::Receiver<MessageType>, // Receives messages for broadcasting
) {
    let mut message_batch = Vec::new(); // Buffer to store batched messages
    let batch_interval = Duration::from_millis(100); // Define batch interval (100ms)

    loop {
        tokio::select! {
            // Collect messages to batch
            Some(message) = rx.recv() => {
                message_batch.push(message);
            }
            _ = tokio::time::sleep(batch_interval) => {
                if !message_batch.is_empty() {
                    // If there are messages, broadcast them to all clients
                    let clients = clients.lock().await;
                    for (_, tx) in clients.iter() {
                        for message in &message_batch {
                            if tx.send(message.clone()).is_err() {
                                println!("Failed to send message to a client.");
                            }
                        }
                    }
                    message_batch.clear(); // Clear the batch after sending
                }
            }
        }
    }
}

async fn handle_disconnection(
    disconnect_handled: Arc<Mutex<bool>>,
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<MessageType>>>>,
    app: Arc<Mutex<App>>,
) {
    let mut handled = disconnect_handled.lock().await;
    if *handled {
        return; // Disconnection already handled
    }
    *handled = true;

    // Log and remove the user from the app
    let client_name = app
        .lock()
        .await
        .get_connected_user(client_id)
        .await
        .unwrap()
        .lock()
        .await
        .username
        .clone();

    app.lock().await.remove_connected_user(client_id).await;

    // Remove the client from the list of connected clients
    clients.lock().await.remove(client_id);

    // Broadcast that the user has disconnected
    let disconnect_message =
        MessageType::SystemMessage(format!("{} has disconnected.", client_name));
    for (_, tx) in clients.lock().await.iter() {
        // Send the message to all connected clients
        let _ = tx.send(disconnect_message.clone());
    }

    println!("{} has disconnected", client_name);
}
