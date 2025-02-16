use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use crate::metric::cpu::CpuInfoCollector;
use crate::metric::io::IOInfoCollector;
use crate::metric::memory::MemoryInfoCollector;
use crate::queue::{Queue, QueueItem};

pub mod memory;
pub mod cpu;
pub mod io;


#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum MetricType {
    CPU,
    Memory,
    IO,
    None,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MetricState {
    Initialized,
    Running,
    Stopped,
    Disabled,
}

pub struct Metric {
    metric: Arc<Mutex<dyn MetricCollector>>,
    metric_type: MetricType,
    queue: Arc<Queue>,
    sender: Arc<Sender<String>>,
    receiver: Arc<Receiver<String>>,
    handle: Option<JoinHandle<()>>,
}

impl Metric {
    pub fn new(metric: Arc<Mutex<dyn MetricCollector>>, metric_type: MetricType, queue: Arc<Queue>, sender: Arc<Sender<String>>, receiver: Arc<Receiver<String>>, handle: Option<JoinHandle<()>>) -> Self {
        Self {
            metric,
            metric_type,
            queue,
            sender,
            receiver,
            handle,
        }
    }

    pub fn get_type(&self) -> MetricType {
        self.metric_type.clone()
    }

    pub fn get_queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    pub fn get_metric_collector(&self) -> Arc<Mutex<dyn MetricCollector>> {
        self.metric.clone()
    }

    pub fn get_receiver(&self) -> Arc<Receiver<String>> {
        self.receiver.clone()
    }

    pub fn add_handle(&mut self, handle: JoinHandle<()>) {
        self.handle = Some(handle);
    }

    pub fn stop(&mut self) -> String {
        self.send_command("stop".to_string());

        let handle = self.handle.take().unwrap();
        let join_result = handle.join();

        if join_result.is_ok() {
           format!("Metric '{}' stopped;", metric_to_str(self.get_type()))
        } else {
            String::from("Error while joining thread;")
        }
    }

    pub fn start(&mut self, refresh_rate: u8) -> String {
        let metric_collector = self.get_metric_collector();
        let rx = self.get_receiver();
        let queue = self.get_queue();

        let handle = thread::spawn(move || {
            let mut metric_guard = metric_collector.lock().unwrap();
            metric_guard.run(rx, refresh_rate, queue);
        });

        self.add_handle(handle);

        format!("Metric '{}' started with rate '{}';", metric_to_str(self.get_type()), refresh_rate.to_string())
    }

    pub fn set_refresh_rate(&mut self, refresh_rate: u8) {
        self.send_command(refresh_rate.to_string());
    }

    fn send_command(&mut self, command: String) {
        self.sender.send(command).unwrap();
    }
}

pub trait MetricCollector: Send + Sync {
    fn run (&mut self, receiver: Arc<Receiver<String>>, refresh_rate: u8, queue: Arc<Queue>) {
        let mut refresh_rate = refresh_rate;

        loop {
            let queue_item = self.collect_info();
            match queue_item {
                QueueItem::CPU(cpu_info) => {
                    queue.enqueue(QueueItem::CPU(cpu_info))
                },
                QueueItem::Memory(mem_info) => {
                    queue.enqueue(QueueItem::Memory(mem_info))
                },
                QueueItem::IO(io_info) => {
                    queue.enqueue(QueueItem::IO(io_info))
                },
                QueueItem::Err(err_info) => {
                    queue.enqueue(QueueItem::Err(err_info))
                },
            }

            sleep(Duration::from_secs(refresh_rate as u64));

            match receiver.try_recv() {
                Ok(command) => {
                    if  command == "stop" {
                        break;
                    } else {
                        refresh_rate = command.parse::<u8>().unwrap();
                    }
                },
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => eprintln!("disconnected"),
            }
        }
    }

    fn collect_info(&mut self) -> QueueItem;
}

pub fn get_metric_collector(metric_type: MetricType) -> Option<Arc<Mutex<dyn MetricCollector>>> {
    match metric_type {
        MetricType::CPU => Some(Arc::new(Mutex::new(CpuInfoCollector::new()))),
        MetricType::Memory => Some(Arc::new(Mutex::new(MemoryInfoCollector::new()))),
        MetricType::IO => Some(Arc::new(Mutex::new(IOInfoCollector::new()))),
        MetricType::None => None,
    }
}



pub fn get_metric_types() -> Vec<MetricType> {
    vec![MetricType::CPU, MetricType::Memory, MetricType::IO]
}

pub fn metric_to_str(metric_type: MetricType) -> &'static str {
    match metric_type {
        MetricType::CPU => "cpu",
        MetricType::Memory => "memory",
        MetricType::IO => "io",
        MetricType::None => "none",
    }
}

pub fn str_to_metric(metric_str: &str) -> MetricType {
    match metric_str {
        "cpu" => MetricType::CPU,
        "memory" => MetricType::Memory,
        "io" => MetricType::IO,
        _ => MetricType::None,
    }
}

pub fn state_to_str(state: MetricState) -> &'static str {
    match state {
        MetricState::Initialized => "initialized",
        MetricState::Stopped => "stopped",
        MetricState::Running => "running",
        MetricState::Disabled => "disabled",
    }
}

pub fn str_to_state(metric_str: &str) -> Option<MetricState> {
    match metric_str {
        "initialized" => Some(MetricState::Initialized),
        "stopped" => Some(MetricState::Stopped),
        "running" => Some(MetricState::Running),
        "disabled" => Some(MetricState::Disabled),
        _ => None,
    }
}