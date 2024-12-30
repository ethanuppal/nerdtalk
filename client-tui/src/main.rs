use std::{env, io, sync::Arc};

pub mod app;
pub mod vim;
use crate::app::App;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let url = env::args().nth(1).unwrap_or_else(|| {
        panic!("Pass the server's wss:// address as a command-line argument")
    });

    let (connection, tx, mut rx) = client_connect::connect_to_server(&url)
        .await
        .map_err(io::Error::other)?;

    let messages = Arc::new(RwLock::new(vec![
        "Welcome to the chat!".into(),
        "Type a message and press Enter.".into(),
    ]));
    let app_messages = messages.clone();

    let mut app = App::new(tx);

    tokio::spawn(async move {
        while let Some(server_message) = rx.recv().await {
            let server_message = server_message.expect("todo");
            match server_message {
                comms::ServerMessage::NewEntry(chat_log_entry) => loop {
                    if let Ok(mut lock) = messages.try_write() {
                        lock.push(format!(
                            "{}: {}",
                            chat_log_entry.username, chat_log_entry.content
                        ));
                        break;
                    }
                },
            }
        }
    });

    crossterm::terminal::enable_raw_mode()?;
    let mut terminal = ratatui::init();

    let app_result = app.run(&mut terminal, app_messages).await;
    ratatui::restore();
    connection.close();
    app_result
}
