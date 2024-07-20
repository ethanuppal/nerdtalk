// lowkey stolen from https://github.com/Eugeny/russh/blob/main/russh/examples/client_exec_simple.rs

use anyhow::Result;
use async_trait::async_trait;
use russh::client::{self, KeyboardInteractiveAuthResponse};
use std::{sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(5)),
        ..<_>::default()
    });
    let client = Client;

    let mut session = client::connect(config, common::ADDRESS, client).await?;
    let auth_result = session
        .authenticate_keyboard_interactive_start("ethan", None)
        .await?;
    if matches!(auth_result, KeyboardInteractiveAuthResponse::Failure) {
        anyhow::bail!("Authentication failed");
    }

    let channel = session.channel_open_session().await?.into_stream();

    // loop {
    //     break;
    // }

    session
        .disconnect(
            russh::Disconnect::ByApplication,
            "nerdtalk disconnect",
            "English"
        )
        .await?;

    Ok(())
}

struct Client;

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;
}
