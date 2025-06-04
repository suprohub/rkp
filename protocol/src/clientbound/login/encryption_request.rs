use protocol_macros::{Decode, Encode, Packet};

use crate::{Bounded, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login, name = "Hello")]
pub struct CEncryptionRequest<'a> {
    // Always empty on vanilla servers
    pub server_id: Bounded<&'a str, 20>,
    pub public_key: &'a [u8],
    pub verify_token: &'a [u8],
    pub should_verify: bool,
}
