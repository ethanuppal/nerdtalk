use std::env;
use tokio::io::AsyncBufReadExt;

use client_connect::{ClientConnection, ClientConnectionResult};

#[tokio::main]
async fn main() -> ClientConnectionResult<()> {
    let url = env::args().nth(1).unwrap_or_else(|| {
        panic!("Pass the server's wss:// address as a command-line argument")
    });

    let mut connection = ClientConnection::connect_to_server(&url).await?;

    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();
    while let Some(line) = lines.next_line().await.expect("io error") {
        if line.is_empty() {
            break;
        }
        connection.send(comms::ClientMessage::Append(comms::AppendChatEntry {
            content: line,
        }));
    }

    Ok(())
}
