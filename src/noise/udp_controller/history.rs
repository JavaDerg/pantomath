use bytes::Bytes;
use std::collections::VecDeque;

pub const MAX_SIZE: usize = 128 * MAX_PKT;
pub const MAX_PKT: usize = 256;

pub struct History {
    inner: VecDeque<Bytes>,
    seq: u64,
    rel_size: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            inner: VecDeque::with_capacity(MAX_PKT),
            seq: 0,
            rel_size: 0,
        }
    }

    pub fn push(&mut self, bytes: Bytes) -> u64 {
        if self.full() {
            let _ = self.pop();
        }
        self.rel_size += bytes.len();
        self.inner.push_back(bytes);
        self.seq + self.inner.len() - 1
    }

    pub fn pop(&mut self) -> Option<Bytes> {
        let res = self.inner.pop_front();
        if let Some(bytes) = &res {
            self.rel_size -= bytes.len();
            self.seq += 1;
        }
        res
    }

    pub fn get(&self, seq: u64) -> Option<Bytes> {
        let index = seq.checked_sub(self.seq)?;
        self.inner.get(index as usize).cloned()
    }

    fn full(&self) -> bool {
        self.inner.len() >= MAX_PKT || self.rel_size >= MAX_SIZE
    }
}
