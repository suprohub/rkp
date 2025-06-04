use std::io::Write;

// Used only for macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{Context, Result, anyhow, bail, ensure};

    pub use crate::varint::VarInt;
    pub use crate::{Decode, Encode, Packet};
}

// This allows us to use our own proc macros internally.
extern crate self as protocol;

pub mod packet_id {
    include!(concat!(env!("OUT_DIR"), "/packet_id.rs"));
}

use derive_more::{From, Into};

pub mod bounded;
pub mod clientbound;
pub mod impls;
pub mod serverbound;
pub mod varint;

pub mod decode;
pub mod encode;

pub use bounded::Bounded;
pub use protocol_macros::{Decode, Encode, Packet};
pub use varint::VarInt;

// TODO: make configurable
pub const MAX_PACKET_SIZE: i32 = 2097152;

/// How large a packet should be before it is compressed by the packet encoder.
///
/// If the inner value is >= 0, then packets with encoded lengths >= to this
/// value will be compressed. If the value is negative, then compression is
/// disabled and no packets are compressed.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, From, Into)]
pub struct CompressionThreshold(pub i32);

impl CompressionThreshold {
    /// No compression.
    pub const DEFAULT: Self = Self(-1);
}

/// No compression.
impl Default for CompressionThreshold {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub trait Encode {
    fn encode(&self, w: impl Write) -> anyhow::Result<()>;

    fn encode_slice(slice: &[Self], mut w: impl Write) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        for value in slice {
            value.encode(&mut w)?;
        }

        Ok(())
    }
}

pub trait Decode<'a>: Sized {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self>;
}

/// Types considered to be Minecraft packets.
///
/// In serialized form, a packet begins with a [`VarInt`] packet ID followed by
/// the body of the packet. If present, the implementations of [`Encode`] and
/// [`Decode`] on `Self` are expected to only encode/decode the _body_ of this
/// packet without the leading ID.
pub trait Packet: std::fmt::Debug {
    /// The leading `VarInt` ID of this packet.
    const ID: (i32, ([u8; 8], u32));
    /// The name of this packet for debugging purposes.
    const NAME: &'static str;
    /// The side this packet is intended for.
    const SIDE: PacketSide;
    /// The state in which this packet is used.
    const STATE: PacketState;

    /// Encodes this packet's `VarInt` ID first, followed by the packet's body.
    fn encode_with_id(&self, mut w: impl Write) -> anyhow::Result<()>
    where
        Self: Encode,
    {
        w.write_all(unsafe { Self::ID.1.0.get_unchecked(..Self::ID.1.1 as usize) })?;
        self.encode(w)
    }
}

/// The side a packet is intended for.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PacketSide {
    /// Server -> Client
    Clientbound,
    /// Client -> Server
    Serverbound,
}

/// The statein  which a packet is used.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PacketState {
    Handshake,
    Status,
    Login,
    Config,
    Play,
}
