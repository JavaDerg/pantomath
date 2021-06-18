pub mod framed;
mod handshake;
pub mod listener;

use crate::config::Config;
use crate::noise::framed::{extract_len, Frame16TcpStream, NOISE_FRAME_MAX_LEN, NOISE_TAG_LEN};
use crate::noise::handshake::NoiseHandshake;
use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use listener::NoiseListener;
use snow::TransportState;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct NoiseStream {
    stream: Frame16TcpStream,
    noise: TransportState,
    config: &'static Config,
}

impl NoiseStream {
    pub async fn connect<A: ToSocketAddrs>(
        addr: A,
        config: &'static Config,
    ) -> std::io::Result<NoiseHandshake> {
        Ok(NoiseHandshake::new(TcpStream::connect(addr).await?, config))
    }

    pub async fn send<M: prost::Message>(&mut self, m: M) -> Result<(), Box<dyn Error>> {
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
        let mut payload = [0u8; NOISE_FRAME_MAX_LEN];
        let mut enc_buf = [0u8; NOISE_FRAME_MAX_LEN];

        let mut buf = BytesMut::new();

        let (header_len, payload_len) = loop {
            let read = self.stream.read(&mut enc_buf[..]).await?;
            let len = self
                .noise
                .read_message(&enc_buf[..read], &mut payload[..])?;
            buf.put_slice(&payload[..len]);

            let explen = extract_len(buf.chunk())?;
            if explen.0 != 0 && explen.0 + explen.1 <= buf.remaining() {
                break explen;
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
