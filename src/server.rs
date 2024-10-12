use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};

#[tokio::main]
async fn main() {
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

                tokio::spawn(async move {
                    while let Some(Ok(Message::Text(text))) = incoming.next().await {
                        if text.starts_with("/") {
                            handle_command(&text, &client_id, &clients).await;
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
                            if history.lock().await.len() == 100 {
                                history.lock().await.pop_front();
                            }
                            history.lock().await.push_back(broadcast_message.clone());

                            for (_, (_, tx)) in clients.lock().await.iter() {
                                tx.send(broadcast_message.clone()).unwrap();
                            }
                        }
                    }

                    // Cleanup: Remove the client after disconnect
                    clients.lock().await.remove(&client_id);
                })
            };

            /* Wait for either task to complete. If one completes, cancel the other. */
            tokio::select! {
                _= send_task => {},
                _= recv_task => {},
            };
        });
    }
}

async fn handle_command(
    command: &str,
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<String>)>>>,
) {
    match command.strip_prefix("/") {
        Some("help") => {
            send_to_client(
                client_id,
                clients,
                r#"
            /help - Show available commands
            /name <your_name> - Set your nickname
            /list - List all connected users
            "#
                .to_string(),
            )
            .await;
        }

        Some(cmd) if cmd.starts_with("name") => {
            let name = cmd[4..].trim().to_string();
            if name.is_empty() {
                send_to_client(
                    client_id,
                    clients,
                    "Please provide a valid name.".to_string(),
                )
                .await;
            } else {
                let old_tx = {
                    // Clone channel in separate scope so we only hold lock temporarily
                    let clients_guard = clients.lock().await;
                    let old_data = clients_guard.get(client_id).unwrap();
                    old_data.1.clone()
                };

                // Now we can insert without deadlocking
                clients
                    .lock()
                    .await
                    .insert(client_id.to_string(), (Some(name.clone()), old_tx)); // Update the client name

                send_to_client(
                    client_id,
                    &clients,
                    format!("Your name is now set to '{}'", name),
                )
                .await;
            }
        }

        Some("list") => {
            /* Get the list of all connected users */
            let list_of_clients: Vec<String> = (*clients)
                .lock()
                .await
                .values()
                .filter_map(|(name_opt, _)| name_opt.as_ref())
                .map(String::clone)
                .collect();

            /* Convert the list to a single string */
            let names: String = list_of_clients.join(", ");

            /* Send the message to the client */
            send_to_client(client_id, clients, format!("Connected users: {}", names)).await;
        }

        _ => {
            send_to_client(
                client_id,
                clients,
                "Unknown command. Type /help for a list of commands.".to_string(),
            )
            .await;
        }
    }
}

async fn send_to_client(
    client_id: &str,
    clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<String>)>>>,
    message: String,
) {
    if let Some((_, tx)) = clients.lock().await.get_mut(&client_id.to_string()) {
        tx.send(message).unwrap();
    }
}
