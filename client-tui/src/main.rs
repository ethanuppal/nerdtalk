use std::{env, io, sync::Arc};

use client_tui::app::App;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let url = env::args().nth(1).unwrap_or_else(|| {
        panic!("Pass the server's wss:// address as a command-line argument")
    });

    let (connection, tx, mut rx) = client_connect::connect_to_server(&url)
        .await
        .map_err(io::Error::other)?;

    let messages = Arc::new(RwLock::new(vec![]));
    let app_messages = messages.clone();

    tx.send(comms::ClientMessage::Request {
        count: 50,
        up_to_slot_number: None,
    })
    .expect("todo");

    let mut app = App::new(tx);

    tokio::spawn(async move {
        while let Some(server_message) = rx.recv().await {
            let server_message = server_message.expect("todo");
            match server_message {
                comms::ServerMessage::NewEntry(chat_log_entry) => loop {
                    if let Ok(mut lock) = messages.try_write() {
                        lock.push(chat_log_entry);
                        break;
                    }
                },
                comms::ServerMessage::EntryRange(entries) => {
                    if !entries.is_empty() {
                        // since we don't have to worry about updates until v0.2, this is going to be
                        // contiguous
                        // TODO(haadi): I'm sure you can find a smarter way, e.g., if your only
                        // requests are for earlier messages, you can just automatically insert them at
                        // the start of the array instead of "finding" the insertion point
                        loop {
                            if let Ok(mut lock) = messages.try_write() {
                                let insertion_point = lock
                                    .iter()
                                    .enumerate()
                                    .find_map(|(index, entry)| {
                                        if entry.slot_number
                                            == entries[0].slot_number
                                        {
                                            Some(index)
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(0);
                                lock.splice(
                                    insertion_point..insertion_point,
                                    entries,
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }
    });

    crossterm::terminal::enable_raw_mode()?; // Ensure raw mode is enabled for cursor shape changes
    let mut terminal = ratatui::init();

    let app_result = app.run(&mut terminal, app_messages).await;
    ratatui::restore();
    connection.close();
    app_result
}
