use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};

use crate::app::App;
use crate::commander::command_handler::handle_command;

pub async fn websocket_task(app: Arc<Mutex<App>>) {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("Server listening on {}", addr);

    let clients = Arc::new(Mutex::new(HashMap::<
        String,
        (Option<String>, mpsc::UnboundedSender<String>),
    >::new()));
    let history = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(100)));

    while let Ok((stream, _)) = listener.accept().await {
        let clients = clients.clone();
        let history = history.clone();
        let app = app.clone();

        tokio::spawn(async move {
            let ws_stream: WebSocketStream<_> =
                accept_async(stream).await.expect("Error during handshake");
            // Get client IP
            let client_id = format!("{}", ws_stream.get_ref().peer_addr().unwrap());

            // Each client gets its own tx/rx
            let (tx_original, mut rx) = mpsc::unbounded_channel();

            // Insert client with no name initially
            {
                let mut clients_guard = clients.lock().await;
                clients_guard.insert(client_id.clone(), (None, tx_original.clone()));
            }

            // Send the message history to the new client
            for message in history.lock().await.iter() {
                tx_original.send(message.clone()).unwrap();
            }

            /* Split the socket into a sender and receiver */
            let (mut outgoing, mut incoming) = ws_stream.split();

            // Task for sending messages to this client
            let send_task = tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    if outgoing.send(Message::Text(message)).await.is_err() {
                        break;
                    }
                }
            });

            // Task for receiving messages from this client
            let recv_task = {
                let clients = clients.clone();
                let history = history.clone();
                let client_id = client_id.clone();
                let app = app.clone();

                tokio::spawn(async move {
                    while let Some(Ok(Message::Text(text))) = incoming.next().await {
                        if text.starts_with("/") {
                            handle_command(&text, &client_id, &clients, app.clone()).await;
                        } else {
                            // Broadcast to all clients
                            let client_name =
                                clients.lock().await.get(&client_id).unwrap().0.clone();
                            let broadcast_message: String;
                            broadcast_message = match client_name {
                                Some(name) => format!("{}: {}", name, text),
                                None => format!("{}: {}", client_id.clone(), text),
                            };
                            // Add to history
                            let mut history_guard = history.lock().await;
                            if history_guard.len() == 100 {
                                history_guard.pop_front();
                            }
                            history_guard.push_back(broadcast_message.clone());

                            for (_, (_, tx)) in clients.lock().await.iter() {
                                tx.send(broadcast_message.clone()).unwrap();
                            }
                        }
                    }

                    handle_disconnect(&client_id, &clients, app.clone()).await;
                })
            };

            /* Wait for either task to complete. If one completes, cancel the other. */
            tokio::select! {
                _= send_task => {},
                _= recv_task => {},
            }

            // Use "app" after tasks have completed
            let app_guard = app.lock().await;

            // Now call methods on App struct as needed
            match app_guard.get_connected_user(&client_id).await {
                Some(user) => println!("User {} found", user),
                None => println!("User not found"),
            }
        });
    }
}

async fn handle_disconnect(
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<String>)>>>,
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

    let disconnect_message = format!("{} has disconnected.", client_name);

    // Broadcast the disconnection message to all remaining clients
    for (_, (_, tx)) in clients.lock().await.iter() {
        tx.send(disconnect_message.clone()).unwrap();
    }

    println!("{} has disconnected", client_name);
}
