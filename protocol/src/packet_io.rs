use std::io::{self, ErrorKind};

use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    CompressionThreshold, Decode, Encode, Packet,
    decode::{PacketDecoder, PacketFrame},
    encode::PacketEncoder,
};

pub struct PacketIo {
    stream: TcpStream,
    enc: PacketEncoder,
    dec: PacketDecoder,
    frame: PacketFrame,
}

const READ_BUF_SIZE: usize = 4096;

impl PacketIo {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            enc: PacketEncoder::new(),
            dec: PacketDecoder::new(),
            frame: PacketFrame {
                id: -1,
                body: BytesMut::new(),
            },
        }
    }

    pub async fn send_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.enc.append_packet(pkt)?;
        let bytes = self.enc.take();
        self.stream.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn recv_packet<'a, P>(&'a mut self) -> anyhow::Result<P>
    where
        P: Packet + Decode<'a>,
    {
        loop {
            if let Some(frame) = self.dec.try_next_packet()? {
                self.frame = frame;

                return self.frame.decode();
            }

            self.dec.reserve(READ_BUF_SIZE);
            let mut buf = self.dec.take_capacity();

            if self.stream.read_buf(&mut buf).await? == 0 {
                return Err(io::Error::from(ErrorKind::UnexpectedEof).into());
            }

            // This should always be an O(1) unsplit because we reserved space earlier and
            // the call to `read_buf` shouldn't have grown the allocation.
            self.dec.queue_bytes(buf);
        }
    }

    pub fn set_compression(&mut self, threshold: CompressionThreshold) {
        self.enc.set_compression(threshold);
        self.dec.set_compression(threshold);
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.enc.enable_encryption(key);
        self.dec.enable_encryption(key);
    }
}
