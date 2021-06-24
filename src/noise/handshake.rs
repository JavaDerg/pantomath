use crate::error::StreamError;
use crate::noise::channel::IntNoiseChannel;
use crate::noise::framed::{Frame16TcpStream, NOISE_FRAME_MAX_LEN};
use crate::noise::{InnerNoiseStream, MaybeShared, NoiseStream, Soon};
use sodiumoxide::crypto::box_::SecretKey;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const ICH_NONE: Soon<IntNoiseChannel, IntNoiseChannel> = Soon::None;

pub struct NoiseHandshake {
    stream: Frame16TcpStream,
    state: snow::HandshakeState,
}

impl NoiseHandshake {
    pub fn new(client: TcpStream, private_key: Option<&SecretKey>) -> Self {
        let mut builder = snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap());
        if let Some(pk) = private_key {
            builder = builder.local_private_key(&pk.0[..]);
        }
        NoiseHandshake {
            stream: Frame16TcpStream::new(client),
            state: builder.build_initiator().unwrap(),
        }
    }

    pub(super) fn new_priv(client: TcpStream, pk: &SecretKey) -> Self {
        NoiseHandshake {
            stream: Frame16TcpStream::new(client),
            state: snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
                .local_private_key(&pk.0[..])
                .build_responder()
                .unwrap(),
        }
    }

    pub async fn shake(self) -> Result<NoiseStream, StreamError> {
        self.shake_priv(true).await
    }

    pub(super) async fn shake_priv(mut self, client: bool) -> Result<NoiseStream, StreamError> {
        let mut s_buf = [0u8; NOISE_FRAME_MAX_LEN];
        let mut n_buf = [0u8; NOISE_FRAME_MAX_LEN];
        if client {
            let len = self.state.write_message(&[], &mut s_buf[..])?;
            self.stream.write(&s_buf[..len]).await?;
        }
        while !self.state.is_handshake_finished() {
            let len = self.stream.read(&mut s_buf[..]).await?;
            let len = self.state.read_message(&s_buf[..len], &mut n_buf[..])?;
            if self.state.is_handshake_finished() {
                break;
            }
            let len = self.state.write_message(&n_buf[..len], &mut s_buf[..])?;
            self.stream.write(&s_buf[..len]).await?;
        }

        Ok(NoiseStream(MaybeShared::Owned(InnerNoiseStream {
            stream: self.stream,
            noise: self.state.into_transport_mode()?,
            channels: [ICH_NONE; 0xFF],
        })))
    }
}
