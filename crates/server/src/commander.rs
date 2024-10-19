//  This file contains functions related to handling commands from clients. It includes a function
//  for handling commands and sending messages to clients.
pub mod command_handler {
    use crate::app::{App, MessageType};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    pub async fn handle_command(
        command_name: String,
        args: Vec<String>,
        client_id: &str,
        clients: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<MessageType>>>>,
        app: Arc<Mutex<App>>,
    ) {
        println!(
            "Handling command '{}' with arguments {:?}",
            command_name, args
        );

        match command_name.as_str() {
            "name" => {
                if let Some(new_name) = args.get(0) {
                    // Update client name in the App (UserInfo)
                    app.lock()
                        .await
                        .update_username(client_id.to_string(), new_name.clone())
                        .await;

                    // Notify client of the name change
                    let system_message = MessageType::SystemMessage(format!(
                        "Your name is now set to '{}'",
                        new_name
                    ));
                    clients
                        .lock()
                        .await
                        .get(client_id)
                        .unwrap()
                        .send(system_message)
                        .unwrap();
                }
            }
            "list" => {
                let app_clone = Arc::clone(&app);
                let app_lock = app_clone.lock().await;
                let connected_users = app_lock.get_connected_users().await;

                // Collect usernames from App's connected_users
                let mut names = Vec::new();
                for user in connected_users.iter() {
                    let user_lock = user.lock().await;
                    names.push(user_lock.username.clone());
                }

                let names_string = names.join(", ");
                let system_message =
                    MessageType::SystemMessage(format!("Connected users: {}", names_string));

                if let Some(sender) = clients.lock().await.get(client_id) {
                    sender.send(system_message).unwrap();
                }
            }
            _ => {
                let system_message = MessageType::SystemMessage(
                    "Unknown command. Type /help for a list of commands.".to_string(),
                );
                if let Some(sender) = clients.lock().await.get(client_id) {
                    sender.send(system_message).unwrap();
                }
            }
        }
    }
}
