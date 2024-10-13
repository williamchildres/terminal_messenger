//use std::collections::HashMap;

pub enum CurrentScreen {
    Main,
    ComposingMessage,
    HelpMenu,
    Exiting,
}

pub struct App {
    pub message_input: String, // the currently being edited message value.
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    pub messages: Vec<String>,
    pub messages_offset: usize,
}

impl App {
    pub fn new() -> App {
        App {
            message_input: String::new(),
            current_screen: CurrentScreen::Main,
            messages: Vec::<String>::new(),
            messages_offset: 0,
        }
    }
    pub fn handle_websocket_message(&mut self, message: String) {
        self.messages.push(message);
        self.messages_offset = self.messages.len().saturating_sub(10); // change 10 to whatever fits your screen height
    }
}
