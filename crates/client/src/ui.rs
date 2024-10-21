use crate::app::{App, CurrentScreen};
use ratatui::Frame;

mod add_server;
mod chat;
mod disconnected;
mod exiting;
mod help;
mod login;
mod server_selection;
mod set_user;
mod utils;

pub fn ui(frame: &mut Frame, app: &mut App) {
    match app.current_screen {
        CurrentScreen::LoggingIn => login::render_login(frame, app),
        CurrentScreen::Main | CurrentScreen::ComposingMessage => chat::render_chat(frame, app),
        CurrentScreen::HelpMenu => help::render_help(frame),
        CurrentScreen::Exiting | CurrentScreen::ExitingLoggingIn => exiting::render_exiting(frame),
        CurrentScreen::Disconnected => disconnected::render_disconnected(frame),
        CurrentScreen::SetUser => set_user::render_set_user(frame, app),
        CurrentScreen::ServerSelection => server_selection::render_server_selection(frame, app), // Route for the server selection screen
        CurrentScreen::AddServer => add_server::render_add_server(frame, app), // _ => {} // Handle other screens if needed
    }
}
