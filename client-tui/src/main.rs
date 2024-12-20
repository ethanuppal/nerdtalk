use std::{env, sync::Arc, time::Duration};

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

#[tokio::main]
async fn main() {
    let url = env::args().nth(1).unwrap_or_else(|| {
        panic!("Pass the server's wss:// address as a command-line argument")
    });

    let ws_stream = setup_ws().await;
}
