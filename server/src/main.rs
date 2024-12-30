use std::net;
use std::{
    collections::HashMap,
    env, error,
    fmt::{self},
    io,
    sync::Arc,
};

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
    lmao: Vec<chat::Entry>,
}

impl FakeChatLog {
    fn append(&mut self, append: AppendChatEntry) -> chat::Entry {
        let entry = chat::Entry::new_timestamped_now(
            self.lmao.len(),
            append.username,
            chat::Content::Original(chat::MessageText(append.content)),
        );
        self.lmao.push(entry.clone());
        entry
    }

    fn entries(
        &self,
        count: usize,
        last_slot: Option<usize>,
    ) -> Vec<chat::Entry> {
        let Some(last_entry) = self.lmao.last() else {
            return vec![];
        };
        let last_slot = last_slot.unwrap_or(last_entry.slot_number);

        let last_index = self
            .lmao
            .iter()
            .enumerate()
            .rfind(|(_, entry)| entry.slot_number == last_slot)
            .expect("slot missing todo don't crash server on this lol")
            .0;

        self.lmao[last_index - count + 1..last_index + 1].to_owned()
    }
}

#[derive(Debug)]
enum Error {
    Tls(tokio_rustls::rustls::Error),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Tls(error) => error.fmt(f),
            Error::Io(error) => error.fmt(f),
        }
    }
}

impl error::Error for Error {}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();

    let certificate = if cfg!(feature = "local") {
        CertificateDer::from_pem_file("testing_cert/cert.pem").expect(
            "Did you remember to run ./scripts/gen_cert.sh for local testing?",
        )
    } else {
        todo!("Ask Peter for midcode certificate")
    };

    let private_key = if cfg!(feature = "local") {
        PrivateKeyDer::from_pem_file("testing_cert/key.pem").expect(
            "Did you remember to run ./scripts/gen_cert.sh for local testing?",
        )
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
        .expect("server takes the address:port it should listen to as a command-line argument");

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
                        "failed to make session with {}: {}",
                        client_address, error
                    );
                }
            }
        }
    });

    while let Some((sender, message)) = message_rx.recv().await {
        match message {
            comms::ClientMessage::Append(append_chat_entry) => {
                let entry = fake_chat_log.append(append_chat_entry);
                for (_, session) in sessions.read().await.iter() {
                    session.send(comms::ServerMessage::NewEntry(entry.clone()));
                }
            }
            comms::ClientMessage::Request {
                count,
                up_to_slot_number,
            } => {
                let entries = fake_chat_log.entries(count, up_to_slot_number);
                sessions.read().await[&sender]
                    .send(comms::ServerMessage::EntryRange(entries));
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
enum SessionError {
    IO(io::Error),
    WebSocket(tokio_tungstenite::tungstenite::Error),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::IO(error) => error.fmt(f),
            SessionError::WebSocket(error) => error.fmt(f),
        }
    }
}

impl error::Error for SessionError {}

struct Session {
    to_client_tx: mpsc::UnboundedSender<Message>,
    _join_handle: JoinHandle<()>,
}

impl Session {
    fn send(&self, message: comms::ServerMessage) {
        self.to_client_tx
            .send(Message::binary(message.to_bytes()))
            .expect("todo");
    }
}

async fn new_client_connection(
    tcp_stream: TcpStream,
    client_address: net::SocketAddr,
    tls_acceptor: &TlsAcceptor,
    message_tx: &mpsc::UnboundedSender<(net::SocketAddr, comms::ClientMessage)>,
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
                                message_tx
                                    .send((client_address, client_request))
                                    .expect("todo");
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
        _join_handle: join_handle,
    })
}
