//  This file contains functions related to handling WebSocket connections.
//  It includes a function for starting the WebSocket task,
//  handling individual connections, and processing incoming and outgoing messages.
//
//  Author: William Childres
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};
use uuid::Uuid; //  unique IDs for users

use crate::app::{App, MessageType};
use crate::commander::command_handler::handle_command;

pub async fn websocket_task(app: Arc<Mutex<App>>, shutdown: broadcast::Sender<()>) {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("Server listening on {}", addr);

    let clients = Arc::new(Mutex::new(HashMap::<
        String,
        mpsc::UnboundedSender<MessageType>,
    >::new()));

    loop {
        let mut shutdown_subscriber = shutdown.subscribe();
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                let clients = clients.clone();
                let app = app.clone();
                let shutdown_subscriber = shutdown.subscribe();

                tokio::spawn(handle_connection(stream, clients, app, shutdown_subscriber));
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
) {
    let ws_stream: WebSocketStream<_> = accept_async(stream).await.expect("Error during handshake");

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
            handle_disconnection(
                disconnect_handled_clone,
                &client_id_clone,
                &clients_clone,
                Arc::clone(&app_clone),
            )
            .await;
        })
    };

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

    let recv_task = {
        let client_id_clone = client_id.clone();
        let clients_clone = Arc::clone(&clients);
        let app_clone = Arc::clone(&app);
        let disconnect_handled_clone = Arc::clone(&disconnect_handled);

        tokio::spawn(async move {
            while let Some(result) = incoming.next().await {
                match result {
                    Ok(Message::Text(text)) => match serde_json::from_str::<MessageType>(&text) {
                        Ok(message) => {
                            handle_incoming_message(
                                message,
                                &client_id_clone,
                                &clients_clone,
                                &app_clone,
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
                        println!("Received Pong from client {}", client_id_clone);
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
    app: &Arc<Mutex<App>>,
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
            for (_, tx) in clients.lock().await.iter() {
                tx.send(broadcast_message.clone()).unwrap();
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

async fn handle_disconnection(
    disconnect_handled: Arc<Mutex<bool>>,
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<MessageType>>>>,
    app: Arc<Mutex<App>>,
) {
    let mut handled = disconnect_handled.lock().await;
    if *handled {
        return;
    }
    *handled = true;

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

    let disconnect_message =
        MessageType::SystemMessage(format!("{} has disconnected.", client_name));
    for (_, tx) in clients.lock().await.iter() {
        tx.send(disconnect_message.clone()).unwrap();
    }

    println!("{} has disconnected", client_name);
}
