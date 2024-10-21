//  This file contains the definition of the `App` struct, which represents the server state.
//  It also defines the `UserInfo` struct and an enumeration of message types.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

// App struct to store connected users and message history
pub struct App {
    // Store users with their UUID as key
    connected_users: HashMap<String, Arc<Mutex<UserInfo>>>,
    // Global message history (last 100 messages)
    message_history: VecDeque<MessageType>,
    user_credentials: HashMap<String, UserCredentials>, // Add this for storing credentials
}

pub struct UserInfo {
    pub username: String,
    pub connection_time: SystemTime,
    pub message_count: usize,
}

pub struct UserCredentials {
    pub username: String,
    pub password: String, // Ideally store hashed passwords
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    ChatMessage { sender: String, content: String },
    Command { name: String, args: Vec<String> },
    SystemMessage(String),
}

impl App {
    pub fn new() -> App {
        let mut user_credentials = HashMap::new();

        // For simplicity, let's add a couple of users (these should be hashed passwords)
        user_credentials.insert(
            "user1".to_string(),
            UserCredentials {
                username: "user1".to_string(),
                password: "password1".to_string(),
            },
        );
        user_credentials.insert(
            "user2".to_string(),
            UserCredentials {
                username: "user2".to_string(),
                password: "password2".to_string(),
            },
        );

        App {
            connected_users: HashMap::new(),
            message_history: VecDeque::with_capacity(100), // Store up to 100 messages
            user_credentials,                              // finitialize the credentials
        }
    }

    // Method to verify username and password
    pub fn authenticate_user(&self, username: &str, password: &str) -> bool {
        if let Some(credentials) = self.user_credentials.get(username) {
            return credentials.password == password; // Verify credentials (ideally hash comparison)
        }
        false
    }

    // Add a connected user by UUID
    pub async fn add_connected_user(&mut self, user_id: String, username: String) {
        let user_info = Arc::new(Mutex::new(UserInfo {
            username,
            connection_time: SystemTime::now(),
            message_count: 0,
            color: "default".to_string(),
        }));
        self.connected_users.insert(user_id.clone(), user_info);
    }

    // Retrieve a connected user by UUID
    pub async fn get_connected_user(&self, user_id: &str) -> Option<Arc<Mutex<UserInfo>>> {
        self.connected_users.get(user_id).cloned()
    }

    // Remove a connected user by UUID
    pub async fn remove_connected_user(&mut self, user_id: &str) -> Option<Arc<Mutex<UserInfo>>> {
        self.connected_users.remove(user_id)
    }

    pub async fn get_connected_users(&self) -> Vec<Arc<Mutex<UserInfo>>> {
        self.connected_users.values().cloned().collect()
    }

    // Update username for a user
    pub async fn update_username(&mut self, user_id: String, username: String) {
        if let Some(user_info) = self.connected_users.get_mut(&user_id) {
            user_info.lock().await.username = username;
        }
    }

    // Add a message to the message history (limit to 100 messages)
    pub async fn add_message_to_history(&mut self, message: MessageType) {
        if self.message_history.len() == 100 {
            self.message_history.pop_front(); // Remove oldest message if full
        }
        self.message_history.push_back(message);
    }

    // Retrieve the message history
    pub async fn get_message_history(&self) -> Vec<MessageType> {
        self.message_history.iter().cloned().collect()
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
