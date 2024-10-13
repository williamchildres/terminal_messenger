use std::sync::{Arc, Mutex};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

pub async fn websocket_task(app: Arc<Mutex<App>>) {
    while let Ok((stream, _)) = listener.accept().await {
        let app = app.clone();

        tokio::spawn(async move {
            let ws_stream: WebSocketStream<_> =
                accept_async(stream).await.expect("Error during handshake");

            handle_websocket_connection(ws_stream, app).await;
        });
    }
}

async fn handle_websocket_connection(ws_stream: WebSocketStream<_>, app: Arc<Mutex<App>>) {
    // Handle the WebSocket connection, messages, and client states here...
}
