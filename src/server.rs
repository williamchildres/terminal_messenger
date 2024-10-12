// Import necessary modules from tokio, tokio_tungstenite, and std
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

// Define the asynchronous main function
#[tokio::main]
async fn main() {
    // Bind the server to the local address and port 8080
    let addr = "127.0.0.1:8080";

    // Create a TCP listener that listens for incoming connections on the specified address
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    // Print out a message indicating the server is listening on the specified address
    println!("Server listening on {}", addr);

    // Create a broadcast channel with a capacity of 100 messages
    let (tx, _rx) = tokio::sync::broadcast::channel(100);

    // Create an Arc Mutex protected vector for storing client identifiers (IP addresses)
    let clients = Arc::new(Mutex::new(Vec::<String>::new()));

    // Start accepting incoming connections in an infinite loop
    while let Ok((stream, _)) = listener.accept().await {
        // Clone the transmitter part of the broadcast channel and clients list for each new
        // connection
        let tx = tx.clone();
        let clients = clients.clone();

        // Spawn a new task for each new connection using Tokio's async runtime
        tokio::spawn(async move {
            // Upgrade TCP stream into WebSocket stream using async handshake process
            let ws_stream = accept_async(stream)
                .await
                .expect("Error during the WebSocket handshake");

            // Get client IP address from TCP Stream underlying WebSocket strream and format it
            // into string as client identifiers
            let peer_addr = ws_stream
                .get_ref()
                .peer_addr()
                .expect("Connected streams should have a peer address");
            let client_id = format!("{}", peer_addr);

            // Log new client connection
            println!("New client connected: {}", client_id);

            // Split the WebSocket stream into a sender and receiver part
            let (mut ws_tx, mut ws_rx) = ws_stream.split();

            // Subscribe to the broadcast channel to receive message
            let mut rx = tx.subscribe();

            // Add this client's ID into shared vector of clients (safely by locking mutex)
            let clients_guard = clients.clone();
            clients_guard.lock().unwrap().push(client_id.clone()); // Cloned here to use later

            // Spawn a task for receiving broadcast messages and sending them over WebSocket to
            // this specific client
            let tx_clone = tx.clone();
            let client_send = tokio::spawn(async move {
                while let Ok(message) = rx.recv().await {
                    ws_tx.send(Message::Text(message)).await.unwrap();
                }
            });

            // Spawn another task for receiving any message sent by this specific client over
            // WebSocket, formatting it with sender identification, printing onto console and then
            // broadcasting over channel
            let client_id_receive = client_id.clone();
            let client_receive = tokio::spawn(async move {
                while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
                    let message = format!("{}: {}", client_id_receive, text);
                    println!("{}", message);
                    tx_clone.send(message).unwrap();
                }
            });

            // Wait until either send or receive task is done. This could be because an error
            // occurred, or because the WebSocket was closed by the other end.
            tokio::select! {
                _ = client_send => (),
                _ = client_receive => (),
            };

            // Log that client has disconnected
            println!("Client {} disconnected", client_id);
        });
    }
}
