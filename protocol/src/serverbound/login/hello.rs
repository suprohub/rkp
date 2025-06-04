use uuid::Uuid;

use crate::{Bounded, Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
pub struct SHello<'a> {
    pub username: Bounded<&'a str, 16>,
    pub uuid: Uuid,
}
