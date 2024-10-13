//use std::collections::HashMap;

pub enum CurrentScreen {
    Main,
    SetUser,
    ComposingMessage,
    HelpMenu,
    Exiting,
    Disconnected,
}

pub enum Command {
    SetName(String),
    ListUsers,
    DirectMessage(String, String), // recipient, message
    Help,
    Unknown(String),
}

pub struct App {
    pub username: Option<String>,      // Keep track of username
    pub message_input: String,         // the currently being edited message value.
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    pub messages: Vec<String>,
    pub scroll_offset: usize,
}

impl App {
    pub fn new() -> App {
        App {
            username: None, // Start without a username
            message_input: String::new(),
            current_screen: CurrentScreen::Main,
            messages: Vec::<String>::new(),
            scroll_offset: 0,
        }
    }
    pub fn handle_websocket_message(&mut self, message: String) {
        self.messages.push(message);
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
