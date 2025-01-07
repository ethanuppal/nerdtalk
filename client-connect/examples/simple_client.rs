use std::env;

use client_connect::ClientConnectionResult;
use tokio::io::AsyncBufReadExt;

#[tokio::main]
async fn main() -> ClientConnectionResult<()> {
    let url = env::args()
        .nth(1)
        .expect("Pass the server's wss:// address as a command-line argument");
    let username = env::args().nth(2).expect("2nd argument is username");

    let (connection, tx, mut rx) =
        client_connect::connect_to_server(&url).await?;

    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();
    while let Some(line) = lines.next_line().await.expect("io error") {
        if line.is_empty() {
            break;
        }
        tx.send(comms::ClientMessage::Post {
            username: username.clone(),
            content: line,
        })
        .expect("todo");
    }

    tokio::spawn(async move {
        while let Some(server_message) = rx.recv().await {
            let server_message = server_message.expect("todo");
            match server_message {
                comms::ServerMessage::NewEntry(chat_log_entry) => {
                    println!(
                        "{}: {:?}",
                        chat_log_entry.metadata.username,
                        chat_log_entry.text_content()
                    );
                }
                comms::ServerMessage::EntryRange {
                    client_id: _,
                    entries,
                } => {
                    println!("got entry range: {:?}", entries);
                }
            }
        }
    });

    // you can also just let this drop
    connection.close();

    Ok(())
}
