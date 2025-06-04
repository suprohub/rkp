use std::io::Write;

use uuid::Uuid;
use crate::{Bounded, Decode, Encode, Packet, PacketState, VarInt};

#[derive(Clone, Debug, Packet)]
#[packet(state = PacketState::Login)]
pub struct CLoginFinished<'a> {
    pub uuid: Uuid,
    pub username: Bounded<&'a str, 16>,
}

impl Encode for CLoginFinished<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.uuid.encode(&mut w)?;
        self.username.encode(&mut w)?;
        let (bytes, bytes_needed) = VarInt(0).encode_const();
        w.write_all(unsafe { bytes.get_unchecked(..bytes_needed as usize) })?;

        Ok(())
    }
}