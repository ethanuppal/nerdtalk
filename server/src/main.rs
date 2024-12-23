use std::{collections::HashMap, env, fmt::Display, io, sync::Arc};

use chat::ChatLogEntry;
use comms::{AppendChatEntry, Codable};
use futures_util::{SinkExt, StreamExt};
use log::info;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
    task::JoinHandle,
};
use tokio_rustls::{
    rustls::{
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        ServerConfig,
    },
    TlsAcceptor,
};
use tokio_tungstenite::tungstenite::Message;

#[derive(Default)]
struct FakeChatLog {
    lmao: Vec<ChatLogEntry>,
}

impl FakeChatLog {
    fn append(&mut self, append: AppendChatEntry) -> ChatLogEntry {
        let entry = ChatLogEntry::new_timestamped_now(
            self.lmao.len(),
            append.username,
            append.content,
        );
        self.lmao.push(entry.clone());
        entry
    }
}

#[derive(Debug)]
enum Error {
    Tls(tokio_rustls::rustls::Error),
    Io(io::Error),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();

    let certificate = if cfg!(feature = "local") {
        CertificateDer::from_pem_file("testing_cert/cert.pem")
            .expect("Did you remember to run ./gen_cert.sh for local testing?")
    } else {
        todo!("Ask Peter for midcode certificate")
    };

    let private_key = if cfg!(feature = "local") {
        PrivateKeyDer::from_pem_file("testing_cert/key.pem")
            .expect("Did you remember to run ./gen_cert.sh for local testing?")
    } else {
        todo!("Ask Peter for midcode private key")
    };

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], private_key)
        .map_err(Error::Tls)?;

    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8443".to_string());

    let listener = TcpListener::bind(&addr).await.map_err(Error::Io)?;
    info!("Listening on: {}", addr);

    let (message_tx, mut message_rx) = mpsc::unbounded_channel();
    let sessions = Arc::new(RwLock::new(HashMap::new()));

    let mut fake_chat_log = FakeChatLog::default();

    let sessions_for_thread = sessions.clone();
    tokio::spawn(async move {
        // TODO: thread pool
        while let Ok((tcp_stream, _)) = listener.accept().await {
            let client_address =
                tcp_stream.peer_addr().expect("missing address");
            match new_client_connection(
                tcp_stream,
                client_address,
                &tls_acceptor,
                &message_tx,
            )
            .await
            {
                Ok(session) => {
                    sessions_for_thread
                        .write()
                        .await
                        .insert(client_address, session);
                }
                Err(error) => {
                    println!(
                        "failed to make session with {}: {:?}",
                        client_address, error
                    );
                }
            }
        }
    });

    while let Some(append) = message_rx.recv().await {
        let entry = fake_chat_log.append(append);
        for (client, session) in sessions.read().await.iter() {
            session.send(comms::ServerMessage::NewEntry(entry.clone()));
        }
    }

    Ok(())
}

#[derive(Debug)]
enum SessionError {
    IO(io::Error),
    WebSocket(tokio_tungstenite::tungstenite::Error),
}

struct Session {
    to_client_tx: mpsc::UnboundedSender<Message>,
    join_handle: JoinHandle<()>,
}

impl Session {
    fn send(&self, message: comms::ServerMessage) {
        self.to_client_tx
            .send(Message::binary(message.to_bytes()))
            .expect("todo");
    }
}

async fn new_client_connection<D: Display + Clone>(
    tcp_stream: TcpStream,
    client_address: D,
    tls_acceptor: &TlsAcceptor,
    message_tx: &mpsc::UnboundedSender<AppendChatEntry>,
) -> Result<Session, SessionError> {
    let tls_acceptor = tls_acceptor.clone();
    let message_tx = message_tx.clone();

    let tls_stream = tls_acceptor
        .accept(tcp_stream)
        .await
        .map_err(SessionError::IO)?;
    let websocket = tokio_tungstenite::accept_async(tls_stream)
        .await
        .map_err(SessionError::WebSocket)?;

    println!("established connection with {}", client_address);

    let (mut websocket_write, mut websocket_read) = websocket.split();

    let (write_websocket_tx, mut write_websocket_rx) =
        mpsc::unbounded_channel();

    let write_websocket_thread_tx = write_websocket_tx.clone();
    let join_handle = tokio::spawn(async move {
        tokio::join!(
            async {
                while let Some(message) = websocket_read.next().await {
                    match message {
                        Ok(message) => match message {
                            Message::Binary(message_bytes) => {
                                let client_request =
                                    comms::ClientMessage::try_from_bytes(
                                        &message_bytes,
                                    )
                                    .expect("failed to decode client request");
                                println!(
                                    "server got message: {:?}",
                                    client_request
                                );
                                match client_request {
                                    comms::ClientMessage::Append(append) => {
                                        message_tx.send(append).expect("todo");
                                    }
                                    _ => todo!(),
                                }
                            }
                            Message::Close(close_frame) => {
                                write_websocket_thread_tx
                                    .send(Message::Close(close_frame))
                                    .expect("todo");
                                println!("closing connection");
                                break;
                            }
                            _ => todo!(),
                        },
                        Err(error) => {
                            todo!("{:?}", error);
                        }
                    }
                }
            },
            async {
                while let Some(message) = write_websocket_rx.recv().await {
                    if matches!(message, Message::Close(_)) {
                        // websocket_write.send(message).await.expect("todo");
                        break;
                    }
                    websocket_write.send(message).await.expect("todo");
                }
            }
        );
    });

    Ok(Session {
        to_client_tx: write_websocket_tx,
        join_handle,
    })
}
