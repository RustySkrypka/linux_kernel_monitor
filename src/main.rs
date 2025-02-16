use linux_kernel_monitor::LinuxKernelMonitor;

fn main() {
    let mut lkm = LinuxKernelMonitor::init();
    lkm.launch();
}
