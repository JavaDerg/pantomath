pub mod framed;
mod handshake;
pub mod listener;

pub use listener::NoiseListener;

use crate::noise::framed::{extract_len, Frame16TcpStream, NOISE_FRAME_MAX_LEN, NOISE_TAG_LEN};
use crate::noise::handshake::NoiseHandshake;
use bytes::{Buf, BufMut, BytesMut};
use snow::TransportState;
use sodiumoxide::crypto::box_::SecretKey;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct NoiseStream {
    stream: Frame16TcpStream,
    noise: TransportState,
}

impl NoiseStream {
    pub async fn connect<A: ToSocketAddrs>(
        addr: A,
        private_key: Option<&SecretKey>,
    ) -> std::io::Result<NoiseHandshake> {
        Ok(NoiseHandshake::new(
            TcpStream::connect(addr).await?,
            private_key,
        ))
    }

    pub async fn send(&mut self, m: &impl prost::Message) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::with_capacity(m.encoded_len());
        m.encode_length_delimited(&mut buf)?;
        let buf = buf.freeze();

        let mut enc_buf = [0u8; NOISE_FRAME_MAX_LEN];
        for chunk in buf.chunks(NOISE_FRAME_MAX_LEN - NOISE_TAG_LEN) {
            let size = self.noise.write_message(chunk, &mut enc_buf[..])?;
            // `Frame16TcpStream` writes all data or errors
            let _ = self.stream.write(&enc_buf[..size]).await?;
        }

        Ok(())
    }

    pub async fn recv<M: prost::Message + Default>(&mut self) -> Result<M, Box<dyn Error>> {
        let mut payload = [0u8; NOISE_FRAME_MAX_LEN - NOISE_TAG_LEN];
        let mut enc_buf = [0u8; NOISE_FRAME_MAX_LEN];

        let mut buf = BytesMut::new();

        let mut explen = None;
        let (header_len, payload_len) = loop {
            let read = self.stream.read(&mut enc_buf[..]).await?;
            let len = self
                .noise
                .read_message(&enc_buf[..read], &mut payload[..])?;
            buf.put_slice(&payload[..len]);

            if explen.is_none() {
                let el = extract_len(buf.chunk(), 10)?;
                if el.0 == 0 {
                    continue;
                }
                let tot = el.0 + el.1;
                if tot > buf.len() {
                    buf.reserve(tot - buf.len());
                }
                explen = Some(el);
            }
            let el = explen.unwrap();
            if el.0 != 0 && el.0 + el.1 <= buf.len() {
                break explen.take().unwrap();
            }
        };

        let mut buf = buf.freeze();
        buf.advance(header_len);
        if buf.remaining() != payload_len {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData).into());
        }

        M::decode(buf).map_err(|err| err.into())
    }
}
