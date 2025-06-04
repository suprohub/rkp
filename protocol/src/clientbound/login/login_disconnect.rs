use std::borrow::Cow;

use protocol_macros::{Decode, Encode, Packet};
use valence_text::Text;

use crate::PacketState;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
pub struct CLoginDisconnect<'a> {
    pub reason: Cow<'a, Text>,
}
