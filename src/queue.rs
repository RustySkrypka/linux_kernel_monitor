use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use crate::metric::cpu::CpuInfo;
use crate::metric::memory::MemoryInfo;
use crate::metric::io::IOInfo;


pub enum QueueItem {
        CPU(CpuInfo),
        Memory(MemoryInfo),
        IO(IOInfo),
        Err(ErrInfo),
}

pub struct ErrInfo {
    error: String,
}

impl Display for ErrInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ErrInfo: {}", self.error)?;
        Ok(())
    }
}

impl ErrInfo {
    pub fn new(error: String) -> ErrInfo {
        ErrInfo { error }
    }
}

pub struct Queue {
    queue: Arc<Mutex<VecDeque<QueueItem>>>,
    cond: Arc<Condvar>,
}

impl Queue {
    pub fn new() -> Self {
        Queue {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            cond: Arc::new(Condvar::new()),
        }
    }

    pub fn enqueue(&self, item: QueueItem) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(item);
        self.cond.notify_one();
    }

    pub fn dequeue(&self) -> Option<QueueItem> {
        let mut q = self.queue.lock().unwrap();
        while q.is_empty() {
            q = self.cond.wait(q).unwrap();
        }
        q.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        let q = self.queue.lock().unwrap();
        q.is_empty()
    }
}