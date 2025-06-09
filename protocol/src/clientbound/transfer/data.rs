use std::net::IpAddr;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Config, name = "custom_payload")]
pub struct CData<'a> {
    pub data_type: CDataTypeByte<'a>,
}

#[derive(Clone, Copy, Debug, Encode, Decode)]
pub enum CDataTypeByte<'a> {
    Connect {
        ip: IpAddr,
        port: u16,
        is_udp: bool,
        connection_id: u16,
    },
    Process {
        connection_id: u16,
        data: &'a [u8],
    },
}
