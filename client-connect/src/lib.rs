use std::{error::Error, fmt, mem, sync::Arc};

use comms::Codable;
use futures_util::{future, SinkExt, StreamExt};
use tokio::{net::TcpStream, pin, sync::mpsc, task::JoinHandle};
use tokio_rustls::rustls as tls;
use tokio_tungstenite::{
    tungstenite::{
        self,
        client::IntoClientRequest,
        protocol::{frame::coding::CloseCode, CloseFrame, Message},
    },
    Connector, MaybeTlsStream, WebSocketStream,
};
use webpki::types::{pem::PemObject, CertificateDer};

/// An error that can occur on the client side.
#[derive(Debug)]
pub enum ClientConnectionError {
    InvalidRootCertificate {
        message: String,
        cause: tls::pki_types::pem::Error,
    },
    WebSocketFailure(tungstenite::Error),
    UnexpectedWebSocketMessage(Message),
    MalformedServerMessage(Message, comms::CodingErrorKind),
}

impl fmt::Display for ClientConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientConnectionError::InvalidRootCertificate {
                message,
                cause,
            } => cause.fmt(f),
            ClientConnectionError::WebSocketFailure(cause) => cause.fmt(f),
            ClientConnectionError::UnexpectedWebSocketMessage(message) => {
                write!(f, "Unexpected WebSocket message: {:?}", message)
            }
            ClientConnectionError::MalformedServerMessage(message, cause) => {
                write!(f, "Malformed server message {:?}: {}", message, cause)
            }
        }
    }
}

impl Error for ClientConnectionError {}

/// [`std::result::Result`] wrapper for client errors.
pub type ClientConnectionResult<T> =
    std::result::Result<T, ClientConnectionError>;

/// Opens a TLS-encrypted web socket to the server at `server_address`.
async fn open_websocket<R: IntoClientRequest + Unpin>(
    server_address: R,
) -> ClientConnectionResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let mut root_store;
    if cfg!(feature = "local") {
        root_store = tls::RootCertStore::empty();
        for cert in CertificateDer::pem_file_iter("testing_cert/rootCA.crt")
            .map_err(|cause| ClientConnectionError::InvalidRootCertificate {
                message:
                    "Did you remember to run ./gen_cert.sh for local testing?"
                        .to_owned(),
                cause,
            })?
        {
            let cert = cert.map_err(|cause| {
                ClientConnectionError::InvalidRootCertificate {
                    message:
                        "Could not extract certificate from root authority file"
                            .to_owned(),
                    cause,
                }
            })?;
            root_store
                .add(cert)
                .expect("failed to add to root store somehow");
        }
    } else {
        root_store = tls::RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        );
    }

    let tls_client_config = tls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let tls_connector = Connector::Rustls(Arc::new(tls_client_config));

    let (websocket, _) = tokio_tungstenite::connect_async_tls_with_config(
        server_address,
        Some(Default::default()),
        false,
        Some(tls_connector),
    )
    .await
    .map_err(ClientConnectionError::WebSocketFailure)?;

    Ok(websocket)
}

struct UnboundedBichannel<Sent, Received> {
    tx: mpsc::UnboundedSender<Sent>,
    rx: mpsc::UnboundedReceiver<Received>,
}

fn unbounded_bichannel<From, To>(
) -> (UnboundedBichannel<From, To>, UnboundedBichannel<To, From>) {
    let (from_tx, from_rx) = mpsc::unbounded_channel();
    let (to_tx, to_rx) = mpsc::unbounded_channel();
    (
        UnboundedBichannel {
            tx: from_tx,
            rx: to_rx,
        },
        UnboundedBichannel {
            tx: to_tx,
            rx: from_rx,
        },
    )
}

