use std::{env, io, sync::Arc};

use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, SetCursorStyle},
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent,
        MouseEventKind,
    },
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

mod vim;
use tokio::sync::{mpsc, RwLock};
use vim::{Mode, VimCommand};

/// Indicates which part of the UI is currently in “focus.”
#[derive(Debug, PartialEq)]
pub enum Focus {
    Messages,
    Input,
}

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
                comms::ServerMessage::NewEntry(chat_log_entry) => {
                    messages.write().await.push(format!(
                        "{}: {}",
                        chat_log_entry.username, chat_log_entry.content
                    ));
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
