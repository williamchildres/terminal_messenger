use crate::app::App;
use futures_util::{SinkExt, StreamExt};
use ratatui::backend::Backend;
use ratatui::Terminal;
use tokio::io;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn connect_to_server(
    app: &App,
) -> Result<WsStream, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(server_name) = &app.selected_server {
        if let Some(server_url) = app.servers.get(server_name) {
            let url_string = server_url.to_string();
            let (ws_stream, _) = connect_async(&url_string).await?;
            return Ok(ws_stream);
        }
    }
    Err(Box::new(io::Error::new(
        io::ErrorKind::NotFound,
        "No server selected",
    )))
}

pub async fn handle_websocket<B: Backend>(
    app: &mut App,
    terminal: &mut Terminal<B>,
    write: &mut futures_util::stream::SplitSink<WsStream, Message>,
    read: &mut futures_util::stream::SplitStream<WsStream>,
) -> io::Result<()> {
    loop {
        tokio::select! {
            ws_msg = read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        app.handle_websocket_message(&text);
                        terminal.draw(|f| crate::ui::ui(f, app))
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    }
                    Some(Ok(Message::Binary(_))) => {
                        // Handle binary message if needed
                    }
                    Some(Ok(Message::Ping(ping))) => {
                        // Respond to ping by sending a Pong message
                      write.send(Message::Pong(ping)).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Handle pong if necessary
                    }
                    Some(Ok(Message::Close(_))) => {
                        app.current_screen = crate::app::CurrentScreen::Disconnected;
                        terminal.draw(|f| crate::ui::ui(f, app))
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        break;
                    }
                    Some(Err(e)) => {
                        // Log the WebSocket error and move to the Disconnected state
                        app.current_screen = crate::app::CurrentScreen::Disconnected;
                        terminal.draw(|f| crate::ui::ui(f, app))
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        log::error!("WebSocket error: {:?}", e);
                        break;
                    }
                    None => {
                        // Handle the case when the stream ends
                        app.current_screen = crate::app::CurrentScreen::Disconnected;
                        terminal.draw(|f| crate::ui::ui(f, app))
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        break;
                    }
                    Some(Ok(Message::Frame(frame_data))) => {
                        let _ = frame_data;
                    }
                }
            }
        }
    }

    Ok(())
}
