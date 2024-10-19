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
    pub messages: Vec<MessageType>,
    pub scroll_offset: usize,
    pub compose_scroll_offset: usize,
    pub failed_login_attempts: u8, // keep track of failed logins
}

impl App {
    pub fn new() -> App {
        App {
            username: None, // Start without a username
            password: None, // Start without a password
            message_input: String::new(),
            current_screen: CurrentScreen::Main,
            messages: Vec::<MessageType>::new(),
            scroll_offset: 0,
            compose_scroll_offset: 0,
            failed_login_attempts: 0,
        }
    }

    // Handling incoming WebSocket messages from the server
    pub fn handle_websocket_message(&mut self, message: &str) {
        if let Ok(message_type) = serde_json::from_str::<MessageType>(&message) {
            match message_type {
                MessageType::ChatMessage { sender, content } => {
                    // Push the chat message into `self.messages`
                    self.messages
                        .push(MessageType::ChatMessage { sender, content });
                }
                MessageType::SystemMessage(system_message) => {
                    if system_message.contains("Authentication successful") {
                        // Push authentication success message
                        self.messages.push(MessageType::SystemMessage(
                            "You are authenticated!".to_string(),
                        ));
                        self.current_screen = CurrentScreen::Main;
                        self.failed_login_attempts = 0; // Reset failed attempts on success
                    } else if system_message.contains("Authentication failed") {
                        self.failed_login_attempts += 1; // Increment failed attempts
                        let remaining_attempts = 5 - self.failed_login_attempts;
                        // Push authentication failure message
                        self.messages.push(MessageType::SystemMessage(format!(
                            "Authentication failed. {} attempts remaining.",
                            remaining_attempts
                        )));
                        if self.failed_login_attempts >= 5 {
                            self.current_screen = CurrentScreen::Disconnected; // Disconnect after max attempts
                            self.messages.push(MessageType::SystemMessage(
                                "Max login attempts reached. Connection closed.".to_string(),
                            ));
                        } else {
                            self.current_screen = CurrentScreen::LoggingIn; // Retry login
                        }
                    } else {
                        // Push any other system message received
                        self.messages
                            .push(MessageType::SystemMessage(system_message));
                    }
                }
                _ => {}
            }
        } else {
            // If parsing fails, treat it as a plain message and push it as is
            self.messages
                .push(MessageType::SystemMessage(message.to_string()));
        }

        self.scroll_offset = 0;
    }

    // Methods for scrolling up and down in main chat
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    // Methods for scrolling up and down in compose area
    pub fn compose_scroll_up(&mut self) {
        self.compose_scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn compose_scroll_down(&mut self) {
        self.compose_scroll_offset = self.scroll_offset.saturating_sub(1);
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
