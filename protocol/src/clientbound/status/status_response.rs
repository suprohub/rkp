use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Status)]
pub struct CStatusResponse<'a> {
    pub json: &'a str,
}
