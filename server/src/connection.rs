use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::{Result, anyhow, bail, ensure};
use protocol::{
    Bounded,
    clientbound::{
        login::{
            encryption_request::CEncryptionRequest, login_disconnect::CLoginDisconnect,
            login_success::CLoginFinished,
        },
        status::{ping_response::CPongResponse, status_response::CStatusResponse},
        transfer::data::{CData, CDataTypeByte},
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
        transfer::{
            client_information::SClientInformation,
            data::{SData, SDataTypeByte},
        },
    },
};
use rsa::Pkcs1v15Encrypt;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UdpSocket},
};
use valence_text::{Color, IntoText};

use crate::server::Server;

pub struct Client {
    io: PacketIo,
    remote_addr: SocketAddr,
    server: Arc<Server>,

    info: Option<SClientInformation>,
    next_connection_id: u16,
    connections: HashMap<u16, Connection>,
}

impl Client {
    pub fn new(stream: TcpStream, remote_addr: SocketAddr, server: Arc<Server>) -> Result<Self> {
        stream.set_nodelay(true)?;

        Ok(Self {
            io: PacketIo::new(stream),
            remote_addr,
            server,

            info: None,
            next_connection_id: 0,
            connections: HashMap::new(),
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
                self.handle_data_transfer().await?;
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
            // TODO: normal errors
            self.io
                .send_packet(&CLoginDisconnect {
                    reason: "кароч новая версия сори".color(Color::WHITE).into(),
                })
                .await?;

            bail!("Client use old version");
        }

        let SHello { username, uuid } = self.io.recv_packet().await?;
        let private_uuid = match self
            .server
            .login_data
            .get(username.0)
            .copied()
            .ok_or(anyhow!("Username not found"))
        {
            Ok((public_uuid, private_uuid)) if public_uuid == uuid => private_uuid,
            _ => {
                self.io
                    .send_packet(&CLoginDisconnect {
                        reason: "ты не в вайтлисте ъ".color(Color::WHITE).into(),
                    })
                    .await?;

                bail!("Client have incorrect name or uuid");
            }
        };

        let username = username.to_string();

        self.encrypt_connection().await?;

        self.io
            .send_packet(&CLoginFinished {
                uuid,
                username: Bounded(&username),
            })
            .await?;

        // Login acknowledged, but we needed login again
        // Because player name & uuid sends when connection isnt encrypted
        self.io.recv_packet::<SLoginAcknowledged>().await?;

        // So, we need another check
        // And if auth fails, then this is a serious warning sign
        // That the client's traffic is being listened to
        self.info = match self.io.recv_packet::<SClientInformation>().await {
            Ok(info) if info.private_uuid == private_uuid => Some(info),
            _ => {
                self.io
                    .send_packet(&CLoginDisconnect {
                        reason: "ты не в вайтлисте ъ".color(Color::WHITE).into(),
                    })
                    .await?;

                log::warn!(
                    "Connection {} with player name {} failed second login, this can be MITM attack",
                    self.remote_addr,
                    username
                );

                bail!("Client failed second login by uuid");
            }
        };

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

    async fn handle_data_transfer(&mut self) -> Result<()> {
        let udp = UdpSocket::bind("0.0.0.0:123").await?;

        loop {
            match self.io.recv_packet::<SData>().await?.data_type {
                SDataTypeByte::Connect { ip, port, is_udp } => {
                    let connection_id = self.next_connection_id;
                    self.next_connection_id = self.next_connection_id.wrapping_add(1);

                    if is_udp {
                        self.connections
                            .insert(connection_id, Connection::Udp((ip, port)));
                    } else {
                        self.connections.insert(
                            connection_id,
                            Connection::Tcp(TcpStream::connect((ip, port)).await?),
                        );
                    }

                    self.io
                        .send_packet(&CData {
                            data_type: CDataTypeByte::Connect {
                                ip,
                                port,
                                is_udp,
                                connection_id,
                            },
                        })
                        .await?;
                }
                SDataTypeByte::Process {
                    connection_id,
                    data,
                } => {
                    match self
                        .connections
                        .get_mut(&connection_id)
                        .ok_or(anyhow!("Connection not found"))?
                    {
                        Connection::Tcp(stream) => {
                            stream.write_all(data).await?;
                        }
                        Connection::Udp((ip, port)) => {
                            udp.send_to(data, (*ip, *port)).await?;
                        }
                    }
                }
                SDataTypeByte::Shutdown { connection_id } => {
                    if let Connection::Tcp(mut stream) = self
                        .connections
                        .remove(&connection_id)
                        .ok_or(anyhow!("Connection not found"))?
                    {
                        stream.shutdown().await?;
                    }
                }
            }
        }

        Ok(())
    }
}

enum Connection {
    Tcp(TcpStream),
    Udp((IpAddr, u16)),
}
