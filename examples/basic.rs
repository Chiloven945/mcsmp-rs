//! Connect to an MCSMP endpoint, print its current status, and close cleanly.
//!
//! Set `MCSMP_ENDPOINT` and `MCSMP_SECRET` before running:
//!
//! ```text
//! MCSMP_ENDPOINT=wss://127.0.0.1:25585 \
//! MCSMP_SECRET=... \
//! cargo run --example basic
//! ```

use mcsmp_rs::{Auth, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("MCSMP_ENDPOINT")?.parse()?;
    let secret = std::env::var("MCSMP_SECRET")?;

    let client = Client::builder(endpoint)
        .auth(Auth::bearer(secret)?)
        .origin("mcsmp-rs-example")
        .connect()
        .await?;

    let capabilities = client.discover().await?;
    let status = client.server().status().await?;

    println!(
        "MCSMP {:?}; server started: {}; online players: {}",
        capabilities.protocol_version,
        status.started,
        status.online_player_count()
    );

    client.shutdown().await?;
    Ok(())
}
