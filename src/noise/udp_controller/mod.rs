mod history;

use crate::error::StreamError;
use bytes::Bytes;
use flume::{Receiver, Sender};
use std::collections::{HashMap, VecDeque};
use tokio::net::UdpSocket;

pub type Tag = u64;
pub const TAG_SIZE: usize = std::mem::size_of::<Tag>();

static_assertions::const_assert_eq!(TAG_SIZE, 8);

pub struct UdpController {
    socket: UdpSocket,
    out_queue: Receiver<(Tag, Bytes)>,
    clients: HashMap<Tag, ClientRef>,
}

struct ClientRef {
    send_queue: Receiver<Bytes>,
    recv_queue: Sender<Bytes>,
    history: history::History,
}

pub struct Client {}

impl UdpController {
    async fn update(&mut self) -> Result<(), StreamError> {
        todo!()
    }
}
