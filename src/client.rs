use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::connect_async;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tungstenite::protocol::Message;
use tokio::net::TcpStream;
use url::Url;

#[tokio::main]
async fn main() {
    let server_url = Url::parse("ws://127.0.0.1:8080").unwrap();
    let (ws_stream, _) = connect_async(server_url).await.expect("Failed to connect");
    println!("Connected to the server");

    let (mut write, mut read) = ws_stream.split();
    let stdin = BufReader::new(io::stdin());

    // Task to read from WebSocket
    let read_ws = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            println!("Received: {}", text);
        }
    });

    // Task to read from terminal and send messages
    let write_ws = tokio::spawn(async move {
        let mut lines = stdin.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            write.send(Message::Text(line)).await.unwrap();
        }
    });

    tokio::select! {
        _ = read_ws => (),
        _ = write_ws => (),
    }
}

