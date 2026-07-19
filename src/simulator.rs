use std::marker::PhantomData;

use crate::{
    bus::{
        Bus, BusDevice, UART_BASE, UART_END, VIRTIO_BLK_BASE, VIRTIO_BLK_END, VIRTIO_GPU_BASE,
        VIRTIO_GPU_END, VIRTIO_NET_BASE, VIRTIO_NET_END, uart::Uart,
    },
    cpu::Cpu,
    host_device::HostDeviceManager,
    native::{NativeReciever, NativeSender},
};

#[cfg(not(target_arch = "wasm32"))]
use std::{sync::mpsc, thread};

#[cfg(target_arch = "wasm32")]
use crate::{
    DeviceMessage,
    wasm::{WasmGpuSender, WasmUartReciever},
};
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

const DEBUG_RUN_MAX_STEPS: u64 = 20_000_000;

impl<T> Simulator<T> {
    pub fn load_flat(&mut self, array: &[u8], addr: u32) {
        self.bus.memory().load_flat_binary(array, addr);
        self.cpu.set_pc(addr);
    }

    pub fn load_elf(&mut self, array: &[u8]) -> u32 {
        let entry_point = self.bus.memory().load_elf_binary(array);
        self.cpu.set_pc(entry_point);
        entry_point
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn debug_run(&mut self, exit_address: u32) -> bool {
        self.cpu.start_debug(exit_address);

        for _ in 0..DEBUG_RUN_MAX_STEPS {
            self.step_once();

            if self.cpu.is_debug_finished() {
                let is_success = self.cpu.is_debug_success();
                self.cpu.stop_debug();
                return is_success;
            }
        }

        self.cpu.stop_debug();
        false
    }

    fn step_once(&mut self) {
        if self.cpu.can_external_interrupt() {
            if self.cpu.is_interrupting() {
                let irq = self.cpu.pop_interrupt().unwrap();
                self.bus.prepare_interrupt(irq);
                self.cpu.raise_interrupt(irq);
            } else {
                self.bus.tick(self.cpu.pending_interrupts_mut());
            }
        } else {
            self.cpu.lower_interrupt();
        }

        if let Some(e) = self.cpu.check_local_intrrupt_active() {
            self.cpu.handle_trap(e);
        }

        match self.cpu.step(&mut self.bus) {
            Err(e) => {
                self.cpu.handle_trap(e);
            }
            Ok(is_jump) => {
                self.cpu.csr_mut().progress_instret();

                if !is_jump {
                    self.cpu.progress_pc();
                }
            }
        }

        self.cpu.csr_mut().progress_cycle();
        self.cpu.csr_mut().progress_time();
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
    // ブロックデバイス用にファイルの中身を持つVecを引数に取る。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_native_devices(mut self, filepath: &str) -> Simulator<NativeSetup> {
        use crate::host_device::shell::Shell;

        let (uart_tx, uart_rx) = mpsc::channel();

        let uart = BusDevice::new(
            Box::new(Uart::new(NativeReciever::new(uart_rx))),
            UART_BASE..UART_END,
        );

        let shell = Box::new(Shell::new(uart_tx));

        self.bus.add_device(uart);

        let mut device_manager = HostDeviceManager::default();

        device_manager.add_device(shell);

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
        use crate::wasm::WasmBlkSender;

        let uart_reciever = WasmUartReciever::default();
        let uart = BusDevice::new(Box::new(Uart::new(uart_reciever)), UART_BASE..UART_END);

        let gpu_sender = WasmGpuSender::new(canvas_ctx);

        let virtio_gpu = BusDevice::new(
            Box::new(VirtioGpu::new(gpu_sender)),
            VIRTIO_GPU_BASE..VIRTIO_GPU_END,
        );

        //let block_bytes = include_bytes!("../statics/fs");
        //let virtio_blk = BusDevice::new(
        //    Box::new(VirtioBlk::new(
        //        WasmBlkSender::default(),
        //        block_bytes.to_vec(),
        //    )),
        //    VIRTIO_BLK_BASE..VIRTIO_BLK_END,
        //);

        self.bus.add_device(uart).add_device(virtio_gpu);
        // .add_device(virtio_blk);

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
            self.step_once();
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Simulator<WasmLoaded> {
    pub fn step(&mut self) {
        self.step_once();
    }

    pub fn send_key(&mut self, key: char) {
        self.bus.push_messaeg(DeviceMessage::Uart(key));
    }
}
