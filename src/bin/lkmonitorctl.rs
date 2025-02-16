use std::os::unix::net::UnixStream;
use std::io::{self, Write, BufRead, BufReader};
use std::process::exit;
use clap::{Parser, Subcommand};

use linux_kernel_monitor::cli::SOCKET_PATH;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
#[clap(name = "monitorctl", version = "1.0", author = "Your Name", about = "Control the Linux Kernel Monitor service")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "List metrics by parameters (running, disabled)")]
    List {
        #[clap(long, help = "Show metrics with specified state (state=[running, disabled])", value_parser = validate_state)]
        state: Option<String>,
    },
    #[clap(about = "Starts a metric thread (cpu, memory, io)")]
    Start {
        #[clap(long, help = "Type of metric (cpu, memory, io)", value_parser = validate_metric)]
        metric: Option<String>,
        #[clap(long, help = "Refresh rate in seconds")]
        rate: Option<u8>,
    },
    #[clap(about = "Stops a metric thread (cpu, memory, io)")]
    Stop {
        #[clap(long, help = "Type of metric to remove (cpu, memory, io)", value_parser = validate_metric)]
        metric: Option<String>,
    },
    #[clap(about = "Set config for metric (cpu, memory, io)")]
    Set {
        #[clap(long, help = "Type of metric (cpu, memory, io)", value_parser = validate_metric)]
        metric: String,
        #[clap(long, help = "Refresh rate in seconds")]
        rate: Option<u8>,
        #[clap(long, help = "Start metric on service launch")]
        enabled: Option<bool>,
    },
    #[clap(about = "Store current config to file")]
    Store { },
}

fn validate_metric(metric: &str) -> Result<String, String> {
    let allowed_metrics = vec!["cpu", "memory", "io"];
    if allowed_metrics.contains(&metric) {
       Ok(metric.to_string())
    } else {
        Err(format!("Invalid metric: '{}'. Allowed values are: {:?}", metric, allowed_metrics))
    }
}

fn validate_state(state: &str) -> Result<String, String> {
    let allowed_states = vec!["running", "stopped", "disabled"];
    if allowed_states.contains(&state) {
        Ok(state.to_string())
    } else {
        Err(format!("Invalid metric: '{}'. Allowed values are: {:?}", state, allowed_states))
    }
}

fn main() -> Result<(), io::Error> {
    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    let cli = Cli::parse();
    let mut command = String::new();

    match cli.command {
        Commands::List { state } => {
            match state {
                Some(state) => {
                    command = format!("list {}\n", state)
                },
                None => {
                    command = String::from("list\n")
                }
            }
        },
        Commands::Start { metric, rate } => {
            match (metric, rate) {
                (Some(metric), Some(rate)) => {
                    command = format!("start {} {}\n", metric, rate)
                },
                (Some(metric), None) => {
                    command = format!("start {}\n", metric)
                },
                (None, Some(_)) => {
                    eprintln!("Error: only rate option provided");
                    exit(0)
                },
                (None, None) => {
                    command = String::from("start\n")
                }
            }
        },
        Commands::Stop { metric } => {
            match metric {
                Some(metric) => {
                    command = format!("stop {}\n", metric)
                },
                None => {
                    command = String::from("stop\n")
                }
            }
        }
        Commands::Set { metric, rate, enabled } => {
            match (metric, rate, enabled) {
                (metric, Some(rate), Some(enabled)) => {
                    command = format!("set {} {} {}\n", metric, rate, enabled)
                },
                (metric, Some(rate), None) => {
                    command = format!("set {} {}\n", metric, rate)
                },
                (metric, None, Some(enabled)) => {
                    command = format!("set {} {}\n", metric, enabled)
                }
                (_, None, None) => {
                    eprintln!("Error: only metric option provided");
                    exit(0)
                },
            }
        },
        Commands::Store { } => {
            command = String::from("store\n");
        }
    }

    if !command.len() > 0 {
        stream.write_all(command.as_bytes())?;
        stream.flush()?;

        let mut reader = BufReader::new(&stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        for s in response.trim().split(';') {
            if !s.is_empty() {
                println!("{}", s);
            }
        }
    }

    Ok(())
}