async fn client_actor(
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    close_connection_channel: UnboundedBichannel<
        Option<CloseFrame>,
        Option<CloseFrame>,
    >,
    channel_with_user: UnboundedBichannel<
        ClientConnectionResult<comms::ServerMessage>,
        comms::ClientMessage,
    >,
) {
    println!("client actor spawned");

    let (mut websocket_write, mut websocket_read) = websocket.split();
    let UnboundedBichannel {
        tx: close_tx,
        rx: mut close_rx,
    } = close_connection_channel;
    let UnboundedBichannel {
        tx: user_tx,
        rx: mut user_rx,
    } = channel_with_user;

    tokio::join!(
        async {
            while let Some(Ok(message)) = websocket_read.next().await {
                match message.clone() {
                    Message::Binary(message_bytes) => {
                        let server_message =
                        comms::ServerMessage::try_from_bytes(&message_bytes)
                            .map_err(|coding_error| {
                                ClientConnectionError::MalformedServerMessage(
                                    message,
                                    *coding_error,
                                )
                            });
                        user_tx.send(server_message).expect(
                            "receiver should not have been dropped/closed",
                        );
                    }
                    Message::Close(close_frame) => {
                        close_tx.send(close_frame).expect(
                            "receiver should not have been dropped/closed",
                        );
                        break;
                    }
                    _ => {}
                }
            }
        },
        async {
            loop {
                let close_frame = close_rx.recv();
                let client_message = user_rx.recv();
                pin!(close_frame, client_message);
                match future::select(client_message, close_frame).await {
                    future::Either::Left((Some(client_message), _)) => {
                        websocket_write
                            .send(Message::binary(client_message.to_bytes()))
                            .await
                            .expect("todo");
                    }
                    future::Either::Right((Some(close_frame), _)) => {
                        websocket_write
                            .send(Message::Close(close_frame))
                            .await
                            .expect("failed to flush item to websocket ig?");
                        break;
                    }
                    _ => {
                        break;
                    }
                }
            }
        },
    );
}

/// Handle for a client connection that automatically closes the connection on
/// drop (or explicit [`ClientConnection::close`].
pub struct ClientConnection {
    close_connection_channel:
        UnboundedBichannel<Option<CloseFrame>, Option<CloseFrame>>,
    actor_thread: Option<JoinHandle<()>>,
}

impl ClientConnection {
    // Manually closes the connection.
    pub fn close(mut self) {
        self.async_drop();
    }

    fn async_drop(&mut self) {
        if let Some(actor_thread) = mem::take(&mut self.actor_thread) {
            // Forgive me, Ferris, for I have async dropped.
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on( async {
                self.close_connection_channel.tx.send(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason:"client connection handle dropped".into(),
                })).expect("channel with actor should be open when ClientConnection is being dropped");
                if let Some(close_frame_response) = self.close_connection_channel.rx.recv().await {
                    println!("client closing connection: {:?}", close_frame_response);
                }
                actor_thread.abort();
            });
            });
        }
    }
}

impl Drop for ClientConnection {
    fn drop(&mut self) {
        self.async_drop();
    }
}

/// Spawns a client thread to communicate with the given server over a
/// TLS-encrypted websocket, returning a client handle. The connection is closed
/// when the handle is dropped.
///
/// # Example
///
/// ```no_run
/// # use client_connect::ClientConnectionResult;
/// # async fn foo() -> ClientConnectionResult<()> {
/// let client_connection =
///     client_connect::connect_to_server("wss://127.0.0.1:8080").await?;
/// # Ok(())
/// # }
/// ```
pub async fn connect_to_server<R: IntoClientRequest + Unpin>(
    server_address: R,
) -> ClientConnectionResult<(
    ClientConnection,
    mpsc::UnboundedSender<comms::ClientMessage>,
    mpsc::UnboundedReceiver<ClientConnectionResult<comms::ServerMessage>>,
)> {
    let websocket = open_websocket(server_address).await?;

    let (local_bichannel, actor_bichannel) = unbounded_bichannel();
    let (user_bichannel, other_actor_bichannel) = unbounded_bichannel();

    let actor_thread = tokio::spawn(client_actor(
        websocket,
        actor_bichannel,
        other_actor_bichannel,
    ));

    Ok((
        ClientConnection {
            close_connection_channel: local_bichannel,
            actor_thread: Some(actor_thread),
        },
        user_bichannel.tx,
        user_bichannel.rx,
    ))
}
