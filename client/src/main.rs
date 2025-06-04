use anyhow::Result;
use protocol::{
    Bounded, VarInt,
    clientbound::login::encryption_request::CEncryptionRequest,
    packet_id::CURRENT_MC_PROTOCOL,
    packet_io::PacketIo,
    serverbound::handshake::intention::{HandshakeNextState, SIntention},
};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    connect("0.0.0.0", 25565).await?;

    Ok(())
}

async fn connect(addr: &str, port: u16) -> Result<()> {
    let mut io = PacketIo::new(TcpStream::connect((addr, port)).await?);

    io.send_packet(&SIntention {
        protocol_version: VarInt(CURRENT_MC_PROTOCOL as i32),
        server_address: Bounded(addr),
        server_port: port,
        next_state: HandshakeNextState::Login,
    })
    .await?;

    let next = io.recv_packet::<CEncryptionRequest>().await?;

    Ok(())
}
