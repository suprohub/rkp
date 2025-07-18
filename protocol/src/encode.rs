use std::io::Write;

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncryptMut, BlockSizeUser, KeyIvInit};
use anyhow::ensure;
use bytes::{BufMut, BytesMut};

use crate::varint::VarInt;
use crate::{CompressionThreshold, Encode, MAX_PACKET_SIZE, Packet};

/// The AES block cipher with a 128 bit key, using the CFB-8 mode of
/// operation.
type Cipher = cfb8::Encryptor<aes::Aes128>;

#[derive(Default)]
pub struct PacketEncoder {
    buf: BytesMut,
    compress_buf: Vec<u8>,
    threshold: CompressionThreshold,
    cipher: Option<Cipher>,
}

impl PacketEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn append_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes)
    }

    pub fn prepend_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        let start_len = self.buf.len();
        self.append_packet(pkt)?;

        let end_len = self.buf.len();
        let total_packet_len = end_len - start_len;

        // 1) Move everything back by the length of the packet.
        // 2) Move the packet to the new space at the front.
        // 3) Truncate the old packet away.
        self.buf.put_bytes(0, total_packet_len);
        self.buf.copy_within(..end_len, total_packet_len);
        self.buf.copy_within(total_packet_len + start_len.., 0);
        self.buf.truncate(end_len);

        Ok(())
    }

    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn append_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        let start_len = self.buf.len();

        pkt.encode_with_id((&mut self.buf).writer())?;

        let data_len = self.buf.len() - start_len;

        if self.threshold.0 >= 0 {
            use std::io::Read;

            use flate2::Compression;
            use flate2::bufread::ZlibEncoder;

            if data_len > self.threshold.0 as usize {
                let mut z = ZlibEncoder::new(&self.buf[start_len..], Compression::new(4));

                self.compress_buf.clear();

                let data_len_size = VarInt(data_len as i32).written_size();

                let packet_len = data_len_size + z.read_to_end(&mut self.compress_buf)?;

                ensure!(
                    packet_len <= MAX_PACKET_SIZE as usize,
                    "packet exceeds maximum length"
                );

                drop(z);

                self.buf.truncate(start_len);

                let mut writer = (&mut self.buf).writer();

                VarInt(packet_len as i32).encode(&mut writer)?;
                VarInt(data_len as i32).encode(&mut writer)?;
                self.buf.extend_from_slice(&self.compress_buf);
            } else {
                let data_len_size = 1;
                let packet_len = data_len_size + data_len;

                ensure!(
                    packet_len <= MAX_PACKET_SIZE as usize,
                    "packet exceeds maximum length"
                );

                let packet_len_size = VarInt(packet_len as i32).written_size();

                let data_prefix_len = packet_len_size + data_len_size;

                self.buf.put_bytes(0, data_prefix_len);
                self.buf
                    .copy_within(start_len..start_len + data_len, start_len + data_prefix_len);

                let mut front = &mut self.buf[start_len..];

                VarInt(packet_len as i32).encode(&mut front)?;
                // Zero for no compression on this packet.
                VarInt(0).encode(front)?;
            }

            return Ok(());
        }

        let packet_len = data_len;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        let packet_len_size = VarInt(packet_len as i32).written_size();

        self.buf.put_bytes(0, packet_len_size);
        self.buf
            .copy_within(start_len..start_len + data_len, start_len + packet_len_size);

        let front = &mut self.buf[start_len..];
        VarInt(packet_len as i32).encode(front)?;

        Ok(())
    }

    /// Takes all the packets written so far and encrypts them if encryption is
    /// enabled.
    pub fn take(&mut self) -> BytesMut {
        if let Some(cipher) = &mut self.cipher {
            for chunk in self.buf.chunks_mut(Cipher::block_size()) {
                let gen_arr = GenericArray::from_mut_slice(chunk);
                cipher.encrypt_block_mut(gen_arr);
            }
        }

        self.buf.split()
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn set_compression(&mut self, threshold: CompressionThreshold) {
        self.threshold = threshold;
    }

    /// Initializes the cipher with the given key. All future packets **and any
    /// that have not been [taken] yet** are encrypted.
    ///
    /// [taken]: Self::take
    ///
    /// # Panics
    ///
    /// Panics if encryption is already enabled.

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        assert!(self.cipher.is_none(), "encryption is already enabled");
        self.cipher = Some(Cipher::new_from_slices(key, key).expect("invalid key"));
    }
}

