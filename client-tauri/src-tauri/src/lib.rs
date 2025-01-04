use std::{io};

use tokio::sync::Mutex;
use client_connect::{self, ClientConnectionError};
use comms::{self, ClientMessage, ServerMessage};
use tauri::State;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};


// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


struct ClientConn {
    connection: client_connect::ClientConnection,
    tx: UnboundedSender<ClientMessage>,
    rx: UnboundedReceiver<Result<ServerMessage, ClientConnectionError>>
}

#[derive(Default)]
struct ClientStore(Mutex<Option<ClientConn>>);

// Establish connection to client
#[tauri::command]
async fn init_client(client_conn: State<'_, ClientStore>) -> Result<String, String> {
    // Initialize connection
    let url = String::from("wss://127.0.0.1:12345/");
    let (connection, tx, rx) = client_connect::connect_to_server(&url)
        .await
        .map_err(|err| err.to_string())?;

    let mut state = client_conn.0.lock().await;
    *state = Some(ClientConn {
        connection,
        tx,
        rx,
    });

    Ok("Connection successful".to_string())
}

// Send message to client
#[tauri::command]
async fn send_message(username: &str, content: &str, client_conn: State<'_, ClientStore>) -> Result<(), ()> {
    let state = client_conn.0.lock().await;
    state.as_ref().unwrap().tx.send(comms::ClientMessage::Post {
        username: username.to_string(),
        content: content.to_string()
    }).expect("channel closed on server");

    Ok(())
}

#[tauri::command]
async fn recv_message(client_conn: State<'_, ClientStore>) -> Result<String, String> {
    let mut state = client_conn.0.lock().await;
    let server_message = state.as_mut().unwrap().rx.recv().await.unwrap().expect("todo");

    match server_message {
        comms::ServerMessage::NewEntry(chat_log_entry) => {
            let json_entry = serde_json::to_string(&chat_log_entry).map_err(|e| e.to_string());

            json_entry
        },
        _ => Err("Unsupported protocol".to_string())
    }
}

// Await message from client

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ClientStore(Default::default()))
        .invoke_handler(tauri::generate_handler![greet, init_client, send_message, recv_message])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
