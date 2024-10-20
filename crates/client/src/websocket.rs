use crate::app::App;
use futures_util::StreamExt;
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::error::Error;
use tokio::io;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

// Connect to the server and return the WebSocket stream
pub async fn connect_to_server() -> Result<WsStream, Box<dyn Error>> {
    //  let server_url = Url::parse("ws://autorack.proxy.rlwy.net:55901")?;
    let server_url = Url::parse("ws://0.0.0.0:8080")?;

    let (ws_stream, _) = connect_async(server_url).await?;
    Ok(ws_stream)
}

// Handle WebSocket messages in a separate module
pub async fn handle_websocket<B: Backend>(
    app: &mut App,
    terminal: &mut Terminal<B>,
    _write: &mut futures_util::stream::SplitSink<WsStream, Message>,
    read: &mut futures_util::stream::SplitStream<WsStream>,
) -> io::Result<()> {
    loop {
        tokio::select! {
            ws_msg = read.next() => {
                if let Some(Ok(Message::Text(text))) = ws_msg {
                    app.handle_websocket_message(&text);

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
