use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    println!("Server listening on {}", addr);

    let (tx, _rx) = tokio::sync::broadcast::channel(100);
    let clients = Arc::new(Mutex::new(Vec::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        let clients = clients.clone();

        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("Error during the WebSocket handshake");

            // Extract the peer address from the WebSocket stream
            let peer_addr = ws_stream
                .get_ref()
                .peer_addr()
                .expect("Connected streams should have a peer address");
            let client_id = format!("{}", peer_addr);
            println!("New client connected: {}", client_id);

            let (mut ws_tx, mut ws_rx) = ws_stream.split();
            let mut rx = tx.subscribe();

            let clients_guard = clients.clone();
            clients_guard.lock().unwrap().push(client_id.clone()); // Cloned here to use later

            let tx_clone = tx.clone();
            let client_send = tokio::spawn(async move {
                while let Ok(message) = rx.recv().await {
                    ws_tx.send(Message::Text(message)).await.unwrap();
                }
            });

            // Clone the client_id for use inside the receive task
            let client_id_receive = client_id.clone();
            let client_receive = tokio::spawn(async move {
                while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
                    let message = format!("{}: {}", client_id_receive, text);
                    println!("{}", message);
                    tx_clone.send(message).unwrap();
                }
            });

            tokio::select! {
                _ = client_send => (),
                _ = client_receive => (),
            };

            println!("Client {} disconnected", client_id); // Original client_id still available here
        });
    }
}
