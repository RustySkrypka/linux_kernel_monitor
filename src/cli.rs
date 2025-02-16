use std::fs;
use std::io::{BufReader, Write, BufRead};
use std::sync::{Arc, Mutex};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread::JoinHandle;

use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};

use crate::metric::{MetricType, MetricState, str_to_metric, get_metric_types, str_to_state};


pub static SOCKET_PATH: &'static str = "/var/run/lkmonitor.sock";

#[derive(Clone, Copy, Debug)]
pub enum Command {
    Start,
    Stop,
    List,
    Set,
    Store,
    None,
}

#[derive(Debug)]
pub struct CliCommand {
    metric_type: Option<MetricType>,
    cmd: Command,
    refresh_rate: Option<u8>,
    enabled: Option<bool>,
    state: Option<MetricState>,
}

impl CliCommand {
    pub fn new(cmd: Command, metric_type: Option<MetricType>, refresh_rate: Option<u8>, enabled: Option<bool>, state: Option<MetricState>) -> CliCommand {
        CliCommand { cmd, metric_type, refresh_rate, enabled, state }
    }

    pub fn get_command(&self) -> Command {
        self.cmd
    }

    pub fn get_metric_type(&self) -> Option<MetricType> {
        self.metric_type
    }

    pub fn get_refresh_rate(&self) -> Option<u8> {
        self.refresh_rate
    }

    pub fn get_enabled(&self) -> Option<bool> {
        self.enabled
    }

    pub fn get_state(&self) -> Option<MetricState> {
        self.state
    }
}

pub struct Cli {
    server: Arc<Mutex<CliServer>>,
    handle: Option<JoinHandle<()>>,
    lkm_receiver: Arc<Receiver<Vec<CliCommand>>>,
    lkm_sender: Arc<Sender<Vec<String>>>,
}

impl Cli {
    pub fn new() -> Self {
        let socket = Path::new(SOCKET_PATH);
        if socket.exists() {
            fs::remove_file(socket);
        }
        let listener = UnixListener::bind(socket).unwrap();

        let (tx, rx) = unbounded::<Vec<CliCommand>>();
        let cli_sender = Arc::new(tx);
        let lkm_receiver = Arc::new(rx);
        let (tx, rx) = unbounded::<Vec<String>>();
        let lkm_sender = Arc::new(tx);
        let cli_receiver = Arc::new(rx);

        Self {
            server: Arc::new(Mutex::new(CliServer::new(listener, cli_sender, cli_receiver))),
            handle: None,
            lkm_receiver,
            lkm_sender,
        }
    }

    pub fn add_handle(&mut self, handle: JoinHandle<()>) {
        self.handle = Some(handle);
    }

    pub fn get_cli_server(&self) -> Arc<Mutex<CliServer>> {
        self.server.clone()
    }

    pub fn receive_cli_command(&mut self) -> Vec<CliCommand> {
        loop {
            match self.lkm_receiver.try_recv() {
                Ok(cli_commands) => {
                    return cli_commands;
                },
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => eprintln!("cli server disconnected"),
            }
        }
    }

    pub fn send_service_response(&self, result: Vec<String>) {
        self.lkm_sender.send(result);
    }
}

pub struct CliServer {
    listener: UnixListener,
    cli_sender: Arc<Sender<Vec<CliCommand>>>,
    cli_receiver: Arc<Receiver<Vec<String>>>,
}

impl CliServer {
    pub fn new(listener: UnixListener, cli_sender: Arc<Sender<Vec<CliCommand>>>, cli_receiver: Arc<Receiver<Vec<String>>>) -> Self {
        Self { listener, cli_sender, cli_receiver }
    }
    pub fn run(&mut self) {
        loop {
            while let Some(stream) = self.listener.incoming().next() {
                match stream {
                    Ok(mut stream) => {
                        self.handle_cli_client(&mut stream);
                    }
                    Err(e) => {
                        eprintln!("Cli connection failed: {}", e);
                    }
                }
            }
        }
    }

    fn handle_cli_client(&mut self, stream: &mut UnixStream) -> Result<(), std::io::Error> {
        let command_str = self.read_cli_command(&stream)?;
        let cli_commands = parse_args(command_str);
        self.send_cli_command(cli_commands);

        let response = self.receive_service_response();
        self.send_service_response(stream, response)?;
        Ok(())
    }

    fn receive_service_response(&mut self) -> Vec<String> {
        loop {
            match self.cli_receiver.try_recv() {
                Ok(response) => {
                    return response
                },
                Err(TryRecvError::Empty) => {
                    continue;
                },
                Err(TryRecvError::Disconnected) => {
                    eprintln!("cli server disconnected");
                    continue;
                },
            }
        }

    }

    fn send_service_response(&mut self, stream: &mut UnixStream, response: Vec<String>) -> Result<(), std::io::Error> {
        for s in response {
            stream.write_all(s.as_bytes())?;
            stream.flush()?;
        }

        Ok(())
    }

    fn send_cli_command(&self, cli_commands: Vec<CliCommand>) {
        self.cli_sender.send(cli_commands);
    }

