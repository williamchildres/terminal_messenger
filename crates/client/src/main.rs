use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use std::io as err_io;
use tokio::io::{self};
use tokio::select;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

mod app;
mod ui;
mod websocket;
use crate::app::{App, Command, CurrentScreen, LoginField, MessageType};
use crate::event::MouseEvent;
use crate::event::MouseEventKind;
use crate::ui::ui;
use websocket::{connect_to_server, handle_websocket};
#[tokio::main]
async fn main() {
    env_logger::init();

    if let Err(e) = launch_tui().await {
        eprintln!("Error launching TUI: {:?}", e);
    }
    return;
}

async fn launch_tui() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    enable_raw_mode().map_err(Box::new)?;
    let mut stdout = err_io::stderr();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Create a channel for handling input events asynchronously
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn a task to read input events asynchronously
    tokio::spawn(async move {
        loop {
            if let Ok(event) = event::read() {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        }
    });

    // Start running the app
    match run_app(&mut terminal, &mut app, &mut rx).await {
        Ok(result) => result,
        Err(err) => {
            log::error!("Error running app: {:?}", err);
            std::process::exit(1);
        }
    };

    // Restore terminal state
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut mpsc::Receiver<Event>,
) -> io::Result<bool> {
    // Set the initial state to ServerSelection
    app.current_screen = CurrentScreen::ServerSelection;
    terminal
        .draw(|f| ui(f, app))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Define `write` and `read` as Options, initially set to `None`
    let mut write: Option<futures_util::stream::SplitSink<websocket::WsStream, Message>> = None;
    let mut read: Option<futures_util::stream::SplitStream<websocket::WsStream>> = None;

    loop {
        select! {
            // Handle WebSocket messages if connection exists
            ws_res = async {
                if let (Some(write_ref), Some(read_ref)) = (write.as_mut(), read.as_mut()) {
                    handle_websocket(app, terminal, write_ref, read_ref).await
                } else {
                    Ok(())  // Skip handling if no WebSocket connection exists
                }
            }, if write.is_some() && read.is_some() => {
                if let Err(ws_err) = ws_res {
                    log::error!("WebSocket error: {:?}", ws_err);
                    app.current_screen = CurrentScreen::Disconnected;
                    write = None;  // Set streams to None on disconnection
                    read = None;
                    terminal.draw(|f| ui(f, app))?;
                }
            }

            // Handle user input events
            Some(event) = rx.recv() => {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }

                    match app.current_screen {
                        CurrentScreen::ServerSelection => {
                            // Handle server selection input, and connect to the selected server afterward
                           if handle_server_selection_input(key.code, app, &mut write, &mut read, terminal).await? {
                                // After the user selects a server, attempt to connect

                            }
                        }
                        CurrentScreen::AddServer => {
                           handle_add_server_input(key.code, app).await?;
                        }

                        // Handle other screens only if WebSocket streams are initialized
                        CurrentScreen::LoggingIn => {

                            if let Some(ref mut write_stream) = write {
                                handle_login_input(key.code, app, write_stream).await?;
                            }
                        }
                        CurrentScreen::Main => handle_main_input(key.code, app).await,
                        CurrentScreen::ComposingMessage => {
                            if let Some(ref mut write_stream) = write {
                                handle_composing_message_input(key.code, app, write_stream).await?;
                            }
                        }
                        CurrentScreen::SetUser => {
                            if let Some(ref mut write_stream) = write {
                                handle_set_user_input(key.code, app, write_stream).await?;
                            }
                        }
                        CurrentScreen::HelpMenu => handle_help_menu_input(key.code, app).await?,
                        CurrentScreen::Exiting => {
                            if handle_exiting_input(key.code, app).await? {
                                break Ok(false);
                            }
                        }
                        CurrentScreen::ExitingLoggingIn => {
                            if handle_exiting_logging_in_input(key.code, app).await? {
                                break Ok(false);
                            }
                        }
                        CurrentScreen::Disconnected => {
                                handle_disconnected_input(key.code, app, terminal, &mut write, &mut read).await?;
                        }
                    }

                    terminal.draw(|f| ui(f, app)).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                } else if let Event::Resize(_, _) = event {
                    terminal.draw(|f| ui(f, app)).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                }
            }
        }
    }
}

