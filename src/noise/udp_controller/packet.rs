use crate::error::StreamError;
use bytes::{Buf, BufMut, Bytes};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub type Resend = bool;

#[derive(Clone)]
pub struct Packet {
    pub data: Bytes,
    pub nonce: u64,
}

thread_local! {
    static THREAD_RNG: Rc<UnsafeCell<ChaCha8Rng>> = {
        Rc::new(UnsafeCell::new(ChaCha8Rng::from_entropy()))
    }
}

pub fn secure_rnd_64() -> u64 {
    THREAD_RNG.with(|rng| unsafe { rng.get().as_mut() }.unwrap().next_u64())
}

fn encode(
    mut buf: impl BufMut,
    pkt: Packet,
    cipher: &snow::StatelessTransportState,
) -> Result<(), StreamError> {
    buf.put_u64(pkt.nonce);

    let mut buffer = [0u8; 512];
    let len = cipher.write_message(pkt.nonce, pkt.data.chunk(), &mut buffer[..])?;

    buf.put_slice(&buffer[..len]);

    Ok(())
}

fn decode(mut pkt: Bytes, cipher: &snow::StatelessTransportState) -> Result<Packet, Resend> {
    if pkt.len() < 16 + 8 {
        // noise tag size + nonce
        return Err(false);
    }

    let nonce = pkt.get_u64();
    let mut buffer = vec![0u8; 512];

    let len = cipher
        .read_message(nonce, pkt.chunk(), buffer.as_mut_slice())
        .map_err(|_| true)?;
    let bytes = Bytes::from(buffer).slice(0..len);

    Ok(Packet { data: bytes, nonce })
}
