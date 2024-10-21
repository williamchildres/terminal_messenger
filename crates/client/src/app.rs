use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Instant;
use url::Url;

pub enum CurrentScreen {
    Main,
    SetUser,
    ComposingMessage,
    HelpMenu,
    Exiting,
    Disconnected,
    LoggingIn,
    ExitingLoggingIn,
    ServerSelection,
    AddServer,
}

pub enum Command {
    SetName(String),
    ListUsers,
    DirectMessage(String, String), // recipient, message
    Help,
    Unknown(String),
}

pub enum LoginField {
    Username,
    Password,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    ChatMessage { sender: String, content: String },
    Command { name: String, args: Vec<String> },
    SystemMessage(String),
}

pub struct App {
    pub username: Option<String>, // Keep track of username
    pub staging_username: Option<String>,
    pub password: Option<String>,      // Password field for login
    pub message_input: String,         // the currently being edited message value.
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    pub messages: Vec<MessageType>,
    pub scroll_offset: usize,
    pub compose_scroll_offset: usize,
    pub failed_login_attempts: u8,       // keep track of failed logins
    pub current_login_field: LoginField, // track current input on login
    pub is_typing: bool,                 // track if user is typing
    pub servers: HashMap<String, Url>,   // storing servers
    pub selected_server: Option<String>, // Track the selected server
    pub selected_server_index: usize,
    sound_sink: Sink,
    sound_path: PathBuf,
    last_notification_time: Option<Instant>,
}

impl App {
    pub fn new() -> App {
        let mut servers = HashMap::new();
        servers.insert(
            "local".to_string(),
            Url::parse("ws://0.0.0.0:8080").unwrap(),
        );
        servers.insert(
            "default".to_string(),
            Url::parse("ws://autorack.proxy.rlwy.net:55901").unwrap(),
        );
        let selected_server = Some("default".to_string());
        let selected_server_index = 1;
        // Initialize rodio components
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        // Assume sound file is stored in `assets/sounds/`

        let assets_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sounds/system-notification-199277.mp3");

        App {
            username: None, // Start without a username
            staging_username: None,
            password: None, // Start without a password
            message_input: String::new(),
            current_screen: CurrentScreen::Main,
            messages: Vec::<MessageType>::new(),
            scroll_offset: 0,
            compose_scroll_offset: 0,
            failed_login_attempts: 0,
            current_login_field: LoginField::Username, // Default value
            is_typing: false,
            servers,
            selected_server,
            selected_server_index,
            sound_sink: sink,
            sound_path: assets_path,
            last_notification_time: None,
        }
    }

    // Play sound asynchronously when a new message arrives
    pub fn play_notification_sound(&self) {
        let sound_path = self.sound_path.clone(); // Clone the path for the closure

        // Spawn a new blocking task to play sound
        tokio::task::spawn_blocking(move || {
            // Create a new output stream and sink for playing the sound
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();

            // Open and decode the sound file
            let file = File::open(sound_path).expect("Failed to open sound file");
            let reader = BufReader::new(file);
            let source = Decoder::new(reader).unwrap();

            // Play the sound
            sink.append(source);
            sink.play();
            sink.sleep_until_end(); // Wait until the sound finishes playing
                                    //println!("Notification sound played.");
        });
    }

    // Handling incoming WebSocket messages from the server
    pub fn handle_websocket_message(&mut self, message: &str) {
        if let Ok(message_type) = serde_json::from_str::<MessageType>(&message) {
            match message_type {
                MessageType::ChatMessage { sender, content } => {
                    // Push the chat message into `self.messages`
                    self.messages
                        .push(MessageType::ChatMessage { sender, content });
                    // Only play sound if there hasn't been a notification within the last 1 seconds
                    if self
                        .last_notification_time
                        .map(|t| t.elapsed().as_secs() > 1)
                        .unwrap_or(true)
                    {
                        self.play_notification_sound(); // Play sound on new chat message
                        self.last_notification_time = Some(Instant::now()); // Update time of last notification
                    }
                }
                MessageType::SystemMessage(system_message) => {
                    if system_message.contains("Authentication successful") {
                        // Push authentication success message
                        self.messages.push(MessageType::SystemMessage(
                            "You are authenticated!".to_string(),
                        ));
                        self.current_screen = CurrentScreen::Main;
                        self.failed_login_attempts = 0; // Reset failed attempts on success
                        self.username = self.staging_username.clone();
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
