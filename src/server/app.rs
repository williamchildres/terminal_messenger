//  This file contains the definition of the `App` struct, which represents the server state.
//  It also defines the `UserInfo` struct and an enumeration of message types.
use std::time::SystemTime;

use crate::HashMap;
use serde::{Deserialize, Serialize};

pub struct App {
    connected_users: HashMap<String, UserInfo>,
}

pub struct UserInfo {
    pub username: String,
    pub connection_time: std::time::SystemTime,
    pub message_count: usize,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    ChatMessage { sender: String, content: String },
    Command { name: String, args: Vec<String> },
    SystemMessage(String),
}

impl App {
    pub fn new() -> App {
        // Initialize and return a new instance of `App`
        App {
            connected_users: HashMap::new(),
        }
    }

    pub async fn add_connected_user(&mut self, user_id: String, username: String) {
        let user_info = UserInfo {
            username,
            connection_time: SystemTime::now(),
            message_count: 0,
            color: "default".to_string(),
        };
        self.connected_users.insert(user_id.clone(), user_info);
    }

    pub async fn get_connected_user(&self, user_id: &str) -> Option<&UserInfo> {
        self.connected_users.get(user_id)
    }

    pub async fn remove_connected_user(&mut self, user_id: &str) -> Option<UserInfo> {
        self.connected_users.remove(user_id)
    }

    pub async fn get_connected_users(&self) -> Vec<&UserInfo> {
        self.connected_users.values().collect()
    }

    pub async fn update_username(&mut self, user_id: String, username: String) {
        if let Some(user_info) = self.connected_users.get_mut(&user_id) {
            user_info.username = username;
        }
    }
}

impl UserInfo {
    pub fn new() -> UserInfo {
        // Initalize and return a new isntance of 'UserInfo'
        UserInfo {
            username: "username".to_string(),
            connection_time: SystemTime::now(),
            message_count: 0,
            color: "default".to_string(),
        }
    }
}
