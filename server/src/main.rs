use std::{
    cmp,
    collections::HashMap,
    env, error,
    fmt::{self},
    io, net,
    sync::Arc,
};

use comms::Codable;
use futures_util::{SinkExt, StreamExt};
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
    fn post(&mut self, username: String, content: String) -> chat::Entry {
        let entry = chat::Entry::new_timestamped_now(
            self.lmao.len(),
            username,
            chat::Content::Original(chat::MessageText(content)),
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
        let count = cmp::min(count, last_index + 1);

        // needs to +1 before -count
        self.lmao[last_index + 1 - count..last_index + 1].to_owned()
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
    let _ = env_logger::try_init_from_env("LOG");

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

    let address = env::args()
        .nth(1)
        .expect("Server takes the address:port it should listen to as a command-line argument");

    let listener = TcpListener::bind(&address).await.map_err(Error::Io)?;
    log::info!("Listening on {}", address);

    let (message_tx, mut message_rx) = mpsc::unbounded_channel();
    let sessions = Arc::new(RwLock::new(HashMap::new()));

    let mut fake_chat_log = FakeChatLog::default();

    let sessions_for_thread = sessions.clone();
    tokio::spawn(async move {
        // TODO: thread pool
        while let Ok((tcp_stream, _)) = listener.accept().await {
            match tcp_stream.peer_addr() {
                Ok(client_address) => match new_client_connection(
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
                        log::error!(
                            "Failed to establish session with client address {}: {}",
                            client_address,
                            error
                        );
                    }
                },
                Err(error) => {
                    log::warn!(
                        "Failed to extract client address from TCP stream: {}",
                        error
                    );
                }
            }
        }
    });

    while let Some((sender, message)) = message_rx.recv().await {
        log::info!("Processing message: {:?}", message);
        match message {
            comms::ClientMessage::Post { username, content } => {
                let entry = fake_chat_log.post(username, content);
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
    client_address: net::SocketAddr,
    to_client_tx: mpsc::UnboundedSender<Message>,
    _join_handle: JoinHandle<()>,
}

impl Session {
    fn send(&self, message: comms::ServerMessage) {
        log::info!(
            "Sending reply {:?} to client address {}",
            message,
            self.client_address
        );
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

    log::info!(
        "Established connection with client address {}",
        client_address
    );

    let (mut websocket_write, mut websocket_read) = websocket.split();

    let (write_websocket_tx, mut write_websocket_rx) =
        mpsc::unbounded_channel();

    let write_websocket_thread_tx = write_websocket_tx.clone();
    let join_handle = tokio::spawn(async move {
        tokio::join!(
            async {
                while let Some(message) = websocket_read.next().await {
                    match message {
                        Ok(message) => {
                            match message {
                                Message::Binary(message_bytes) => {
                                    match comms::ClientMessage::try_from_bytes(
                                        &message_bytes,
                                    ) {
                                        Ok(client_request) => {
                                            log::info!(
                                            "Received {:?} from client address {}",
                                            client_request,
                                            client_address
                                        );
                                            message_tx
                                            .send((
                                                client_address,
                                                client_request,
                                            ))
                                            .expect("Failed to queue client message for processing");
                                        }
                                        Err(decoding_error) => {
                                            log::error!("Failed to decode client request message: {}", decoding_error);
                                        }
                                    }
                                }
                                Message::Close(close_frame) => {
                                    write_websocket_thread_tx
                                    .send(Message::Close(close_frame))
                                    .expect("Failed to close websocket with client");
                                    log::info!("Closing connection with client address {}", client_address);
                                    break;
                                }
                                _ => todo!(),
                            }
                        }
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
                    websocket_write.send(message).await.unwrap_or_else(|_| {
                        panic!(
                            "Failed to send message to client address {}",
                            client_address
                        )
                    });
                }
            }
        );
    });

    Ok(Session {
        client_address,
        to_client_tx: write_websocket_tx,
        _join_handle: join_handle,
    })
}
