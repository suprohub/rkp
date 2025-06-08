use std::sync::Arc;

use anyhow::Result;
use rsa::{RsaPrivateKey, rand_core::OsRng, traits::PublicKeyParts};
use tokio::net::TcpListener;

use crate::{connection::Connection, ping::ServerListPing};

pub struct Server {
    pub private_key: RsaPrivateKey,
    pub public_key: Box<[u8]>,
    pub server_list_ping: ServerListPing,
}

impl Server {
    pub fn new() -> Result<Self> {
        let private_key = RsaPrivateKey::new(&mut OsRng, 1024)?;
        let public_key = rsa_der::public_key_to_der(
            &private_key.n().to_bytes_be(),
            &private_key.e().to_bytes_be(),
        )
        .into_boxed_slice();

        Ok(Self {
            private_key,
            public_key,
            server_list_ping: ServerListPing::default(),
        })
    }

    pub async fn start(self: Arc<Self>, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;

        log::info!("Server started on {addr}");

        while let Ok((stream, remote_addr)) = listener.accept().await {
            let server = self.clone();

            tokio::spawn(async move {
                Connection::new(stream, remote_addr, server)
                    .unwrap()
                    .handle()
                    .await
                    .unwrap();
            });
        }

        Ok(())
    }
}
