use crate::noise::udp_controller::packet::Packet;
use std::collections::{HashMap, VecDeque};

pub const MAX_SIZE: usize = 256 * MAX_PKT;
pub const MAX_PKT: usize = 128;

pub struct History {
    inner: HashMap<u64, Packet>,
    order: VecDeque<u64>,
    rel_size: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            inner: HashMap::with_capacity(MAX_PKT),
            order: VecDeque::with_capacity(MAX_PKT),
            rel_size: 0,
        }
    }

    pub fn push(&mut self, packet: Packet) -> u64 {
        if self.full() {
            let _ = self.pop();
        }
        self.rel_size += packet.data.len();
        let nonce = packet.nonce;
        self.inner.insert(nonce, packet);
        self.order.push_back(nonce);
        nonce
    }

    pub fn pop(&mut self) -> Option<Packet> {
        self.order.pop_front().map(|id| {
            let packet = self.inner.remove(&id).unwrap();
            self.rel_size -= packet.data.len();
            packet
        })
    }

    pub fn get(&self, nonce: u64) -> Option<Packet> {
        self.inner.get(&nonce).cloned()
    }

    fn full(&self) -> bool {
        self.inner.len() >= MAX_PKT || self.rel_size >= MAX_SIZE
    }
}
