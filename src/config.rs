use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use crate::metric::{get_metric_types, MetricState, MetricType};


pub static CONFIG_PATH: &'static str = "/etc/lkmconfig.toml";

#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub struct MetricsConfig {
    cpu_config: CpuConfig,
    memory_config: MemoryConfig,
    io_config: IOConfig,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            cpu_config: CpuConfig::default(),
            memory_config: MemoryConfig::default(),
            io_config: IOConfig::default(),
        }
    }
}

impl MetricsConfig {
    pub fn get_refresh_rate(&self, metric_type: MetricType) -> u8 {
        match metric_type {
            MetricType::CPU => self.cpu_config.get_refresh_rate(),
            MetricType::Memory => self.memory_config.get_refresh_rate(),
            MetricType::IO => self.io_config.get_refresh_rate(),
            MetricType::None => 0,
        }
    }

    pub fn set_refresh_rate(&mut self, metric_type: MetricType, refresh_rate: u8) {
        match metric_type {
            MetricType::CPU => self.cpu_config.set_refresh_rate(refresh_rate),
            MetricType::Memory => self.memory_config.set_refresh_rate(refresh_rate),
            MetricType::IO => self.io_config.set_refresh_rate(refresh_rate),
            MetricType::None => (),
        }
    }

    pub fn get_enabled(&self, metric_type: MetricType) -> bool {
        match metric_type {
            MetricType::CPU => self.cpu_config.get_enabled(),
            MetricType::Memory => self.memory_config.get_enabled(),
            MetricType::IO => self.io_config.get_enabled(),
            MetricType::None => false,
        }
    }

    pub fn set_enabled(&mut self, metric_type: MetricType, enabled: bool) {
        match metric_type {
            MetricType::CPU => self.cpu_config.set_enabled(enabled),
            MetricType::Memory => self.memory_config.set_enabled(enabled),
            MetricType::IO => self.io_config.set_enabled(enabled),
            MetricType::None => (),
        }
    }
}

pub struct MonitorConfig {
    metrics_config: MetricsConfig,
    states: HashMap<MetricType, MetricState>,
}

impl MonitorConfig {
    pub fn new() -> Self {
        let mut states = HashMap::new();
        states.insert(MetricType::CPU, MetricState::Initialized);
        states.insert(MetricType::Memory, MetricState::Initialized);
        states.insert(MetricType::IO, MetricState::Initialized);

        let mut metrics_config = MetricsConfig::default();

        let config = Path::new(CONFIG_PATH);
        if config.exists() {
            match read_config(CONFIG_PATH) {
                Ok(config) => {
                    metrics_config = config;
                },
                Err(e) => eprintln!("{}", e),
            }
        }

        for metric_type in get_metric_types() {
            if metrics_config.get_enabled(metric_type) {
                states.insert(metric_type, MetricState::Initialized);
            } else {
                states.insert(metric_type, MetricState::Disabled);
            }
        }

        MonitorConfig {
            metrics_config,
            states,
        }
    }

    pub fn get_config(&self) -> &MetricsConfig {
        &self.metrics_config
    }

    pub fn get_state(&self, metric_type: MetricType) -> MetricState {
        self.states.get(&metric_type).unwrap().clone()
    }

    pub fn set_state(&mut self, metric_type: MetricType, state: MetricState) {
        self.states.insert(metric_type, state);
    }

    pub fn get_refresh_rate(&self, metric_type: MetricType) -> u8 {
        self.metrics_config.get_refresh_rate(metric_type)
    }

    pub fn set_refresh_rate(&mut self, metric_type: MetricType, refresh_rate: u8) {
        self.metrics_config.set_refresh_rate(metric_type, refresh_rate);
    }

    pub fn get_enabled(&self, metric_type: MetricType) -> bool {
        self.metrics_config.get_enabled(metric_type)
    }

    pub fn set_enabled(&mut self, metric_type: MetricType, enabled: bool) {
        self.metrics_config.set_enabled(metric_type, enabled);
    }
}

fn read_config(path: &str) -> Result<MetricsConfig, std::io::Error> {
    let config_str = fs::read_to_string(path)?;

    match toml::from_str(&config_str) {
        Ok(config) => Ok(config),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub struct CpuConfig {
    enabled: bool,
    refresh_rate: u8,
}

impl Default for CpuConfig {
    fn default() -> Self {
        CpuConfig {
            enabled: true,
            refresh_rate: 1,
        }
    }
}

impl CpuConfig {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_refresh_rate(&self) -> u8 {
        self.refresh_rate
    }

    pub fn set_refresh_rate(&mut self, refresh_rate: u8) {
        self.refresh_rate = refresh_rate;
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub struct MemoryConfig {
    enabled: bool,
    refresh_rate: u8,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        MemoryConfig {
            enabled: true,
            refresh_rate: 1,
        }
    }
}

impl MemoryConfig {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_refresh_rate(&self) -> u8 {
        self.refresh_rate
    }

    pub fn set_refresh_rate(&mut self, refresh_rate: u8) {
        self.refresh_rate = refresh_rate;
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub struct IOConfig {
    enabled: bool,
    refresh_rate: u8,
}

impl Default for IOConfig {
    fn default() -> Self {
        IOConfig {
            enabled: true,
            refresh_rate: 1,
        }
    }
}

impl IOConfig {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_refresh_rate(&self) -> u8 {
        self.refresh_rate
    }

    pub fn set_refresh_rate(&mut self, refresh_rate: u8) {
        self.refresh_rate = refresh_rate;
    }

}