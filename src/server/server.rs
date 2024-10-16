//  This is the main file that sets up the server and handles shutdown signals.
//  It spawns the WebSocket task and listens for shutdown signals using `tokio::select!`.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

mod app;
mod commander;
mod websocket;
use crate::app::App;
use crate::websocket::websocket_task;
#[tokio::main]
async fn main() {
    // Load port from ENV or default to 8080
    let port:u16 = std::env::var("PORT")
        .unwrap_or("8080".into())
        .parse()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Initialize server state
    let app = Arc::new(Mutex::new(App::new()));

    // Channel to broadcast shutdown signal
    let (shutdown_tx, _) = broadcast::channel(1);

    // Clone shutdown sender to pass it to websocket task
    let shutdown_tx_websocket = shutdown_tx.clone();

    // Start the WebSocket task
    let websocket_handle = tokio::spawn(websocket_task(addr, app.clone(), shutdown_tx_websocket));

    // Listen for shutdown signal (Ctrl+C)
    tokio::select! {
        _ = shutdown_signal() => {
            println!("Shutdown signal received");
            // Notify the websocket task to shut down
            shutdown_tx.send(()).unwrap();
        }
        _ = websocket_handle => {
            // Handle if the WebSocket task completes first (in case of error, etc.)
            println!("Websocket task completed");
        }
    }

    println!("Server shutdown complete.");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");
    println!("Ctrl+C received, shutting down...");
}
