use std::env;
use tokio::io::AsyncBufReadExt;

use client_connect::{ClientConnection, ClientConnectionResult};

#[tokio::main]
async fn main() -> ClientConnectionResult<()> {
    let url = env::args()
        .nth(1)
        .expect("Pass the server's wss:// address as a command-line argument");
    let username = env::args().nth(2).expect("2nd argument is username");

    let mut connection = ClientConnection::connect_to_server(&url).await?;

    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();
    while let Some(line) = lines.next_line().await.expect("io error") {
        if line.is_empty() {
            break;
        }
        connection.send(comms::ClientMessage::Append(comms::AppendChatEntry {
            username: username.clone(),
            content: line,
        }));
        if let Some(get) = connection.recv().await {
            println!("receive {:?}", get);
        }
    }

    Ok(())
}
