//  This file contains functions related to handling commands from clients. It includes a function
//  for handling commands and sending messages to clients.
//  Author: William Childres
pub mod command_handler {
    use crate::app::{App, MessageType, UserInfo};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    pub async fn handle_command(
        command_name: String,
        args: Vec<String>,
        client_id: &str,
        clients: &Arc<Mutex<HashMap<String, (Option<String>, mpsc::UnboundedSender<MessageType>)>>>,
        app: Arc<Mutex<App>>,
    ) {
        println!(
            "Handling command '{}' with arguments {:?}",
            command_name, args
        );

        match command_name.as_str() {
            "name" => {
                if let Some(new_name) = args.get(0) {
                    // Update client name
                    let old_tx = {
                        let clients_guard = clients.lock().await;
                        let old_data = clients_guard.get(client_id).unwrap();
                        old_data.1.clone()
                    };

                    clients
                        .lock()
                        .await
                        .insert(client_id.to_string(), (Some(new_name.clone()), old_tx));

                    app.lock()
                        .await
                        .add_connected_user(client_id.to_string(), new_name.clone())
                        .await;

                    // Notify client
                    let system_message = MessageType::SystemMessage(format!(
                        "Your name is now set to '{}'",
                        new_name
                    ));
                    clients
                        .lock()
                        .await
                        .get(client_id)
                        .unwrap()
                        .1
                        .send(system_message)
                        .unwrap();
                }
            }
            "list" => {
                let app_clone = Arc::clone(&app);
                let app_lock = app_clone.lock().await;
                let connected_users = app_lock.get_connected_users().await;

                // Create a vector to hold the usernames of connected users
                let mut names = Vec::new();

                // Iterate over each connected user and get their username
                for user in connected_users.iter() {
                    let user_lock = user.lock().await;
                    names.push(user_lock.username.clone());
                }

                // Join the usernames into a single string separated by commas
                let names_string = names.join(", ");

                // Create a system message with the list of connected users
                let system_message =
                    MessageType::SystemMessage(format!("Connected users: {}", names_string));

                // Get the client's sender channel using `get` instead of using `unwrap`
                if let Some((_name, sender)) = clients.lock().await.get(client_id) {
                    sender.send(system_message).unwrap();
                }
            }

            _ => {
                let system_message = MessageType::SystemMessage(
                    "Unknown command. Type /help for a list of commands.".to_string(),
                );
                clients
                    .lock()
                    .await
                    .get(client_id)
                    .unwrap()
                    .1
                    .send(system_message)
                    .unwrap();
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
