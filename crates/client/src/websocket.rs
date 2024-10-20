use crate::app::App;
use futures_util::StreamExt;
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

// Handle WebSocket messages in a separate module
pub async fn handle_websocket<B: Backend>(
    app: &mut App,
    terminal: &mut Terminal<B>,
    _write: &mut futures_util::stream::SplitSink<
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        Message,
    >,
    read: &mut futures_util::stream::SplitStream<
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    >,
) -> io::Result<()> {
    loop {
        tokio::select! {
            ws_msg = read.next() => {
                if let Some(Ok(Message::Text(text))) = ws_msg {
                    app.handle_websocket_message(&text);

                    // Redraw the terminal to reflect the new messages
                    terminal.draw(|f| crate::ui::ui(f, app))
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                } else if let Some(Ok(Message::Close(_))) = ws_msg {
                    app.current_screen = crate::app::CurrentScreen::Disconnected;
                    terminal.draw(|f| crate::ui::ui(f, app))
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    break;
                } else if let Some(Err(e)) = ws_msg {
                    app.current_screen = crate::app::CurrentScreen::Disconnected;
                    terminal.draw(|f| crate::ui::ui(f, app))
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    log::error!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
