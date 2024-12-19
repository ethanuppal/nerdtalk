use std::{env, fmt::Display, io::Error, sync::Arc};

use comms::Codable;
use futures_util::{future, StreamExt, TryStreamExt};
use log::info;
use tokio::net::{TcpListener, TcpStream};
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

    // TODO: thread pool
    while let Ok((tcp_stream, _)) = listener.accept().await {
        let tls_acceptor = tls_acceptor.clone();

        let addr = tcp_stream.peer_addr()?;

        tokio::spawn(async move {
            match tls_acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => {
                    if let Err(e) =
                        handle_tls_connection(tls_stream, addr).await
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

async fn handle_tls_connection<D: Display>(
    tls_stream: TlsStream<TcpStream>,
    addr: D,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert TLS stream to WebSocket
    let ws_stream = tokio_tungstenite::accept_async(tls_stream).await?;

    info!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();

    // Echo messages back
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .map(|msg| {
            if let Ok(Message::Binary(data)) = &msg {
                println!(
                    "Received a message: {:?}",
                    comms::ClientRequest::from_bytes(data)
                );
            }
            msg
        })
        .forward(write)
        .await?;

    Ok(())
}
