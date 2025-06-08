use std::{net::SocketAddr, sync::Arc};

use anyhow::{Result, ensure};
use protocol::{
    Bounded,
    clientbound::{
        login::{
            encryption_request::CEncryptionRequest, login_disconnect::CLoginDisconnect,
            login_success::CLoginFinished,
        },
        status::{ping_response::CPongResponse, status_response::CStatusResponse},
    },
    packet_id::CURRENT_MC_PROTOCOL,
    packet_io::PacketIo,
    serverbound::{
        handshake::intention::{HandshakeNextState, SIntention},
        login::{
            encryption_response::SEncryptionResponse, hello::SHello,
            login_acknowledged::SLoginAcknowledged,
        },
        status::{ping_request::SPingRequest, status_request::SStatusRequest},
    },
};
use rsa::Pkcs1v15Encrypt;
use tokio::net::TcpStream;
use valence_text::{Color, IntoText};

use crate::server::Server;

pub struct Connection {
    io: PacketIo,
    remote_addr: SocketAddr,
    server: Arc<Server>,
}

impl Connection {
    pub fn new(stream: TcpStream, remote_addr: SocketAddr, server: Arc<Server>) -> Result<Self> {
        stream.set_nodelay(true)?;

        Ok(Self {
            io: PacketIo::new(stream),
            remote_addr,
            server,
        })
    }

    pub async fn handle(mut self) -> Result<()> {
        let SIntention {
            next_state,
            protocol_version,
            ..
        } = self.io.recv_packet().await?;

        match next_state {
            HandshakeNextState::Status => self.handle_status(protocol_version.0).await?,
            HandshakeNextState::Login => {
                self.handle_login(protocol_version.0).await?;
            }
        }

        Ok(())
    }

    async fn handle_status(mut self, ver: i32) -> Result<()> {
        self.io.recv_packet::<SStatusRequest>().await?;
        self.io
            .send_packet(&CStatusResponse {
                json: &serde_json::to_string(&self.server.server_list_ping)?,
            })
            .await?;

        let SPingRequest { payload } = self.io.recv_packet().await?;
        self.io.send_packet(&CPongResponse { payload }).await?;

        log::info!("Accepted status from {}", self.remote_addr);
        Ok(())
    }

    async fn handle_login(&mut self, ver: i32) -> Result<()> {
        // TODO: remove as i32
        if ver != CURRENT_MC_PROTOCOL as i32 {
            self.io
                .send_packet(&CLoginDisconnect {
                    reason: "кароч новая версия сори".color(Color::WHITE).into(),
                })
                .await?;
            // TODO: normal errors
            return Ok(());
        }

        let SHello { username, uuid } = self.io.recv_packet().await?;

        let username = username.to_string();

        self.encrypt_connection().await?;

        self.io
            .send_packet(&CLoginFinished {
                uuid,
                username: Bounded(&username),
            })
            .await?;

        self.io.recv_packet::<SLoginAcknowledged>().await?;

        log::info!("Accepted login from {}", self.remote_addr);

        Ok(())
    }

    async fn encrypt_connection(&mut self) -> Result<()> {
        let server_verify_token: [u8; 16] = rand::random();

        self.io
            .send_packet(&CEncryptionRequest {
                server_id: Bounded::default(),
                public_key: &self.server.public_key,
                verify_token: &server_verify_token,
                // Disabling mojang auth for anonymous connections
                should_verify: false,
            })
            .await?;

        let SEncryptionResponse {
            shared_secret,
            verify_token: encrypted_verify_token,
        } = self.io.recv_packet().await?;

        let shared_secret = self
            .server
            .private_key
            .decrypt(Pkcs1v15Encrypt, shared_secret)?;

        let verify_token = self
            .server
            .private_key
            .decrypt(Pkcs1v15Encrypt, encrypted_verify_token)?;

        ensure!(
            server_verify_token.as_slice() == verify_token,
            "verify tokens do not match"
        );

        let key: [u8; 16] = shared_secret.as_slice().try_into()?;

        self.io.enable_encryption(&key);

        log::info!("Base encryption enabled on {}", self.remote_addr);

        Ok(())
    }
}
