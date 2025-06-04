use anyhow::Result;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    connect("0.0.0.0", 25565).await?;

    Ok(())
}

async fn connect(addr: &str, port: u16) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;

    Ok(())
}
