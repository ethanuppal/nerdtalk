use std::sync::Arc;

use comms::Codable;
use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::mpsc, task::JoinHandle};
use tokio_rustls::rustls as tls;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{self, client::IntoClientRequest, protocol::Message},
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

/// [`std::result::Result`] wrapper for client errors.
pub type ClientConnectionResult<T> =
    std::result::Result<T, ClientConnectionError>;

/// Opens a TLS-encrypted web socket to the server at `server_address`.
pub async fn open_websocket<R: IntoClientRequest + Unpin>(
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

    let (websocket, _) = connect_async_tls_with_config(
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

enum ActorWrapper<T> {
    Wrapped(T),
    Close,
}

async fn client_actor(
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    channel_with_user: UnboundedBichannel<
        ActorWrapper<ClientConnectionResult<comms::ServerMessage>>,
        ActorWrapper<comms::ClientMessage>,
    >,
) {
    println!("client actor spawned");

    let (mut websocket_write, mut websocket_read) = websocket.split();
    let UnboundedBichannel { tx, mut rx } = channel_with_user;

    tokio::join!(
        async {
            while let Some(Ok(message)) = websocket_read.next().await {
                match &message {
                    Message::Binary(message_bytes) => {
                        let server_message =
                        comms::ServerMessage::try_from_bytes(message_bytes)
                            .map_err(|coding_error| {
                                ClientConnectionError::MalformedServerMessage(
                                    message,
                                    *coding_error,
                                )
                            });
                        tx.send(ActorWrapper::Wrapped(server_message)).expect(
                            "receiver should not have been dropped/closed",
                        );
                    }
                    Message::Close(_) => {
                        tx.send(ActorWrapper::Close).expect(
                            "receiver should not have been dropped/closed",
                        );
                        break;
                    }
                    _ => {}
                }
            }
        },
        async {
            while let Some(command) = rx.recv().await {
                match command {
                    ActorWrapper::Wrapped(client_message) => {
                        websocket_write
                            .send(Message::binary(client_message.to_bytes()))
                            .await
                            .expect("todo");
                    }
                    ActorWrapper::Close => {
                        websocket_write
                            .send(Message::Close(None))
                            .await
                            .expect("failed to flush item to websocket ig?");
                        break;
                    }
                }
            }
        },
    );
}

pub struct ClientConnection {
    channel_with_actor: UnboundedBichannel<
        ActorWrapper<comms::ClientMessage>,
        ActorWrapper<ClientConnectionResult<comms::ServerMessage>>,
    >,
    actor_thread: JoinHandle<()>,
}

impl ClientConnection {
    /// Spawns a client thread to communicate with the given server over a TLS-encrypted websocket,
    /// returning a client handle. The connection is closed when the handle is dropped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use client_connect::{ClientConnection, ClientConnectionResult};
    /// # async fn foo() -> ClientConnectionResult<()> {
    /// let client_connection = ClientConnection::connect_to_server("wss://127.0.0.1:8080").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_to_server<R: IntoClientRequest + Unpin>(
        server_address: R,
    ) -> ClientConnectionResult<
        Self,
        // mpsc::UnboundedSender<comms::ClientMessage>,
        // mpsc::UnboundedReceiver<comms::ServerMessage>,
    > {
        let websocket = open_websocket(server_address).await?;

        let (local_bichannel, actor_bichannel) = unbounded_bichannel();

        let actor_thread =
            tokio::spawn(client_actor(websocket, actor_bichannel));

        Ok(Self {
            channel_with_actor: local_bichannel,
            actor_thread,
        })
    }

    // TODO: figure out how to directly expose channels someho
    pub fn send(&mut self, message: comms::ClientMessage) {
        let _ = self
            .channel_with_actor
            .tx
            .send(ActorWrapper::Wrapped(message));
    }
}

impl Drop for ClientConnection {
    fn drop(&mut self) {
        // Forgive me, Ferris, for I have async dropped.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on( async {
                self.channel_with_actor.tx.send(ActorWrapper::Close).expect("channel with actor should be open when ClientConnection is being dropped");
                while let Some(server_message) = self.channel_with_actor.rx.recv().await {
                    if matches!(server_message, ActorWrapper::Close) {
                        break;
                    }
                }
                self.actor_thread.abort();
            });
        });
    }
}