async fn handle_add_server_input(key: KeyCode, app: &mut App) -> io::Result<bool> {
    match key {
        KeyCode::Enter => {
            if app.message_input.contains(':') {
                // Add a new server if the input contains "name:url"
                let parts: Vec<&str> = app.message_input.splitn(2, ':').collect();
                if let Ok(url) = Url::parse(parts[1]) {
                    app.servers.insert(parts[0].to_string(), url);
                }
                app.message_input.clear();
            }
        }
        KeyCode::Backspace => {
            app.message_input.pop(); // Handle backspace to delete characters
        }
        KeyCode::Char(c) => {
            app.message_input.push(c); // Add character to input
        }
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::ServerSelection; // Cancel add_server input and go back
        }

        _ => {}
    }

    Ok(false) // Return false if no valid server is selected
}

async fn handle_server_selection_input(
    key: KeyCode,
    app: &mut App,
    write: &mut Option<futures_util::stream::SplitSink<websocket::WsStream, Message>>,
    read: &mut Option<futures_util::stream::SplitStream<websocket::WsStream>>,
    terminal: &mut Terminal<impl Backend>,
) -> io::Result<bool> {
    match key {
        KeyCode::Enter => {
            if let Some(selected_server) = app.servers.get(app.selected_server.as_ref().unwrap()) {
                // Disconnect the current WebSocket streams
                *write = None;
                *read = None;

                // Establish a new WebSocket connection with the selected server
                let ws_stream = connect_to_server(app)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                // Split the new WebSocket stream into `write` and `read`
                let (new_write, new_read) = ws_stream.split();
                *write = Some(new_write);
                *read = Some(new_read);

                // Transition to the login screen after connection
                app.current_screen = CurrentScreen::LoggingIn;
                app.message_input.clear();

                // Reset login input fields
                app.username = None; // Clear any existing username
                app.password = None; // Clear any existing password
                app.current_login_field = LoginField::Username; // Start with the username field

                terminal
                    .draw(|f| ui(f, app))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

                return Ok(true);
            }
        }

        KeyCode::Up => {
            if let Some(selected_server) = &app.selected_server {
                let server_names: Vec<&String> = app.servers.keys().collect();

                for (i, name) in server_names.iter().enumerate() {
                    if *name == selected_server {
                        if i > 0 {
                            let new_selected_server_name =
                                server_names.get(i - 1).expect("Failed to get server name");
                            app.selected_server = Some(new_selected_server_name.to_string());
                            break;
                        }
                    }
                }
            } else {
                if let Some(first_server_name) = app.servers.keys().next() {
                    app.selected_server = Some(first_server_name.to_string());
                }
            }
        }

        KeyCode::Down => {
            if let Some(selected_server) = &app.selected_server {
                let server_names: Vec<&String> = app.servers.keys().collect();

                for (i, name) in server_names.iter().enumerate() {
                    if *name == selected_server {
                        if i < app.servers.len() - 1 {
                            let new_selected_server_name =
                                server_names.get(i + 1).expect("Failed to get server name");
                            app.selected_server = Some(new_selected_server_name.to_string());
                            break;
                        }
                    }
                }
            } else {
                if let Some(first_server_name) = app.servers.keys().next() {
                    app.selected_server = Some(first_server_name.to_string());
                }
            }
        }

        KeyCode::Char('n') => {
            app.current_screen = CurrentScreen::AddServer; // Transition to add server screen
            app.message_input.clear(); // Clear any input

            terminal
                .draw(|f| ui(f, app))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            //return Ok(true);
        }
        KeyCode::Esc => {
            if write.is_some() && read.is_some() {
                // Check if there is an active server connection
                app.current_screen = CurrentScreen::Main; // Transition to the main screen
                app.message_input.clear(); // Clear any input

                terminal
                    .draw(|f| ui(f, app))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

                return Ok(true);
            } else {
                // If no active connection exists, clear input and stay on server selection screen
                app.message_input.clear();
            }
        }
        KeyCode::Tab => {
            if let Some(selected_server) = &app.selected_server {
                let server_names: Vec<&String> = app.servers.keys().collect();
                let current_index = server_names
                    .iter()
                    .position(|name| *name == selected_server);

                if let Some(index) = current_index {
                    let next_index = (index + 1) % app.servers.len();
                    let new_selected_server_name = server_names
                        .get(next_index)
                        .expect("Failed to get server name");
                    app.selected_server = Some(new_selected_server_name.to_string());
                } else if let Some(first_server_name) = app.servers.keys().next() {
                    app.selected_server = Some(first_server_name.to_string());
                }
            } else if let Some(first_server_name) = app.servers.keys().next() {
                app.selected_server = Some(first_server_name.to_string());
            }
        }

        _ => {}
    }

    Ok(false) // Return false if no valid server is selected
}

