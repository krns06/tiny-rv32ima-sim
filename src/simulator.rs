use std::marker::PhantomData;

use crate::{
    bus::{
        Bus, BusDevice, UART_BASE, UART_END, VIRTIO_GPU_BASE, VIRTIO_GPU_END, VIRTIO_NET_BASE,
        VIRTIO_NET_END, uart::Uart, virtio_gpu::VirtioGpu, virtio_net::VirtioNet,
    },
    cpu::Cpu,
    host_device::HostDeviceManager,
    native::{NativeReciever, NativeSender},
};

#[cfg(not(target_arch = "wasm32"))]
use std::{sync::mpsc, thread};

#[cfg(not(target_arch = "wasm32"))]
use crate::host_device::{gpu::HostGpu, net::HostNet, shell::Shell};

#[cfg(target_arch = "wasm32")]
use crate::{device::DeviceMessage, wasm::WasmGpuSender};
#[cfg(target_arch = "wasm32")]
use web_sys::CanvasRenderingContext2d;

pub struct Simulator<T> {
    cpu: Cpu,
    bus: Bus,
    host_device_manager: Option<HostDeviceManager>,
    _marker: PhantomData<T>,
}

pub struct Initial;
pub struct NativeSetup;
pub struct WasmSetup;

pub struct NativeLoaded;
pub struct WasmLoaded;

impl<T> Simulator<T> {
    pub fn load_flat(&mut self, array: &[u8], addr: u32) {
        self.bus.memory().load_flat_binary(array, addr);
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
}

impl Simulator<Initial> {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::default(),
            bus: Bus::default(),
            host_device_manager: None,
            _marker: PhantomData,
        }
    }

    // native
    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_native_devices(mut self) -> Simulator<NativeSetup> {
        let (uart_tx, uart_rx) = mpsc::channel();

        let uart = BusDevice::new(
            Box::new(Uart::new(NativeReciever::new(uart_rx))),
            UART_BASE..UART_END,
        );

        let shell = Box::new(Shell::new(uart_tx));

        let (net_host_tx, net_guest_rx) = mpsc::channel();
        let (net_guest_tx, net_host_rx) = mpsc::channel();

        let virtio_net = BusDevice::new(
            Box::new(VirtioNet::new(
                NativeReciever::new(net_guest_rx),
                NativeSender::new(net_guest_tx),
            )),
            VIRTIO_NET_BASE..VIRTIO_NET_END,
        );

        let host_net = Box::new(HostNet::new(net_host_rx, net_host_tx));

        let (gpu_tx, gpu_rx) = mpsc::channel();

        let virtio_gpu = BusDevice::new(
            Box::new(VirtioGpu::new(NativeSender::new(gpu_tx))),
            VIRTIO_GPU_BASE..VIRTIO_GPU_END,
        );

        let host_gpu = Box::new(HostGpu::new(gpu_rx));

        self.bus
            .add_device(uart)
            .add_device(virtio_net)
            .add_device(virtio_gpu);

        let mut device_manager = HostDeviceManager::default();

        device_manager
            .add_device(shell)
            .add_device(host_net)
            .add_device(host_gpu);

        Simulator {
            cpu: self.cpu,
            bus: self.bus,
            host_device_manager: Some(device_manager),
            _marker: PhantomData,
        }
    }

    // wasm
    #[cfg(target_arch = "wasm32")]
    pub fn setup_wasm_devices(
        mut self,
        canvas_ctx: CanvasRenderingContext2d,
    ) -> Simulator<WasmSetup> {
        use crate::wasm::WasmUartReciever;

        let uart_reciever = WasmUartReciever::default();
        let uart = BusDevice::new(Box::new(Uart::new(uart_reciever)), UART_BASE..UART_END);

        let virtio_sender = WasmGpuSender::new(canvas_ctx);

        let virtio_gpu = BusDevice::new(
            Box::new(VirtioGpu::new(virtio_sender)),
            VIRTIO_GPU_BASE..VIRTIO_GPU_END,
        );

        self.bus.add_device(uart).add_device(virtio_gpu);

        Simulator {
            cpu: self.cpu,
            bus: self.bus,
            host_device_manager: self.host_device_manager,
            _marker: PhantomData,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Simulator<NativeSetup> {
    pub fn set_entry_point(mut self, entry_point: u32) -> Simulator<NativeLoaded> {
        self.cpu.set_pc(entry_point);

        Simulator {
            cpu: self.cpu,
            bus: self.bus,
            host_device_manager: self.host_device_manager,
            _marker: PhantomData,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Simulator<WasmSetup> {
    pub fn set_entry_point(mut self, entry_point: u32) -> Simulator<WasmLoaded> {
        self.cpu.set_pc(entry_point);

        Simulator {
            cpu: self.cpu,
            bus: self.bus,
            host_device_manager: self.host_device_manager,
            _marker: PhantomData,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Simulator<NativeLoaded> {
    pub fn run(mut self) {
        let device_manager = self.host_device_manager.take().unwrap();

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

#[cfg(target_arch = "wasm32")]
impl Simulator<WasmLoaded> {
    pub fn step(&mut self) {
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

    pub fn send_key(&mut self, key: char) {
        self.bus.push_messaeg(DeviceMessage::Uart(key));
    }
}
