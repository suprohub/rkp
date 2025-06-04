use std::net::SocketAddr;

use anyhow::Result;
use protocol::{
    clientbound::login::login_disconnect::CLoginDisconnect,
    packet_id::CURRENT_MC_PROTOCOL,
    packet_io::PacketIo,
    serverbound::{
        handshake::intention::{HandshakeNextState, SIntention},
        login::hello::CHello,
    },
};
use tokio::net::{TcpListener, TcpStream};
use valence_text::{Color, IntoText, Text};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    let listener = TcpListener::bind("0.0.0.0:25565").await?;

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(async move {
            handle(stream, addr).await.unwrap();
        });
    }

    Ok(())
}

async fn handle(stream: TcpStream, remote_addr: SocketAddr) -> Result<()> {
    stream.set_nodelay(true)?;
    let mut io = PacketIo::new(stream);

    let SIntention {
        next_state,
        protocol_version,
        ..
    } = io.recv_packet().await?;

    match next_state {
        HandshakeNextState::Status => handle_status(io, remote_addr, protocol_version.0).await?,
        HandshakeNextState::Login => handle_login(io, remote_addr, protocol_version.0).await?,
    }

    Ok(())
}

async fn handle_status(io: PacketIo, remote_addr: SocketAddr, ver: i32) -> Result<()> {
    Ok(())
}

async fn handle_login(mut io: PacketIo, remote_addr: SocketAddr, ver: i32) -> Result<()> {
    // TODO: remove as i32
    if ver != CURRENT_MC_PROTOCOL as i32 {
        io.send_packet(&CLoginDisconnect {
            reason: "кароч новая версия сорри".color(Color::WHITE).into(),
        })
        .await?;
        // TODO: normal errors
        return Ok(());
    }

    let CHello {
        username,
        profile_id,
    } = io.recv_packet().await?;

    Ok(())
}
