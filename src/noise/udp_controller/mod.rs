mod client;
mod history;
mod packet;

use crate::error::StreamError;
use bytes::Bytes;
use flume::{Receiver, Sender};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::Semaphore;

pub type Tag = u64;
pub const TAG_SIZE: usize = std::mem::size_of::<Tag>();

static_assertions::const_assert_eq!(TAG_SIZE, 8);

pub struct UdpController {
    socket: UdpSocket,
    out_queue: Receiver<(Tag, Bytes)>,
    clients: HashMap<Tag, ClientRef>,
    send_lock: Semaphore,
    read_lock: Semaphore,
}

struct ClientRef {
    send_queue: Receiver<Bytes>,
    recv_queue: Sender<Bytes>,
    history: history::History,
    cipher: snow::StatelessTransportState,
    last_data: Instant,
    controller: Arc<UdpController>,
}

impl UdpController {
    async fn update(&mut self) -> Result<(), StreamError> {
        todo!()
    }
}

impl ClientRef {}
