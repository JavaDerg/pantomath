use crate::config::Config;
use crate::noise::framed::{Frame16TcpStream, NOISE_FRAME_MAX_LEN};
use crate::noise::NoiseStream;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct NoiseHandshake {
    config: &'static Config,
    stream: Frame16TcpStream,
    state: snow::HandshakeState,
}

impl NoiseHandshake {
    pub fn new(client: TcpStream, config: &'static Config) -> Self {
        NoiseHandshake {
            config,
            stream: Frame16TcpStream::new(client),
            state: snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
                .local_private_key(&config.keypair.private[..])
                .build_initiator()
                .unwrap(),
        }
    }

    pub(super) fn new_priv(client: TcpStream, config: &'static Config) -> Self {
        NoiseHandshake {
            config,
            stream: Frame16TcpStream::new(client),
            state: snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
                .local_private_key(&config.keypair.private[..])
                .build_responder()
                .unwrap(),
        }
    }

    // FIXME: Use better Error type
    pub async fn shake(self) -> Result<NoiseStream, Box<dyn Error>> {
        self.shake_priv(true).await
    }

    // FIXME: Use better Error type
    pub(super) async fn shake_priv(mut self, client: bool) -> Result<NoiseStream, Box<dyn Error>> {
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

        Ok(NoiseStream {
            stream: self.stream,
            noise: self.state.into_transport_mode()?,
            config: self.config,
        })
    }
}
