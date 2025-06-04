use protocol_macros::{Decode, Encode, Packet};

use crate::PacketState;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login, name = "Key")]
pub struct SEncryptionResponse<'a> {
    pub shared_secret: &'a [u8],
    pub verify_token: &'a [u8],
}
