pub mod channel;
pub mod framed;
mod handshake;
pub mod listener;

pub use listener::NoiseListener;

use crate::error::StreamError;
use crate::noise::channel::{ChannelId, Control, FailureResolution, IntNoiseChannel, NoiseChannel};
use crate::noise::framed::{extract_len, Frame16TcpStream, MAX_PAYLOAD_LEN, NOISE_FRAME_MAX_LEN};
use crate::noise::handshake::NoiseHandshake;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use flume::TryRecvError;
use prost::Message;
use snow::TransportState;
use sodiumoxide::crypto::box_::SecretKey;
use std::mem::swap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::sync::Mutex;

pub struct NoiseStream(MaybeShared<InnerNoiseStream>);

struct InnerNoiseStream {
    stream: Frame16TcpStream,
    noise: TransportState,
    // channel id 255 is reserved for protocol messages only
    channels: [Soon<IntNoiseChannel, IntNoiseChannel>; 0xFF],
}

enum MaybeShared<T> {
    Owned(T),
    Shared(Arc<Mutex<Option<T>>>),
    Dead,
}

enum Soon<N, S> {
    Now(N),
    Soon(S),
    None,
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

    pub async fn make_channel(&mut self) -> Result<NoiseChannel, StreamError> {
        if matches!(self.0, MaybeShared::Dead) {
            return Err(StreamError::AlreadyClosed);
        }
        self.0.make_shared();
        let sch = match &self.0 {
            MaybeShared::Shared(sch) => sch,
            _ => unreachable!(),
        };

        todo!()
    }

    /// Sends a encrypted message to the other peer on the specified channel
    /// Use `NoiseChannel` over this function
    pub async fn send(
        &mut self,
        m: &impl prost::Message,
        id: ChannelId,
    ) -> Result<(), StreamError> {
        match &mut self.0 {
            MaybeShared::Owned(inner) => return inner.send(m, id).await,
            MaybeShared::Shared(mutex) => {
                if let Some(inner) = mutex.lock().await.as_mut() {
                    return inner.send(m, id).await;
                }
            }
            MaybeShared::Dead => return Err(StreamError::AlreadyClosed),
        }

        self.0 = MaybeShared::Dead;
        Err(StreamError::AlreadyClosed)
    }

    /// Receives a encrypted message from the other peer on the specified channel and decodes it into `M` where `M: prost::Message + Default`,
    /// Use `NoiseChannel` over this function
    pub async fn recv<M: prost::Message + Default>(
        &mut self,
        id: ChannelId,
    ) -> Result<Option<M>, StreamError> {
        match &mut self.0 {
            MaybeShared::Owned(inner) => return inner.recv(id).await,
            MaybeShared::Shared(mutex) => {
                if let Some(inner) = mutex.lock().await.as_mut() {
                    return inner.recv(id).await;
                }
            }
            MaybeShared::Dead => return Err(StreamError::AlreadyClosed),
        }

        self.0 = MaybeShared::Dead;
        Err(StreamError::AlreadyClosed)
    }
}

impl InnerNoiseStream {
    pub async fn send(
        &mut self,
        m: &impl prost::Message,
        id: ChannelId,
    ) -> Result<(), StreamError> {
        let mut buf = BytesMut::with_capacity(m.encoded_len() + 4);
        buf.put_u8(id.0);
        m.encode_length_delimited(&mut buf)?;
        let buf = buf.freeze();
        self.send_raw(buf.chunk()).await
    }

    async fn send_raw(&mut self, payload: &[u8]) -> Result<(), StreamError> {
        let mut enc_buf = [0u8; NOISE_FRAME_MAX_LEN];
        for chunk in payload.chunks(MAX_PAYLOAD_LEN) {
            let size = self.noise.write_message(chunk, &mut enc_buf[..])?;
            // `Frame16TcpStream` writes all data or errors
            let _ = self.stream.write(&enc_buf[..size]).await?;
        }
        Ok(())
    }

