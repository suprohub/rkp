use std::io::Write;

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