    fn read_cli_command(&mut self, stream: &UnixStream) -> Result<String, std::io::Error> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Cli command length == 0"));
        }

        Ok(line)
    }
}

fn parse_args(line: String) -> Vec<CliCommand> {
    let mut commands: Vec<CliCommand> = Vec::new();

    let mut args = line.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>();
    let command = get_command(args.remove(0).as_str());

    match command {
        Command::Start => {
            commands = parse_start_command(&mut args);
        },
        Command::Stop => {
            commands = parse_stop_command(&mut args);
        },
        Command::List => {
            commands = parse_list_command(&mut args);
        },
        Command::Set => {
            commands = parse_set_command(&mut args)
        },
        Command::Store => {
            let cli_command = CliCommand::new(Command::Store, None, None, None, None);
            commands.push(cli_command);
        },
        Command::None => {

        },
    }

    commands
}

fn parse_start_command(args: &mut Vec<String>) -> Vec<CliCommand> {
    let mut commands: Vec<CliCommand> = Vec::new();

    let mut metric_type: Option<MetricType> = None;
    let mut refresh_rate: Option<u8> = None;
    let mut enabled: Option<bool> = None;

    if !args.is_empty() {
        metric_type = Some(str_to_metric(args.remove(0).as_str()));
    }
    if !args.is_empty() {
        refresh_rate = Some(args.remove(0).parse::<u8>().unwrap());
    }
    if !args.is_empty() {
        enabled = Some(args.remove(0).parse::<bool>().unwrap());
    }

    if metric_type.is_some() {
        let cli_command = CliCommand::new(Command::Start, metric_type, refresh_rate, enabled, None);
        commands.push(cli_command);
    } else {
        for metric_type in get_metric_types() {
            let cli_command = CliCommand::new(Command::Start, Some(metric_type), refresh_rate, enabled, None);
            commands.push(cli_command);
        }
    }

    commands
}

fn parse_stop_command(args: &mut Vec<String>) -> Vec<CliCommand> {
    let mut commands: Vec<CliCommand> = Vec::new();

    let mut metric_type: Option<MetricType> = None;

    if !args.is_empty() {
        metric_type = Some(str_to_metric(args.remove(0).as_str()));
    }

    if metric_type.is_some() {
        let cli_command = CliCommand::new(Command::Stop, metric_type, None, None, None);
        commands.push(cli_command);
    } else {
        for metric_type in get_metric_types() {
            let cli_command = CliCommand::new(Command::Stop, Some(metric_type), None, None, None);
            commands.push(cli_command);
        }
    }

    commands
}

fn parse_list_command(args: &mut Vec<String>) -> Vec<CliCommand> {
    let mut commands: Vec<CliCommand> = Vec::new();

    let mut state: Option<MetricState> = None;

    if !args.is_empty() {
        state = str_to_state(args.remove(0).as_str());
    }

    if state.is_some() {
        let cli_command = CliCommand::new(Command::List, None, None, None, state);
        commands.push(cli_command);
    } else {
        for metric_type in get_metric_types() {
            let cli_command = CliCommand::new(Command::List, Some(metric_type), None, None, None);
            commands.push(cli_command);
        }
    }

    commands
}

fn parse_set_command(args: &mut Vec<String>) -> Vec<CliCommand> {
    let mut commands: Vec<CliCommand> = Vec::new();

    let mut metric_type: Option<MetricType> = None;
    let mut refresh_rate: Option<u8> = None;
    let mut enabled: Option<bool> = None;

    if !args.is_empty() {
        metric_type = Some(str_to_metric(args.remove(0).as_str()));
    }
    if !args.is_empty() {
        let arg = args.remove(0);
        let mut arg_u8_test = arg.parse::<u8>();
        if arg_u8_test.is_ok() {
            refresh_rate = Some(arg_u8_test.unwrap());
        } else {
             let arg_bool_test = arg.parse::<bool>();
            if arg_bool_test.is_ok() {
                enabled = Some(arg_bool_test.unwrap());
            }
        }
    }
    if !args.is_empty() {
        let arg = args.remove(0);
        let mut arg_u8_test = arg.parse::<u8>();
        if arg_u8_test.is_ok() {
            refresh_rate = Some(arg_u8_test.unwrap());
        } else {
            let arg_bool_test = arg.parse::<bool>();
            if arg_bool_test.is_ok() {
                enabled = Some(arg_bool_test.unwrap());
            }
        }
    }

    if metric_type.is_some() {
        let cli_command = CliCommand::new(Command::Set, metric_type, refresh_rate, enabled, None);
        commands.push(cli_command);
    } else {
        for metric_type in get_metric_types() {
            let cli_command = CliCommand::new(Command::Set, Some(metric_type), refresh_rate, enabled, None);
            commands.push(cli_command);
        }
    }

    commands
}

pub fn get_command(cmd_str: &str) -> Command {
    match cmd_str {
        "start" => Command::Start,
        "stop" => Command::Stop,
        "list" => Command::List,
        "set" => Command::Set,
        "store" => Command::Store,
        _ => Command::None,
    }
}

