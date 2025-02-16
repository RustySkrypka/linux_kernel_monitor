pub mod metric;
pub mod config;
pub mod queue;
pub mod logger;
pub mod cli;

use std::sync::Arc;
use std::thread;
use std::{fs, fs::File};
use std::path::Path;
use clap::builder::TypedValueParser;
use crossbeam_channel::unbounded;

use queue::Queue;
use config::{MonitorConfig, CONFIG_PATH};
use logger::Logger;
use cli::{Cli, CliCommand, Command};
use metric::{Metric, MetricCollector, MetricState, MetricType,
             get_metric_collector, get_metric_types, metric_to_str, state_to_str};

pub struct LinuxKernelMonitor {
    config: MonitorConfig,
    metrics: Vec<Metric>,
    logger: Logger,
    cli: Cli,
}

impl LinuxKernelMonitor {
    pub fn init() -> Self {
        let queue = Arc::new(Queue::new());

        let mut config = MonitorConfig::new();

        let mut metrics = Vec::new();
        for metric_type in get_metric_types() {
            if metric_type == MetricType::None {
                continue;
            }

            let (tx, rx) = unbounded::<String>();
            let metric = Metric::new(get_metric_collector(metric_type).unwrap(), metric_type, queue.clone(), Arc::new(tx), Arc::new(rx), None);
            if config.get_state(metric.get_type()) != MetricState::Disabled {
                config.set_state(metric_type, MetricState::Initialized);
            }
            metrics.push(metric);
        }

        let logger = Logger::new(queue.clone());

        let cli = Cli::new();

        LinuxKernelMonitor {
            config,
            metrics,
            logger,
            cli,
        }
    }

    pub fn launch(&mut self) {
        self.launch_metrics();
        self.launch_logger();
        self.launch_cli();

        self.listen_cli();
    }

    fn launch_metrics(&mut self) {
        for metric in self.metrics.iter_mut() {
            if self.config.get_state(metric.get_type()) != MetricState::Disabled {
                metric.start(self.config.get_refresh_rate(metric.get_type()));
                self.config.set_state(metric.get_type(), MetricState::Running)
            }
        }
    }

    fn launch_logger(&mut self) {
        let logger_ref = self.logger.get_log_collector();
        let handle = thread::spawn(move || {
            let mut log_guard = logger_ref.lock().unwrap();
            log_guard.run();
        });
        self.logger.add_handle(handle);
    }

    fn launch_cli(&mut self) {
        let cli_server_ref = self.cli.get_cli_server();

        let handle = thread::spawn(move || {
            let mut cli_server_guard = cli_server_ref.lock().unwrap();
            cli_server_guard.run();
        });
        self.cli.add_handle(handle);
    }

    fn listen_cli(&mut self) {
        loop {
            let cli_commands = self.cli.receive_cli_command();
            let result = self.handle_cli_commands(cli_commands);
            self.cli.send_service_response(result);
        }
    }

    fn handle_cli_commands(&mut self, cli_commands: Vec<CliCommand>) -> Vec<String> {
        let mut results = Vec::new();

        for cli_command in cli_commands {
            match cli_command.get_command() {
                Command::Start => {
                    let result_str = self.start_metric(cli_command);
                    results.push(result_str);
                },
                Command::Stop => {
                    let result_str = self.stop_metric(cli_command);
                    results.push(result_str);
                },
                Command::List => {
                    let result_str = self.metric_info(cli_command);
                    results.push(result_str);
                },
                Command::Set => {
                    let result_str = self.set_config(cli_command);
                    results.push(result_str);
                },
                Command::Store => {
                    let result_str = self.store_config();
                    results.push(result_str);
                },
                Command::None => { },
            }
        }

        results
    }

    fn start_metric(&mut self, cli_command: CliCommand) -> String {
        let mut result_str = String::new();
        let refresh_rate = cli_command.get_refresh_rate();
        let metric_type = cli_command.get_metric_type().unwrap();
        let metric_state = self.config.get_state(metric_type);

            for metric in self.metrics.iter_mut() {
                if metric.get_type() == metric_type {
                    if metric_state == MetricState::Running {
                        if refresh_rate.is_some() {
                            if self.config.get_refresh_rate(metric_type) != refresh_rate.unwrap() {
                                metric.set_refresh_rate(refresh_rate.unwrap());
                                result_str = format!("'{}' metric 'rate' is set to '{}';", metric_to_str(metric.get_type()), refresh_rate.unwrap());
                            } else {
                                result_str = format!("'{}' metric already running with rate '{}';", metric_to_str(metric_type), self.config.get_refresh_rate(metric_type));
                            }
                        } else {
                            result_str = format!("Metric '{}' already running;", metric_to_str(metric_type));
                        }
                    } else {
                        if refresh_rate.is_some() {
                            result_str = metric.start(refresh_rate.unwrap());
                            self.config.set_state(metric_type, MetricState::Running);
                        } else {
                            result_str = metric.start(self.config.get_refresh_rate(metric_type));
                        }
                    }

                    self.config.set_state(metric_type, MetricState::Running);
                }
            }

        result_str
    }