/// Types that can have packets written to them.
pub trait WritePacket {
    /// Writes a packet to this object. Encoding errors are typically logged and
    /// discarded.
    fn write_packet<P>(&mut self, packet: &P)
    where
        P: Packet + Encode,
    {
        if let Err(e) = self.write_packet_fallible(packet) {
            log::warn!("failed to write packet '{}': {e:#}", P::NAME);
        }
    }

    /// Writes a packet to this object. The result of encoding the packet is
    /// returned.
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode;

    /// Copies raw packet data directly into this object. Don't use this unless
    /// you know what you're doing.
    fn write_packet_bytes(&mut self, bytes: &[u8]);
}

impl<W: WritePacket> WritePacket for &mut W {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        (*self).write_packet_fallible(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        (*self).write_packet_bytes(bytes)
    }
}

/// An implementor of [`WritePacket`] backed by a `Vec` mutable reference.
///
/// Packets are written by appending to the contained vec. If an error occurs
/// while writing, the written bytes are truncated away.
#[derive(Debug)]
pub struct PacketWriter<'a> {
    pub buf: &'a mut Vec<u8>,
    pub threshold: CompressionThreshold,
}

impl<'a> PacketWriter<'a> {
    pub fn new(buf: &'a mut Vec<u8>, threshold: CompressionThreshold) -> Self {
        Self { buf, threshold }
    }
}

impl WritePacket for PacketWriter<'_> {
    fn write_packet_fallible<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        let start = self.buf.len();

        let res;

        if self.threshold.0 >= 0 {
            res = encode_packet_compressed(self.buf, pkt, self.threshold.0 as u32);
        } else {
            res = encode_packet(self.buf, pkt)
        };

        if res.is_err() {
            self.buf.truncate(start);
        }

        res
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        if let Err(e) = self.buf.write_all(bytes) {
            log::warn!("failed to write packet bytes: {e:#}");
        }
    }
}

impl WritePacket for PacketEncoder {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.append_packet(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.append_bytes(bytes)
    }
}

fn encode_packet<P>(buf: &mut Vec<u8>, pkt: &P) -> anyhow::Result<()>
where
    P: Packet + Encode,
{
    let start_len = buf.len();

    pkt.encode_with_id(&mut *buf)?;

    let packet_len = buf.len() - start_len;

    ensure!(
        packet_len <= MAX_PACKET_SIZE as usize,
        "packet exceeds maximum length"
    );

    let packet_len_size = VarInt(packet_len as i32).written_size();

    buf.put_bytes(0, packet_len_size);
    buf.copy_within(
        start_len..start_len + packet_len,
        start_len + packet_len_size,
    );

    let front = &mut buf[start_len..];
    VarInt(packet_len as i32).encode(front)?;

    Ok(())
}

#[allow(clippy::needless_borrows_for_generic_args)]
fn encode_packet_compressed<P>(buf: &mut Vec<u8>, pkt: &P, threshold: u32) -> anyhow::Result<()>
where
    P: Packet + Encode,
{
    use std::io::Read;

    use flate2::Compression;
    use flate2::bufread::ZlibEncoder;

    let start_len = buf.len();

    pkt.encode_with_id(&mut *buf)?;

    let data_len = buf.len() - start_len;

    if data_len > threshold as usize {
        let mut z = ZlibEncoder::new(&buf[start_len..], Compression::new(4));

        let mut scratch = vec![];

        let packet_len = VarInt(data_len as i32).written_size() + z.read_to_end(&mut scratch)?;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        drop(z);

        buf.truncate(start_len);

        VarInt(packet_len as i32).encode(&mut *buf)?;
        VarInt(data_len as i32).encode(&mut *buf)?;
        buf.extend_from_slice(&scratch);
    } else {
        let data_len_size = 1;
        let packet_len = data_len_size + data_len;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        let packet_len_size = VarInt(packet_len as i32).written_size();

        let data_prefix_len = packet_len_size + data_len_size;

        buf.put_bytes(0, data_prefix_len);
        buf.copy_within(start_len..start_len + data_len, start_len + data_prefix_len);

        let mut front = &mut buf[start_len..];

        VarInt(packet_len as i32).encode(&mut front)?;
        // Zero for no compression on this packet.
        VarInt(0).encode(front)?;
    }

    Ok(())
}
