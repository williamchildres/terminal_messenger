use futures_util::{SinkExt, StreamExt};
use libc;
use signal_hook::iterator::SignalsInfo;

use ratatui::{
    backend::{self, Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use std::io as err_io;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use url::Url;

mod app;
mod ui;
use crate::app::{App, CurrentScreen};
use crate::ui::ui;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "/tui" {
        if let Err(e) = launch_tui().await {
            eprintln!("Error launching TUI: {:?}", e);
        }
        return;
    }

    // Specify the server URL to connect to
    let server_url = Url::parse("ws://127.0.0.1:8080").unwrap();

    // Establish a WebSocket connection with the server
    let (ws_stream, _) = connect_async(server_url).await.expect("Failed to connect");

    println!("Connected to the server");

    // Split the WebSocket stream into a writer and reader part
    let (mut write, mut read) = ws_stream.split();

    // Prepare for reading input from terminal (std input)
    let stdin = BufReader::new(io::stdin());

    // Spawn a task for reading WebSocket messages
    let read_ws = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            println!("Received: {}", text);
        }
    });

    // Spawn another task for reading lines from terminal and sending them over WebSocket
    let write_ws = tokio::spawn(async move {
        let mut lines = stdin.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Err(e) = write.send(Message::Text(line)).await {
                eprintln!("Failed to send message: {:?}", e);
                break;
            }
        }
    });

    // Wait until either read or write task is done
    tokio::select! {
        _ = read_ws => (),
        _ = write_ws => (),
    }
}

async fn launch_tui() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
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
            eprintln!("Error running app: {:?}", err);
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
    // Specify the server URL to connect to
    let server_url = Url::parse("ws://127.0.0.1:8080").unwrap();

    // Establish a WebSocket connection with the server
    let (ws_stream, _) = connect_async(server_url).await.expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    loop {
        // Use tokio::select! to handle both WebSocket messages and terminal events
        tokio::select! {
            // Handle WebSocket messages
            ws_msg = read.next() => {
                if let Some(Ok(Message::Text(text))) = ws_msg {
                    // Update app state with the received WebSocket message
                    app.handle_websocket_message(text);
                    // Redraw the UI after receiving the message
                    terminal.draw(|f| ui(f, app))?;
                } else if let Some(Err(e)) = ws_msg {
                    eprintln!("WebSocket error: {:?}", e);
                    return Ok(false); // Exit on WebSocket error
                }
            }



            // Handle user input events
            Some(event) = rx.recv() => {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    match app.current_screen {
                        CurrentScreen::Main => match key.code {
                            KeyCode::Enter =>{
                                app.current_screen = CurrentScreen::ComposingMessage;
                                app.message_input.clear();
                            }
                            KeyCode::Char('h') => {
                                app.current_screen = CurrentScreen::HelpMenu;
                            }
                            KeyCode::Char('q') => {
                                app.current_screen = CurrentScreen::Exiting;
                            }
                            _ => {}
                        },
                        CurrentScreen::Exiting => match key.code {
                            KeyCode::Char('y') => {
                                return Ok(true);
                            }
                            KeyCode::Char('n') | KeyCode::Char('q') => {
                                //return Ok(false);
                                app.current_screen = CurrentScreen::Main;
                            }
                            _ => {}
                        },
                         CurrentScreen::ComposingMessage => match key.code {
                            KeyCode::Enter => {
                                // Send the message to the WebSocket server
                                if let Err(e) = write.send(Message::Text(app.message_input.clone())).await {
                                    eprintln!("Failed to send message: {:?}", e);
                                }
                                app.message_input.clear();  // Clear the input buffer after sending
                                app.current_screen = CurrentScreen::Main;  // Go back to the main screen
                            }
                            KeyCode::Backspace => {
                                // Remove the last character from the message input
                                app.message_input.pop();
                            }
                            KeyCode::Esc => {
                                // Cancel message composing and go back to the main screen
                                app.current_screen = CurrentScreen::Main;
                            }
                            KeyCode::Char(c) => {
                                // Add the character to the message input
                                app.message_input.push(c);
                            }
                            _ => {}
                        },
                        CurrentScreen::HelpMenu => match key.code {  // pressing any key will exit help menu
                         _ => {
                             app.current_screen = CurrentScreen::Main;
                         }
                     },
                      //  _ => {}

                    }

                    // Redraw the UI after handling the user input event
                    terminal.draw(|f| ui(f, app))?;
                } else if let Event::Resize(_, _) = event {
                    terminal.draw(|f| ui(f, app))?;
                }
            }
        }
    }
}