async fn handle_login_input(
    key: KeyCode,
    app: &mut App,
    write: &mut SplitSink<websocket::WsStream, Message>,
) -> io::Result<()> {
    // Handle input based on whether the user is typing
    if app.is_typing {
        match key {
            // Submit the field after typing
            KeyCode::Enter => {
                match app.current_login_field {
                    LoginField::Username => {
                        if !app.message_input.is_empty() {
                            app.username = Some(app.message_input.clone());
                            app.message_input.clear(); // Clear for password input
                            app.current_login_field = LoginField::Password; // Move to password field
                            app.is_typing = false; // Stop typing until the user hits Enter again
                            app.messages.push(MessageType::SystemMessage(
                                "Enter your password:".to_string(),
                            ));
                        }
                    }
                    LoginField::Password => {
                        if !app.message_input.is_empty() {
                            app.password = Some(app.message_input.clone());
                            app.message_input.clear();

                            // If both fields are filled, submit the login request
                            if let (Some(username), Some(password)) = (&app.username, &app.password)
                            {
                                let auth_message = MessageType::SystemMessage(format!(
                                    "{}:{}",
                                    username, password
                                ));
                                write
                                    .send(Message::Text(
                                        serde_json::to_string(&auth_message).unwrap(),
                                    ))
                                    .await
                                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                                // Store username as staging and reset for a retry if needed
                                app.staging_username = Some(username.clone());
                            }

                            // Reset after submission
                            app.username = None;
                            app.password = None;
                            app.current_login_field = LoginField::Username; // Reset to username field
                            app.is_typing = false; // Stop typing
                        }
                    }
                }
            }

            // Handle Backspace key press only when typing
            KeyCode::Backspace => {
                if !app.message_input.is_empty() {
                    app.message_input.pop();
                }
            }

            // Handle character input while typing
            KeyCode::Char(c) => {
                app.message_input.push(c);
            }

            // Handle Esc key press to stop typing
            KeyCode::Esc => {
                // Stop typing when the user presses Esc
                app.is_typing = false;
            }

            _ => {}
        }
    } else {
        // When not typing, handle navigation and quitting
        match key {
            // Start typing when Enter is pressed
            KeyCode::Enter => {
                app.is_typing = true;
            }

            // Switch between fields using Tab
            KeyCode::Tab => {
                app.current_login_field = match app.current_login_field {
                    LoginField::Username => LoginField::Password,
                    LoginField::Password => LoginField::Username,
                };
            }

            // Quit the application with 'q' when not typing
            KeyCode::Char('q') => {
                // Set screen to Exiting
                app.current_screen = CurrentScreen::ExitingLoggingIn;
            }

            _ => {}
        }
    }

    Ok(())
}

async fn handle_main_input(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Enter => {
            app.current_screen = CurrentScreen::ComposingMessage;
            app.message_input.clear();
        }
        KeyCode::Char('h') => {
            app.current_screen = CurrentScreen::HelpMenu;
        }
        KeyCode::Char('q') => {
            app.current_screen = CurrentScreen::Exiting;
        }
        KeyCode::Char('n') => {
            app.current_screen = CurrentScreen::SetUser;
        }
        KeyCode::Char('s') => {
            // Transition to server selection screen
            app.current_screen = CurrentScreen::ServerSelection;
            app.message_input.clear();
        }
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        _ => {}
    }
}
async fn handle_composing_message_input(
    key: KeyCode,
    app: &mut App,
    write: &mut futures_util::stream::SplitSink<websocket::WsStream, Message>,
) -> io::Result<()> {
    match key {
        KeyCode::Enter => {
            let user_input = app.message_input.clone();
            match app.parse_command(&user_input) {
                Command::SetName(name) => {
                    let cmd = MessageType::Command {
                        name: "name".to_string(),
                        args: vec![name.clone()],
                    };
                    write
                        .send(Message::Text(serde_json::to_string(&cmd).unwrap()))
                        .await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                    app.set_username(name);
                }
                Command::ListUsers => {
                    let cmd = MessageType::Command {
                        name: "list".to_string(),
                        args: vec![],
                    };
                    write
                        .send(Message::Text(serde_json::to_string(&cmd).unwrap()))
                        .await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                }
                Command::DirectMessage(recipient, message) => {
                    let cmd = MessageType::Command {
                        name: "DirectMessage".to_string(),
                        args: vec![recipient.clone(), message.clone()],
                    };
                    write
                        .send(Message::Text(serde_json::to_string(&cmd).unwrap()))
                        .await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                }
                Command::Help => {
                    app.current_screen = CurrentScreen::HelpMenu;
                }
                Command::Unknown(input) => {
                    let msg = MessageType::ChatMessage {
                        sender: app.username.clone().unwrap_or_else(|| "You".to_string()),
                        content: input.clone(),
                    };
                    app.messages.push(msg.clone());
                    write
                        .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                        .await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                }
            }

            app.message_input.clear();
            app.current_screen = CurrentScreen::Main;
            return Ok(());
        }
        KeyCode::Up | KeyCode::PageUp => {
            app.compose_scroll_up();
            return Ok(());
        }
        KeyCode::Down | KeyCode::PageDown => {
            app.compose_scroll_down();
            return Ok(());
        }
        KeyCode::Backspace => {
            app.message_input.pop();
            return Ok(());
        }
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::Main;
            return Ok(());
        }
        KeyCode::Char(c) => app.message_input.push(c),
        _ => {}
    }

    Ok(())
}

