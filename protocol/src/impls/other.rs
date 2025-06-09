use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use anyhow::bail;
use uuid::Uuid;

use crate::{Decode, Encode};

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            Some(t) => {
                true.encode(&mut w)?;
                t.encode(w)
            }
            None => false.encode(w),
        }
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Option<T> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(if bool::decode(r)? {
            Some(T::decode(r)?)
        } else {
            None
        })
    }
}

impl Encode for Uuid {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.as_u128().encode(w)
    }
}

impl<'a> Decode<'a> for Uuid {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        u128::decode(r).map(Uuid::from_u128)
    }
}

impl Encode for Ipv4Addr {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.to_bits().encode(w)
    }
}

impl<'a> Decode<'a> for Ipv4Addr {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        u32::decode(r).map(Ipv4Addr::from_bits)
    }
}

impl Encode for Ipv6Addr {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.to_bits().encode(w)
    }
}

impl<'a> Decode<'a> for Ipv6Addr {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        u128::decode(r).map(Ipv6Addr::from_bits)
    }
}

impl Encode for IpAddr {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            Self::V4(v4) => {
                0u8.encode(&mut w)?;
                v4.encode(w)?;
            }
            Self::V6(v6) => {
                1u8.encode(&mut w)?;
                v6.encode(w)?;
            }
        };
        Ok(())
    }
}

impl<'a> Decode<'a> for IpAddr {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(match u8::decode(r)? {
            0 => Self::V4(Ipv4Addr::decode(r)?),
            1 => Self::V6(Ipv6Addr::decode(r)?),
            _ => bail!("Unknown addr type"),
        })
    }
}
