use std::{env, sync::Arc, time::Duration};

use comms::Codable;
use futures_util::{future, pin_mut, StreamExt};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
};
use tokio_rustls::rustls as tls;
use tokio_tungstenite::{
    connect_async_tls_with_config, tungstenite,
    tungstenite::{client::IntoClientRequest, protocol::Message},
    Connector, MaybeTlsStream, WebSocketStream,
};
use webpki::types::{pem::PemObject, CertificateDer};

/// An error that can occur on the client side.
pub enum Error {
    InvalidRootCertificate {
        message: String,
        cause: tls::pki_types::pem::Error,
    },
    WebSocketFailure {
        cause: tungstenite::Error,
    },
}

/// [`std::result::Result`] wrapper for client errors.
pub type Result<T> = std::result::Result<T, Error>;

/// Opens a TLS-encrypted web socket to the server at `server_address`.
pub async fn open_websocket<R: IntoClientRequest + Unpin>(
    server_address: R,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let mut root_store;
    if cfg!(feature = "local") {
        root_store = tls::RootCertStore::empty();
        for cert in CertificateDer::pem_file_iter("testing_cert/rootCA.crt")
            .map_err(|cause| Error::InvalidRootCertificate {
                message:
                    "Did you remember to run ./gen_cert.sh for local testing?"
                        .to_owned(),
                cause,
            })?
        {
            let cert = cert.map_err(|cause| Error::InvalidRootCertificate {
                message:
                    "Could not extract certificate from root authority file"
                        .to_owned(),
                cause,
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

    let (ws_stream, _) = connect_async_tls_with_config(
        server_address,
        Some(Default::default()),
        false,
        Some(tls_connector),
    )
    .await
    .map_err(|cause| Error::WebSocketFailure { cause })?;

    Ok(ws_stream)
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    let mut stdin = BufReader::new(tokio::io::stdin());
    loop {
        let mut buf = String::new();
        tokio::select! {
            _ = interval.tick() => {
                let ping = comms::ClientRequest::Ping {
                    last_slot_number: 0,
                };
                tx.unbounded_send(Message::binary(ping.to_bytes())).expect("failed to send");
            },
            input = stdin.read_line(&mut buf) => {
                let n = match input {
                    Err(_) | Ok(0) => break,
                    Ok(n) => n,
                };
                buf.truncate(n);
                let append = comms::ClientRequest::Append {
                    content: buf,
                    sequence_number: 0
                };
                tx.unbounded_send(Message::binary(append.to_bytes())).unwrap();
            }
        }
    }
}

async fn foo(websocket: WebSocketStream<MaybeTlsStream<TcpStream>>) {
    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    let (write, read) = websocket.split();

    let stdin_to_ws = stdin_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            let server_reply = comms::ServerReply::try_from_bytes(&data)
                .expect("failed to decode server reply");
            tokio::io::stdout()
                .write_all(format!("{:?}\n", server_reply).as_bytes())
                .await
                .unwrap();
            tokio::io::stdout().flush().await.unwrap();
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}
