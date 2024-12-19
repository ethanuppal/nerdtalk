use futures_util::{future, StreamExt, TryStreamExt};
use log::info;
use std::{env, fmt::Display, io::Error, sync::Arc};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{
    rustls::{
        crypto::CryptoProvider,
        pki_types::{
            pem::PemObject, CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer,
        },
        RootCertStore, ServerConfig,
    },
    server::TlsStream,
    TlsAcceptor,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();

    let local_testing_certificate =
        CertificateDer::from_pem_slice(include_bytes!("../../cert/cert.pem"))
            .expect("failed to load local testing certificate");

    let local_testing_key = PrivateKeyDer::from_pem_slice(include_bytes!(
            "../../cert/server.key.pem"
        ))
        .expect("failed to load local testing key");

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![local_testing_certificate], local_testing_key)
        .map_err(std::io::Error::other)?;

    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    // Bind to address
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8443".to_string());

    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on: {}", addr);

    while let Ok((tcp_stream, _)) = listener.accept().await {
        let tls_acceptor = tls_acceptor.clone();

        let addr = tcp_stream.peer_addr()?;

        // Spawn a task to handle the TLS connection
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
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary())).map(|msg| {
        info!("Received a message: {:?}", msg);
        msg
    })
        .forward(write)
        .await?;

    Ok(())
}

// // Helper functions to load certificates (you'll need to implement these)
// fn load_cert_chain() -> Result<CertifiedKey, Box<dyn std::error::Error>> {
//     // Load your certificate chain and private key
//     // This is a placeholder - you'll need to provide real certificate
// loading     // logic
//     unimplemented!("Implement certificate chain loading")
// }
//
// fn load_private_key() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
//     // Load your private key
//     // This is a placeholder - you'll need to provide real private key
// loading     // logic
//     unimplemented!("Implement private key loading")
// }
