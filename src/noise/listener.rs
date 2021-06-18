use crate::config::Config;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};

pub struct NoiseListener {
    config: &'static Config,
    listener: TcpListener,
}

pub struct NoiseHandshake {
    config: &'static Config,
    stream: TcpStream,
    state: snow::HandshakeState,
}

impl NoiseListener {
    pub async fn accept(&mut self) -> std::io::Result<NoiseHandshake> {
        let client = self.listener.accept().await?.0;
        Ok(NoiseHandshake {
            config: self.config,
            stream: client,
            state: snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
                .local_private_key(&self.config.keypair.private[..])
                .build_responder()
                .unwrap(),
        })
    }
}

impl NoiseHandshake {
    /* pub async fn shake(mut self) -> Result<NoiseStream, Box<dyn Error>> {
        let mut s_buf = [0u8; 65535];
        let mut n_buf = [0u8; 65535];
        while !self.state.is_handshake_finished() {
            let len = self.stream.read(&mut s_buf[..]).await?;
            let len = self.state.read_message(&s_buf[..len], &mut n_buf[..])?;
            let len = self.state.write_message(&n_buf[..len], &mut s_buf[..])?;
            self.stream.write_all(&s_buf[..len]).await?;
        }
        todo!()
    } */
}