async fn handle_disconnected_input(
    key: KeyCode,
    app: &mut App,
    terminal: &mut Terminal<impl Backend>,
    write: &mut Option<futures_util::stream::SplitSink<websocket::WsStream, Message>>,
    read: &mut Option<futures_util::stream::SplitStream<websocket::WsStream>>,
) -> io::Result<()> {
    match key {
        KeyCode::Char('r') => {
            // Attempt to reconnect to the selected server
            if let Ok(ws_stream) = websocket::connect_to_server(app).await {
                let (new_write, new_read) = ws_stream.split();
                *write = Some(new_write);
                *read = Some(new_read);

                // Clear the terminal and force a full redraw
                terminal.clear()?;
                app.current_screen = CurrentScreen::Main; // Back to main screen after reconnection
                terminal.draw(|f| crate::ui::ui(f, app))?;
            } else {
                // Handle reconnection failure, maybe push a system message to the app
                app.messages.push(MessageType::SystemMessage(
                    "Reconnection failed. Please check the server.".to_string(),
                ));
                terminal.draw(|f| crate::ui::ui(f, app))?;
            }
        }
        KeyCode::Char('q') => {
            // Quit the app gracefully
            std::process::exit(0);
        }
        _ => {}
    }

    Ok(())
}

async fn handle_set_user_input(
    key: KeyCode,
    app: &mut App,
    write: &mut futures_util::stream::SplitSink<websocket::WsStream, Message>,
) -> io::Result<()> {
    match key {
        KeyCode::Enter => {
            // Set the username and switch back to the main screen
            let username = app.message_input.clone();
            app.set_username(username.clone());

            let cmd = MessageType::Command {
                name: "name".to_string(),
                args: vec![username.clone()],
            };
            if let Err(e) = write
                .send(Message::Text(serde_json::to_string(&cmd).unwrap()))
                .await
            {
                log::error!("Failed to send command: {:?}", e);
            }

            app.current_screen = CurrentScreen::Main; // Go back to the main screen
            app.message_input.clear(); // Clear input after setting username
        }
        KeyCode::Backspace => {
            app.message_input.pop(); // Handle backspace to delete last character
        }
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::Main; // Cancel username input and go back
        }
        KeyCode::Char(c) => {
            app.message_input.push(c); // Add typed character to input
        }
        _ => {}
    }
    Ok(())
}

async fn handle_help_menu_input(_key: KeyCode, app: &mut App) -> io::Result<()> {
    // pressing any key will exit help menu and go back to main screen
    app.current_screen = CurrentScreen::Main;

    Ok(())
}

async fn handle_exiting_input(key: KeyCode, app: &mut App) -> io::Result<bool> {
    match key {
        KeyCode::Char('y') => {
            return Ok(true); // Exit the app
        }
        KeyCode::Char('n') | KeyCode::Char('q') => {
            app.current_screen = CurrentScreen::Main;
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_exiting_logging_in_input(key: KeyCode, app: &mut App) -> io::Result<bool> {
    match key {
        KeyCode::Char('y') => {
            return Ok(true); // Exit the app
        }
        KeyCode::Char('n') | KeyCode::Char('q') => {
            app.current_screen = CurrentScreen::LoggingIn;
        }
        _ => {}
    }
    Ok(false)
}
