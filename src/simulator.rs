use std::{error::Error, marker::PhantomData, sync::mpsc, thread};

use crate::{
    bus::{
        Bus, Device, UART_BASE, UART_END, VIRTIO_GPU_BASE, VIRTIO_GPU_END, VIRTIO_NET_BASE,
        VIRTIO_NET_END, uart::Uart, virtio_gpu::VirtioGpu, virtio_net::VirtioNet,
    },
    cpu::Cpu,
    device::{DeviceManager, gpu::HostGpu, net::HostNet, shell::Shell},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct Simulator<T> {
    cpu: Cpu,
    bus: Bus,
    device_manager: Option<DeviceManager>,
    _marker: PhantomData<T>,
}

pub struct Initial;
pub struct NativeSetup;

pub struct Loaded;

impl<T> Simulator<T> {
    pub fn load_flat(&mut self, array: &[u8], addr: u32) {
        self.bus.memory().load_flat_binary(array, addr);
    }
}

impl Simulator<Initial> {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::default(),
            bus: Bus::default(),
            device_manager: None,
            _marker: PhantomData,
        }
    }

    // native
    pub fn setup_native_devices(mut self) -> Simulator<NativeSetup> {
        let (uart_tx, uart_rx) = mpsc::channel();

        let uart = Device::new(Box::new(Uart::new(uart_rx)), UART_BASE..UART_END);

        let shell = Box::new(Shell::new(uart_tx));

        let (net_host_tx, net_guest_rx) = mpsc::channel();
        let (net_guest_tx, net_host_rx) = mpsc::channel();

        let virtio_net = Device::new(
            Box::new(VirtioNet::new(net_guest_rx, net_guest_tx)),
            VIRTIO_NET_BASE..VIRTIO_NET_END,
        );

        let host_net = Box::new(HostNet::new(net_host_rx, net_host_tx));

        let (gpu_tx, gpu_rx) = mpsc::channel();

        let virtio_gpu = Device::new(
            Box::new(VirtioGpu::new(gpu_tx)),
            VIRTIO_GPU_BASE..VIRTIO_GPU_END,
        );

        let host_gpu = Box::new(HostGpu::new(gpu_rx));

        self.bus
            .add_device(uart)
            .add_device(virtio_net)
            .add_device(virtio_gpu);

        let mut device_manager = DeviceManager::default();

        device_manager
            .add_device(shell)
            .add_device(host_net)
            .add_device(host_gpu);

        Simulator {
            cpu: self.cpu,
            bus: self.bus,
            device_manager: Some(device_manager),
            _marker: PhantomData,
        }
    }

    // wasm
}

impl Simulator<NativeSetup> {
    pub fn set_entry_point(mut self, entry_point: u32) -> Simulator<Loaded> {
        self.cpu.set_pc(entry_point);

        Simulator {
            cpu: self.cpu,
            bus: self.bus,
            device_manager: self.device_manager,
            _marker: PhantomData,
        }
    }
}

impl Simulator<Loaded> {
    pub fn run(mut self) {
        let device_manager = self.device_manager.take().unwrap();

        for device in device_manager.devices() {
            thread::spawn(move || device.run());
        }

        loop {
            self.bus.tick(self.cpu.prv(), self.cpu.mut_csr());

            if let Some(e) = self.cpu.check_local_intrrupt_active() {
                self.cpu.handle_trap(e, &mut self.bus);
            }

            match self.cpu.step(&mut self.bus) {
                Err(e) => {
                    self.cpu.handle_trap(e, &mut self.bus);
                }
                Ok(is_jump) => {
                    self.cpu.mut_csr().progress_instret();

                    if !is_jump {
                        self.cpu.progress_pc();
                    }
                }
            }

            self.cpu.mut_csr().progress_cycle();
            self.cpu.mut_csr().progress_time();
        }
    }
}
