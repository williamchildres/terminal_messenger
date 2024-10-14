use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

mod app;
mod websocket;
use crate::app::App;
use crate::websocket::websocket_task;
#[tokio::main]
async fn main() {
    // Initialize server state
    let app = Arc::new(Mutex::new(App::new()));

    // Start the Websocket task
    websocket_task(app).await;
}
