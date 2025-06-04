use std::io::{Read, Write};

use anyhow::bail;
use byteorder::ReadBytesExt;
use derive_more::{Deref, DerefMut, From, Into};
use thiserror::Error;

use crate::{Decode, Encode};

/// An `i32` encoded with variable length.
#[derive(
    Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deref, DerefMut, From, Into,
)]
#[repr(transparent)]
pub struct VarInt(pub i32);

impl VarInt {
    /// The maximum number of bytes a `VarInt` could occupy when read from and
    /// written to the Minecraft protocol.
    pub const MAX_SIZE: usize = 5;

    /// Returns the exact number of bytes this varint will write when
    /// [`Encode::encode`] is called, assuming no error occurs.
    pub const fn written_size(self) -> usize {
        match self.0 {
            0 => 1,
            n => (31 - n.leading_zeros() as usize) / 7 + 1,
        }
    }

    pub fn decode_partial<R: Read>(mut r: R) -> Result<i32, VarIntDecodeError> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = r.read_u8().map_err(|_| VarIntDecodeError::Incomplete)?;
            val |= (i32::from(byte) & 0b01111111) << (i * 7);
            if byte & 0b10000000 == 0 {
                return Ok(val);
            }
        }

        Err(VarIntDecodeError::TooLarge)
    }

    pub const fn encode_const(&self) -> ([u8; 8], u32) {
        let x = self.0 as u64;
        let stage1 = (x & 0x000000000000007f)
            | ((x & 0x0000000000003f80) << 1)
            | ((x & 0x00000000001fc000) << 2)
            | ((x & 0x000000000fe00000) << 3)
            | ((x & 0x00000000f0000000) << 4);

        let leading = stage1.leading_zeros();

        let unused_bytes = (leading - 1) >> 3;
        let bytes_needed = 8 - unused_bytes;

        // set all but the last MSBs
        let msbs = 0x8080808080808080;
        let msbmask = 0xffffffffffffffff >> (((8 - bytes_needed + 1) << 3) - 1);

        let merged = stage1 | (msbs & msbmask);
        (merged.to_le_bytes(), bytes_needed)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error)]
pub enum VarIntDecodeError {
    #[error("incomplete VarInt decode")]
    Incomplete,
    #[error("VarInt is too large")]
    TooLarge,
}

impl Encode for VarInt {
    // Adapted from VarInt-Simd encode
    // https://github.com/as-com/varint-simd/blob/0f468783da8e181929b01b9c6e9f741c1fe09825/src/encode/mod.rs#L71
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let (bytes, bytes_needed) = self.encode_const();

        w.write_all(unsafe { bytes.get_unchecked(..bytes_needed as usize) })?;

        Ok(())
    }
}

impl Decode<'_> for VarInt {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = r.read_u8()?;
            val |= (i32::from(byte) & 0b01111111) << (i * 7);
            if byte & 0b10000000 == 0 {
                return Ok(VarInt(val));
            }
        }
        bail!("VarInt is too large")
    }
}