    pub async fn recv<M: prost::Message + Default>(
        &mut self,
        id: ChannelId,
    ) -> Result<Option<M>, StreamError> {
        if let Soon::Now(ch) = &self.channels[id.0 as usize] {
            loop {
                match ch.receiver.try_recv() {
                    Ok(Control::Message(msg)) => {
                        return M::decode(msg.clone())
                            .map(Some)
                            .map_err(|err| StreamError::DecodeError(err, msg))
                    }
                    Ok(Control::Eof) => return Ok(None),
                    Ok(Control::Failure(err, res)) => {
                        match res {
                            FailureResolution::Ignore => continue,
                            FailureResolution::CloseChannel => {
                                self.send_raw(&[id.0, 0][..]).await?;
                                break;
                            }
                            FailureResolution::CloseConnection => {
                                self.die().await?;
                            }
                        }
                        return Err(err.into());
                    }
                    Err(TryRecvError::Disconnected) => {
                        // Found dead channel but received packet, sending heads up to peer
                        self.send_raw(&[id.0, 0][..]).await?;
                        break;
                    }
                    _ => (),
                }
            }
            // TODO
        }

        loop {
            let mut payload = [0u8; MAX_PAYLOAD_LEN];
            let mut enc_buf = [0u8; NOISE_FRAME_MAX_LEN];

            let mut buf = BytesMut::new();

            let mut explen = None;
            let (header_len, payload_len) = loop {
                let read = self.stream.read(&mut enc_buf[..]).await?;
                let len = self
                    .noise
                    .read_message(&enc_buf[..read], &mut payload[..])?;
                buf.put_slice(&payload[..len]);

                if buf.len() <= 1 {
                    continue;
                }

                if explen.is_none() {
                    let el = extract_len(&buf.chunk()[1..], 10)?;
                    if el.0 == 0 {
                        continue;
                    }
                    let tot = el.0 + el.1 + 1;
                    if tot > buf.len() {
                        buf.reserve(tot - buf.len());
                    }
                    explen = Some(el);
                }
                let el = explen.unwrap();
                if el.0 != 0 && el.0 + el.1 <= buf.len() - 1 {
                    break explen.take().unwrap();
                }
            };

            let mut buf = buf.freeze();
            let r_id = buf.get_u8();
            buf.advance(header_len);
            let fail_buf = buf.clone();
            if buf.remaining() != payload_len {
                return Err(std::io::Error::from(std::io::ErrorKind::InvalidData).into());
            }

            if r_id == 0xFF {
                self.process_proto_pack(buf).await?;
                continue;
            }

            if id.0 != r_id {
                let dl_ch = match &self.channels[r_id as usize] {
                    Soon::Now(ch) => {
                        match ch.sender.send(buf) {
                            Ok(()) => false,
                            // channel is closed, notify peer by sending empty packet
                            Err(_) => {
                                self.send_raw(&[r_id, 0][..]).await?;
                                true
                            }
                        }
                    }
                    _ => false,
                };

                if dl_ch {
                    self.channels[r_id as usize] = Soon::None;
                }
                // recursion would be nice, not having stack overflows too :c
                continue;
            }

            break M::decode(buf)
                .map(Some)
                .map_err(|err| StreamError::DecodeError(err, fail_buf));
        }
    }

    async fn die(&mut self) -> Result<(), StreamError> {
        // Drop all channel senders/receivers
        self.channels.iter_mut().for_each(|soon| *soon = Soon::None);
        self.stream.shutdown().await?;
        Ok(())
    }

    async fn process_proto_pack(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        use crate::proto::protocol::{
            make_channel_response::Kind as McrKind, protocol_packet::Kind as PKind,
            MakeChannelResponse, ProtocolPacket,
        };

        let packet = ProtocolPacket::decode(bytes.clone())
            .map_err(|err| StreamError::DecodeError(err, bytes))?;
        match packet.kind.ok_or(StreamError::InvalidPacket)? {
            PKind::MakeChannel(mkch) => {
                let id = mkch.id;
                if id >= 0xFF {
                    return Err(StreamError::InvalidPacket);
                }
                let id = id as usize;
                if !matches!(&self.channels[id], Soon::None) {
                    let open = self
                        .channels
                        .iter()
                        .zip(0u8..)
                        .filter(|(soon, _)| matches!(*soon, Soon::None))
                        .map(|(_, index)| index)
                        .collect::<Vec<u8>>();
                    self.send(
                        &MakeChannelResponse {
                            id: id as u32,
                            kind: McrKind::Fail as i32,
                            open,
                        },
                        ChannelId(0xFF),
                    )
                    .await?;
                    return Ok(());
                }
            }
            PKind::MakeChannelResponse(mcrsp) => {
                todo!()
            }
        }
        Ok(())
    }

    fn find_new_channel(&self) -> Option<u8> {
        self.channels
            .iter()
            .zip(0u8..)
            .filter(|(soon, _)| matches!(*soon, Soon::None))
            .map(|(_, index)| index)
            .next()
    }
}

impl<T> MaybeShared<T> {
    pub fn make_shared(&mut self) {
        if matches!(self, Self::Dead | Self::Shared(_)) {
            return;
        }
        let mut this = Self::Dead;
        swap(self, &mut this);
        let t = match this {
            MaybeShared::Owned(t) => t,
            _ => unreachable!(),
        };
        this = Self::Shared(Arc::new(Mutex::new(Some(t))));
        swap(self, &mut this);
    }
}
