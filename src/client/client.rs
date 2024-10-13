use futures::stream::Chunks;
// Import necessary packages from tokio, tokio_tungstenite, url and futures_util
use futures_util::{SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use url::Url;

use std::{error::Error, io as err_io};

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

mod app;
mod ui;
use crate::{
    app::{App, CurrentScreen, CurrentlyEditing},
    ui::ui,
};

// Define the asyncchronous main function
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "/tui" {
        launch_tui().await;
        return;
    }
    // Specify the server URL to connect to
    let server_url = Url::parse("ws://127.0.0.1:8080").unwrap();

    // Establish a WebSocket connection with the server
    let (ws_stream, _) = connect_async(server_url).await.expect("Failed to connect");

    // Print out a message indicating that the client has connected to the server
    println!("Connected to the server");

    // Split the WebSocket stream into a writer and reader part
    let (mut write, mut read) = ws_stream.split();

    // Prepare for reading input from terminal (std input)
    let stdin = BufReader::new(io::stdin());

    // Spawn a task for reading messages from the WebSocket and print them in the terminal
    let read_ws = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            println!("Received: {}", text);
        }
    });

    // Spawn another task for reading lines from terminal sending them as text messages over
    // WebSocket to server
    let write_ws = tokio::spawn(async move {
        let mut lines = stdin.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Err(e) = write.send(Message::Text(line)).await {
                eprintln!("Failed to send message: {:?}", e);
                break;
            }
        }
    });

    // Wait until either read or write task is done. This could be because an error occurred, or
    // because the WebSocket was closed by the other end
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

    // create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Ok(do_print) = res {
        if do_print {
            app.print_json()?;
        }
    } else if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEventKind::Press
                continue;
            }
            match app.current_screen {
                CurrentScreen::Main => match key.code {
                    KeyCode::Char('e') => {
                        app.current_screen = CurrentScreen::Editing;
                        app.currently_editing = Some(CurrentlyEditing::Key);
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
                        return Ok(false);
                    }
                    _ => {}
                },
                CurrentScreen::Editing if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Enter => {
                        if let Some(editing) = &app.currently_editing {
                            match editing {
                                CurrentlyEditing::Key => {
                                    app.currently_editing = Some(CurrentlyEditing::Value);
                                }
                                CurrentlyEditing::Value => {
                                    app.save_key_value();
                                    app.current_screen = CurrentScreen::Main;
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(editing) = &app.currently_editing {
                            match editing {
                                CurrentlyEditing::Key => {
                                    app.key_input.pop();
                                }
                                CurrentlyEditing::Value => {
                                    app.value_input.pop();
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.current_screen = CurrentScreen::Main;
                        app.currently_editing = None;
                    }
                    KeyCode::Tab => {
                        app.toggle_editing();
                    }
                    KeyCode::Char(value) => {
                        if let Some(editing) = &app.currently_editing {
                            match editing {
                                CurrentlyEditing::Key => {
                                    app.key_input.push(value);
                                }
                                CurrentlyEditing::Value => {
                                    app.value_input.push(value);
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
