//use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub enum CurrentScreen {
    Main,
    SetUser,
    ComposingMessage,
    HelpMenu,
    Exiting,
    Disconnected,
    LoggingIn,
}

pub enum Command {
    SetName(String),
    ListUsers,
    DirectMessage(String, String), // recipient, message
    Help,
    Unknown(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    ChatMessage { sender: String, content: String },
    Command { name: String, args: Vec<String> },
    SystemMessage(String),
}

pub struct App {
    pub username: Option<String>,      // Keep track of username
    pub password: Option<String>,      // Password field for login
    pub message_input: String,         // the currently being edited message value.
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    pub messages: Vec<String>,
    pub scroll_offset: usize,
    pub failed_login_attempts: u8, // keep track of failed logins
}

impl App {
    pub fn new() -> App {
        App {
            username: None, // Start without a username
            password: None, // Start without a password
            message_input: String::new(),
            current_screen: CurrentScreen::Main,
            messages: Vec::<String>::new(),
            scroll_offset: 0,
            failed_login_attempts: 0,
        }
    }

    // Handling incoming WebSocket messages from the server
    pub fn handle_websocket_message(&mut self, message: &str) {
        if let Ok(message_type) = serde_json::from_str::<MessageType>(&message) {
            match message_type {
                MessageType::ChatMessage { sender, content } => {
                    let formatted_message = format!("{}: {}", sender, content);
                    self.messages.push(formatted_message);
                }
                MessageType::SystemMessage(system_message) => {
                    if system_message.contains("Authentication successful") {
                        self.messages.push("You are authenticated!".to_string());
                        self.current_screen = CurrentScreen::Main;
                        self.failed_login_attempts = 0; // Reset failed attempts on success
                    } else if system_message.contains("Authentication failed") {
                        self.failed_login_attempts += 1; // Increment failed attempts
                        let remaining_attempts = 5 - self.failed_login_attempts;
                        self.messages.push(format!(
                            "Authentication failed. {} attempts remaining.",
                            remaining_attempts
                        ));

                        if self.failed_login_attempts >= 5 {
                            self.current_screen = CurrentScreen::Disconnected; // Disconnect after max attempts
                            self.messages
                                .push("Max login attempts reached. Connection closed.".to_string());
                        } else {
                            self.current_screen = CurrentScreen::LoggingIn; // Retry login
                        }
                    } else {
                        self.messages.push(system_message);
                    }
                }
                _ => {}
            }
        } else {
            self.messages.push(message.to_string());
        }

        self.scroll_offset = 0;
    }
    // Methods for scrolling up and down
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    // Method for setting username
    pub fn set_username(&mut self, name: String) {
        self.username = Some(name);
    }
    pub fn parse_command(&self, input: &str) -> Command {
        let input = input.trim();

        if input.starts_with("/") {
            let parts: Vec<&str> = input.splitn(3, ' ').collect();
            match parts.as_slice() {
                ["/name", name] if !name.is_empty() => Command::SetName(name.to_string()),
                ["/list"] => Command::ListUsers,
                ["/dm", recipient, message] if !message.is_empty() => {
                    Command::DirectMessage(recipient.to_string(), message.to_string())
                }
                ["/help"] => Command::Help,
                _ => Command::Unknown(input.to_string()),
            }
        } else {
            Command::Unknown(input.to_string()) // Treat as unknown if it's not a valid command
        }
    }
}
