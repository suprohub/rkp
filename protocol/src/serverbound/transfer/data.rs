use std::net::IpAddr;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Copy, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Config, name = "custom_payload")]
pub struct SData<'a> {
    pub data_type: SDataTypeByte<'a>,
}

#[derive(Clone, Copy, Debug, Encode, Decode)]
pub enum SDataTypeByte<'a> {
    Connect { ip: IpAddr, port: u16, is_udp: bool },
    Process { connection_id: u16, data: &'a [u8] },
    Shutdown { connection_id: u16 },
}
