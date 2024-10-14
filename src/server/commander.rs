pub mod command_handler {
    use crate::app::App;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    pub async fn handle_command(
        command: &str,
        client_id: &str,
        clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<String>)>>>,
        app: Arc<Mutex<App>>,
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
            /dm <name> <message> - Send a private message to the user with the specified name
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

                    // Update the App state with new user info
                    app.lock()
                        .await
                        .add_connected_user(client_id.to_string(), name.clone())
                        .await;

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
                let list_of_clients: Vec<String> = app.lock().await.get_connected_users().await;

                /* Convert the list to a single string */
                let names: String = list_of_clients.join(", ");

                /* Send the message to the client */
                send_to_client(client_id, clients, format!("Connected users: {}", names)).await;
            }

            Some(cmd) if cmd.starts_with("dm") => {
                let args: Vec<&str> = cmd[2..].trim().splitn(2, ' ').collect();
                if args.len() != 2 {
                    send_to_client(
                        client_id,
                        clients,
                        "Usage: /whisper <name> <message>".to_string(),
                    )
                    .await;
                } else {
                    let recipient_name = args[0];
                    let message = args[1];

                    let recipient = {
                        let clients_guard = clients.lock().await;
                        clients_guard
                            .iter()
                            .find(|(_, (name, _))| name.as_deref() == Some(recipient_name))
                            .map(|(_, (_, tx))| tx.clone())
                    };

                    match recipient {
                        Some(tx) => {
                            tx.send(format!(
                                "(Private message from {}): {}",
                                recipient_name, message
                            ))
                            .unwrap();
                            send_to_client(
                                client_id,
                                clients,
                                format!("(Private message to {}): {}", recipient_name, message),
                            )
                            .await;
                        }
                        None => {
                            send_to_client(
                                client_id,
                                clients,
                                format!("User '{}' not found.", recipient_name),
                            )
                            .await;
                        }
                    }
                }
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

    pub async fn send_to_client(
        client_id: &str,
        clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<String>)>>>,
        message: String,
    ) {
        if let Some((_, tx)) = clients.lock().await.get_mut(&client_id.to_string()) {
            tx.send(message).unwrap();
        }
    }
}
