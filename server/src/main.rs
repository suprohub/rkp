use crate::server::Server;
use anyhow::Result;
use std::sync::Arc;

pub mod connection;
pub mod ping;
pub mod server;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init()?;

    Arc::new(Server::new()?).start("0.0.0.0:25565").await?;

    Ok(())
}
