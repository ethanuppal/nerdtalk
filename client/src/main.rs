use std::{env, sync::Arc, time::Duration};

use comms::Codable;
use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_rustls::rustls as tls;
use tokio_tungstenite::{
    connect_async_tls_with_config, tungstenite::protocol::Message, Connector,
};
use webpki::types::{pem::PemObject, CertificateDer};

#[tokio::main]
async fn main() {
    let url = env::args().nth(1).unwrap_or_else(|| {
        panic!("Pass the server's wss:// address as a command-line argument")
    });

    // We either load the local testing root certificates or we use those
    // trusted by Mozilla.
    let mut root_store;
    if cfg!(feature = "local") {
        root_store = tls::RootCertStore::empty();
        for cert in CertificateDer::pem_file_iter("testing_cert/rootCA.crt")
            .expect("Did you remember to run ./gen_cert.sh for local testing?")
        {
            root_store
                .add(cert.expect("failed to load sub file ig"))
                .expect("failed to add local testing certificate");
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
        &url,
        Some(Default::default()),
        false,
        Some(tls_connector),
    )
    .await
    .expect("Failed to connect to server");

    println!("WebSocket handshake has been successfully completed");

    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    let (write, read) = ws_stream.split();

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
