use std::collections::HashMap;

pub enum CurrentScreen {
    Main,
    ComposingMessage,
    Exiting,
}

pub struct App {
    pub message_input: String, // the currently being edited message value.
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    pub messages: Vec<String>,
}

impl App {
    pub fn new() -> App {
        App {
            message_input: String::new(),

            current_screen: CurrentScreen::Main,

            messages: Vec::<String>::new(),
        }
    }

    pub fn handle_websocket_message(&mut self, message: String) {
        self.messages.push(message);
    }
}
