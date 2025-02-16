use std::fmt::{Display, Formatter};

use sysinfo::{Disks, DiskUsage};

use crate::queue::QueueItem;
use crate::metric::MetricCollector;


pub struct IOInfo {
    disks_info: Vec<DiskInfo>
}

pub struct DiskInfo {
    name: String,
    mount_point: String,
    total_space: u64,
    available_space: u64,
}

impl Display for IOInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for disk_info in &self.disks_info {
            write!(f, "name: {}, mount_point: {}, total_space: {} MB, available_space: {} MB\n", disk_info.name, disk_info.mount_point, disk_info.total_space, disk_info.available_space)?;
        }
        Ok(())
    }
}

pub struct IOInfoCollector { }

impl MetricCollector for IOInfoCollector {
    fn collect_info(&mut self) -> QueueItem {
        QueueItem::IO(self.get_io_info())
    }
}

impl IOInfoCollector {
    pub fn new() -> Self {
        Self { }
    }

    pub fn get_io_info(&mut self) -> IOInfo {
        let mut disks_info = Vec::<DiskInfo>::new();

        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            let disk_info = DiskInfo {
                name: disk.name().to_str().unwrap().to_string(),
                mount_point: disk.mount_point().to_str().unwrap().to_string(),
                total_space: disk.total_space() / (1024 * 1024),
                available_space: disk.available_space() / (1024 * 1024),
            };

            disks_info.push(disk_info);
        }

        IOInfo { disks_info }
    }
}