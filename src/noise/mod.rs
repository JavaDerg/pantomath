pub mod channel;
pub mod framed;
mod handshake;
pub mod listener;

pub use listener::NoiseListener;

use crate::error::StreamError;
use crate::noise::channel::{ChannelId, Control, FailureResolution, IntNoiseChannel};
use crate::noise::framed::{
    extract_len, Frame16TcpStream, MAX_PAYLOAD_LEN, NOISE_FRAME_MAX_LEN, NOISE_TAG_LEN,
};
use crate::noise::handshake::NoiseHandshake;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use flume::TryRecvError;
use snow::TransportState;
use sodiumoxide::crypto::box_::SecretKey;
use std::mem::{forget, swap, ManuallyDrop, MaybeUninit};
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::sync::Mutex;

pub struct NoiseStream(MaybeOwned<InnerNoiseStream>);

struct InnerNoiseStream {
    stream: Frame16TcpStream,
    noise: TransportState,
    // channel id 255 is reserved for protocol messages only
    channels: [Option<IntNoiseChannel>; 0xFF],
}

enum MaybeOwned<T> {
    Owned(T),
    Shared(Arc<Mutex<T>>),
    Dead,
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

    /// Sends a encrypted message to the other peer on the specified channel
    /// Use `NoiseChannel` over this function
    pub async fn send(
        &mut self,
        m: &impl prost::Message,
        id: ChannelId,
    ) -> Result<(), StreamError> {
        match &mut self.0 {
            MaybeOwned::Owned(inner) => inner.send(m, id).await,
            MaybeOwned::Shared(mutex) => mutex.lock().await.send(m, id).await,
        }
    }

    async fn send_raw(&mut self, payload: &[u8]) -> Result<(), StreamError> {
        match &mut self.0 {
            MaybeOwned::Owned(inner) => inner.send_raw(payload).await,
            MaybeOwned::Shared(mutex) => mutex.lock().await.send_raw(payload).await,
        }
    }

    /// Receives a encrypted message from the other peer on the specified channel and decodes it into `M` where `M: prost::Message + Default`,
    /// Use `NoiseChannel` over this function
    pub async fn recv<M: prost::Message + Default>(
        &mut self,
        id: ChannelId,
    ) -> Result<Option<M>, StreamError> {
        match &mut self.0 {
            MaybeOwned::Owned(inner) => inner.recv(id).await,
            MaybeOwned::Shared(mutex) => mutex.lock().await.recv(id).await,
        }
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
        if let Some(ch) = &self.channels[id.0 as usize] {
            match ch.receiver.try_recv() {
                Ok(Control::Message(msg)) => {
                    return M::decode(msg.clone())
                        .map(|m| Some(m))
                        .map_err(|err| StreamError::DecodeError(err, msg))
                }
                Ok(Control::Eof) => return Ok(None),
                Ok(Control::Failure(err, res)) => {
                    match res {
                        FailureResolution::Ignore => (),
                        FailureResolution::CloseChannel => self.send_raw(&[id.0, 0][..]).await?,
                        FailureResolution::CloseConnection =>
                    }
                    return Err(err)?;
                }
                Err(TryRecvError::Disconnected) => {
                    // Found dead channel but received packet, sending heads up to peer
                    self.send_raw(&[id.0, 0][..]).await?;
                }
                _ => (),
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

            if id.0 != r_id {
                let dl_ch = match &self.channels[r_id as usize] {
                    Some(ch) => {
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
                    self.channels[r_id as usize] = None;
                }
                // recursion would be nice, not having stack overflows too :c
                continue;
            }

            break M::decode(buf)
                .map(Some)
                .map_err(|err| StreamError::DecodeError(err, fail_buf));
        }
    }
}

impl<T> MaybeOwned<T> {
    pub fn make_shared(&mut self) {
        // We later write a _valid_ instance here but one that isn't safe to access.
        let mut slot: ManuallyDrop<Self>;
        match self {
            Self::Shared(_) => return,
            Self::Owned(that) => {
                let mut mutex: Arc<Mutex<MaybeUninit<T>>> =
                    Arc::new(Mutex::new(MaybeUninit::uninit()));
                let inner = Arc::get_mut(&mut mutex).unwrap().get_mut();
                // Prepare the logical move, initializing the inner of mutex in the process.
                unsafe { core::ptr::copy(that, inner as *mut _ as *mut T, 1) };
                // This is slightly iffy because we allow Arc to access the instance. It is valid, but not yet safe to do so.
                // However, Arc doesn't actually access it or dereference the pointer in any way.
                let instance =
                    Self::Shared(unsafe { Arc::from_raw(Arc::into_raw(mutex) as *mut Mutex<T>) });
                slot = ManuallyDrop::new(instance);
            }
        }
        // Important: nothing here panics so we don't leak anything inside the manually drop.
        // Promote the valid but unsafe instance to the active one.
        // This completes the 'move' of inner into the arc of `slot`.
        core::mem::swap(self, &mut *slot);
    }
}
