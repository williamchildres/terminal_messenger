// Import necessary packages from tokio, tokio_tungstenite, url and futures_util
use futures_util::{SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use url::Url;

// Define the asyncchronous main function
#[tokio::main]
async fn main() {
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
            write.send(Message::Text(line)).await.unwrap();
        }
    });

    // Wait until either read or write task is done. This could be because an error occurred, or
    // because the WebSocket was closed by the other end
    tokio::select! {
        _ = read_ws => (),
        _ = write_ws => (),
    }
}
