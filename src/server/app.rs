use crate::HashMap;
use serde::{Deserialize, Serialize};

pub struct App {
    connected_users: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    ChatMessage { sender: String, content: String },
    Command { name: String, args: Vec<String> },
    SystemMessage(String),
}

pub struct UserInfo {
    pub username: String,
    pub connection_time: std::time::SystemTime,
    pub message_count: usize,
}

impl App {
    pub fn new() -> App {
        // Initialize and return a new instance of `App`
        App {
            connected_users: HashMap::new(),
        }
    }

    pub async fn add_connected_user(&mut self, user_id: String, username: String) {
        self.connected_users.insert(user_id, username);
    }

    pub async fn get_connected_user(&self, user_id: &str) -> Option<&String> {
        self.connected_users.get(user_id)
    }

    pub async fn remove_connected_user(&mut self, user_id: &str) -> Option<String> {
        self.connected_users.remove(user_id)
    }

    pub async fn get_connected_users(&self) -> Vec<String> {
        self.connected_users.values().cloned().collect()
    }
}
