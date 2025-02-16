use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::queue::{Queue, QueueItem, ErrInfo};

pub struct Logger {
    log_collector: Arc<Mutex<LogCollector>>,
    handle: Option<JoinHandle<()>>,
}

impl Logger {
    pub fn new(queue: Arc<Queue>) -> Self {
        Self {
            log_collector: Arc::new(Mutex::new(LogCollector::new(queue))),
            handle: None,
        }
    }

    pub fn add_handle(&mut self, handle: JoinHandle<()>) {
        self.handle = Some(handle);
    }

    pub fn get_log_collector(&self) -> Arc<Mutex<LogCollector>> {
        self.log_collector.clone()
    }
}

pub struct LogCollector {
    queue: Arc<Queue>,
}

impl LogCollector {
    pub fn new(queue: Arc<Queue>) -> Self {
        Self { queue }
    }

    pub fn run(&self) {
        loop {
            if let Some(item) = self.queue.dequeue() {
                match item {
                    QueueItem::CPU(cpu_info) => println!("CPU Info:\n{}", cpu_info),
                    QueueItem::Memory(mem_info) => println!("Memory Info:\n{}", mem_info),
                    QueueItem::IO(io_info) => println!("I/O Info:\n{}", io_info),
                    QueueItem::Err(err_info) => println!("{}", err_info),
                }
            }
        }
    }
}