    fn stop_metric(&mut self, cli_command: CliCommand) -> String {
        let mut result_str = String::new();
        let metric_type = cli_command.get_metric_type().unwrap();
        let metric_state = self.config.get_state(metric_type);

        for metric in self.metrics.iter_mut() {
            if metric.get_type() == metric_type {

                if metric_state == MetricState::Running {
                    result_str = metric.stop();
                    self.config.set_state(metric_type, MetricState::Stopped);
                } else if metric_state == MetricState::Stopped {
                    result_str = format!("Metric '{}' already stopped;", metric_to_str(metric_type));
                } else {
                    result_str = format!("Metric '{}' disabled;", metric_to_str(metric_type));
                }

            }
        }

        if result_str.is_empty() {
            result_str = String::from("All metrics already stopped;");
        }

        result_str
    }

    fn metric_info(&mut self, cli_command: CliCommand) -> String {
        let mut result_str = String::new();
        let state = cli_command.get_state();

        if state.is_some() {
            for metric_type in get_metric_types() {

                let metric_state = self.config.get_state(metric_type);
                if metric_state == state.unwrap() {
                    result_str = result_str + format!("Metric '{}' is in state '{}' rate '{}';", metric_to_str(metric_type), state_to_str(metric_state), self.config.get_refresh_rate(metric_type)).as_str();
                }
            }

            if result_str.is_empty() {
                result_str = format!("No metrics in state '{}';", state_to_str(state.unwrap()));
            }

        } else {
                let metric_type = cli_command.get_metric_type().unwrap();
                let metric_state = self.config.get_state(metric_type);
                result_str = result_str + format!("Metric '{}' is in state '{}' rate '{}';", metric_to_str(metric_type), state_to_str(metric_state), self.config.get_refresh_rate(metric_type)).as_str();
        }

        result_str
    }

    fn set_config(&mut self, cli_command: CliCommand) -> String {
        let mut result_str = String::new();

        let metric_type = cli_command.get_metric_type().unwrap();
        let metric_state = self.config.get_state(metric_type);
        let refresh_rate = cli_command.get_refresh_rate();
        let enabled = cli_command.get_enabled();

        for metric in self.metrics.iter_mut() {
            if metric.get_type() == metric_type {
                if enabled.is_some() {
                    let metric_enabled = self.config.get_enabled(metric.get_type());
                    if enabled.unwrap() != metric_enabled {
                        self.config.set_enabled(metric_type, enabled.unwrap());
                        result_str = result_str + format!("'{}' metric 'enabled' is set to '{}';", metric_to_str(metric_type), enabled.unwrap()).as_str();

                        if metric_state == MetricState::Running && enabled.unwrap() == false {
                            let s = metric.stop();
                            result_str = result_str + s.as_str();
                            self.config.set_state(metric_type, MetricState::Disabled);
                        } else if metric_state == MetricState::Disabled && enabled.unwrap() == true {
                            let s = metric.start(self.config.get_refresh_rate(metric.get_type()));
                            result_str = result_str + s.as_str();
                            self.config.set_state(metric_type, MetricState::Running);
                        }

                    } else {
                        result_str = result_str + format!("'{}' metric 'enabled' is is already '{}';", metric_to_str(metric_type), metric_enabled).as_str();
                    }
                }

                if refresh_rate.is_some() {
                    let metric_rate = self.config.get_refresh_rate(metric.get_type());
                    if metric_rate != refresh_rate.unwrap() {
                        self.config.set_refresh_rate(metric.get_type(), refresh_rate.unwrap());

                        if metric_state == MetricState::Running {
                            metric.set_refresh_rate(refresh_rate.unwrap());
                        }

                        result_str = result_str + format!("'{}' metric 'rate' is set to '{}';", metric_to_str(metric.get_type()), refresh_rate.unwrap()).as_str();
                    } else {
                        result_str = result_str + format!("'{}' metric 'rate' is is already '{}';", metric_to_str(metric_type), metric_rate).as_str();
                    }
                }
            }
        }

        result_str
    }

    fn store_config(&mut self) -> String {
        let mut result_str = String::new();

        let config_path = Path::new(CONFIG_PATH);
        if config_path.exists() {
            let res = File::create(config_path);
            if res.is_err() {
                result_str = format!("Error: failed to create config file: {:?};", res.unwrap_err());
                return result_str;
            }
        }

        let new_toml_string = toml::to_string(&self.config.get_config());
        if new_toml_string.is_err() {
            result_str = format!("Error: failed to serialize config file: {:?};", new_toml_string.unwrap_err());
            return result_str;
        }

        let res = fs::write(config_path, new_toml_string.unwrap());
        if res.is_err() {
            result_str = format!("Error: failed to write config file: {:?};", res.unwrap_err());
            return result_str;
        }

        result_str = String::from("Config successfully saved");

        result_str
    }
}