use std::{env, fmt::Display, io::Error, sync::Arc};

use chat::ChatLogEntry;
use comms::{AppendChatEntry, Codable};
use futures_util::StreamExt;
use log::info;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_rustls::{
    rustls::{
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        ServerConfig,
    },
    server::TlsStream,
    TlsAcceptor,
};
use tokio_tungstenite::tungstenite::Message;

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
        .map_err(std::io::Error::other)?;

    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8443".to_string());

    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on: {}", addr);

    let (message_tx, message_rx) = mpsc::unbounded_channel();

    // TODO: thread pool
    while let Ok((tcp_stream, _)) = listener.accept().await {
        let tls_acceptor = tls_acceptor.clone();

        let addr = tcp_stream.peer_addr()?;

        let message_tx = message_tx.clone();

        tokio::spawn(async move {
            match tls_acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => {
                    if let Err(e) =
                        handle_tls_connection(tls_stream, addr, message_tx)
                            .await
                    {
                        eprintln!("Error handling TLS connection: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("TLS handshake failed: {}", e);
                }
            }
        });
    }

    Ok(())
}

// TODO: return the write and manage a websocket thread pool
async fn handle_tls_connection<D: Display>(
    tls_stream: TlsStream<TcpStream>,
    addr: D,
    message_tx: mpsc::UnboundedSender<AppendChatEntry>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert TLS stream to WebSocket
    let ws_stream = tokio_tungstenite::accept_async(tls_stream).await?;

    println!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();

    read.map(|message| match message {
        Ok(Message::Binary(data)) => {
            let client_request = comms::ClientMessage::try_from_bytes(&data)
                .expect("failed to decode client request");
            println!("server got message: {:?}", client_request);
            // let server_reply = match client_request {
            //     comms::ClientMessage::Append(append) => message_tx.send(append),
            //     comms::ClientMessage::Request {
            //         count,
            //         up_to_slot_number,
            //     } => todo!(),
            // };
            let server_reply = comms::ServerMessage::NewEntry(
                ChatLogEntry::new_timestamped_now(
                    0,
                    "test".into(),
                    "test".into(),
                ),
            );
            Ok(Message::binary(server_reply.to_bytes()))
        }
        Ok(Message::Close(close_frame)) => {
            println!("server closing connecting with {}", addr);
            Ok(Message::Close(close_frame))
        }
        Err(e) => panic!("server got error: {:?}", e),
        other => panic!("server got non binary data: {:?}", other),
    })
    .forward(write)
    .await
    .or_else(|error| match error {
        // this is probably a bad idea, but there's no other way to do it after a forward; the
        // stream gets closed when the server sends a Close and then I assume forward tries to
        // poll again and it fails, causing this error
        // the TODO is to not use forward and handle the forwarding logic ourselves
        tokio_tungstenite::tungstenite::Error::AlreadyClosed => Ok(()),
        other_error => Err(other_error),
    })?;

    Ok(())
}
