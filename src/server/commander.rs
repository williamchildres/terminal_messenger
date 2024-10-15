//  This file contains functions related to handling commands from clients. It includes a function
//  for handling commands and sending messages to clients.
//  Author: William Childres
pub mod command_handler {
    use crate::app::{App, MessageType};
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
                let list_of_clients: Vec<String> = app.lock().await.get_connected_users().await;
                let names = list_of_clients.join(", ");
                let system_message =
                    MessageType::SystemMessage(format!("Connected users: {}", names));
                clients
                    .lock()
                    .await
                    .get(client_id)
                    .unwrap()
                    .1
                    .send(system_message)
                    .unwrap();
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
