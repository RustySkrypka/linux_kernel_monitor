use std::fmt::{Display, Formatter};

use sysinfo::System;

use crate::metric::MetricCollector;
use crate::queue::QueueItem;


struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
    guest: u64,
    guest_nice: u64,
    total: u64,
}

pub struct CpuInfo {
    cpus: Vec<(String, f32)>,
}

impl Display for CpuInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (name, load) in &self.cpus {
            writeln!(f, " {}: Load: {:.2}%", name, load)?;
        }
        Ok(())
    }
}

pub struct CpuInfoCollector {
    sys: System,
}

impl MetricCollector for CpuInfoCollector {
    fn collect_info(&mut self) -> QueueItem {
        QueueItem::CPU(self.get_cpu_info())
    }
}

impl CpuInfoCollector {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    pub fn get_cpu_info(&mut self) -> CpuInfo {
        self.sys.refresh_all();

        let cpus = self.sys.cpus();
        let cpu_data: Vec<(String, f32)> = cpus
            .iter()
            .map(|cpu| (cpu.name().to_string(), cpu.cpu_usage()))
            .collect();

        CpuInfo { cpus: cpu_data }
    }
}

