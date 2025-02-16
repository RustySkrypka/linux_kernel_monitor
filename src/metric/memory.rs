use std::fmt::{Display, Formatter};

use sysinfo::System;

use crate::queue::{ErrInfo, QueueItem};
use crate::metric::MetricCollector;


pub struct MemoryInfo {
    total: u64,
    used: u64,
    free: u64,
}

impl Display for MemoryInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Total memory: {} MB, Used: {} MB, Free: {} MB", self.total, self.used, self.free)?;
        Ok(())
    }
}

pub struct MemoryInfoCollector {
    sys: System,
}

impl MetricCollector for MemoryInfoCollector {
    fn collect_info(&mut self) -> QueueItem {
        QueueItem::Memory(self.get_memory_info())
    }
}

impl MemoryInfoCollector {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    pub fn get_memory_info(&mut self) -> MemoryInfo {
        self.sys.refresh_all();

        MemoryInfo {
            total: self.sys.total_memory() / (1024 * 1024),
            used: self.sys.used_memory() / (1024 * 1024),
            free: self.sys.free_memory() / (1024 * 1024),
        }
    }
}

