use std::{collections::VecDeque, ops::Range};

#[cfg(target_arch = "wasm32")]
use crate::DeviceMessage;
use crate::{
    AccessType, Result, Trap,
    csr::Csr,
    memory::Memory,
    plic::{Irq, IrqQueue},
};

pub mod uart;
pub mod virtio_blk;
pub mod virtio_gpu;
pub mod virtio_mmio;
pub mod virtio_net;

pub const MEMORY_BASE: u32 = 0x80000000;
pub const MEMORY_END: u32 = 0x90000000;

pub const UART_BASE: u32 = 0x10000000;
pub const UART_END: u32 = UART_BASE + 0x100;

pub const VIRTIO_NET_BASE: u32 = 0x10008000;
pub const VIRTIO_NET_END: u32 = VIRTIO_NET_BASE + 0x1000;

pub const VIRTIO_GPU_BASE: u32 = 0x10009000;
pub const VIRTIO_GPU_END: u32 = VIRTIO_GPU_BASE + 0x801000;

pub const VIRTIO_BLK_BASE: u32 = 0x10900000;
pub const VIRTIO_BLK_END: u32 = VIRTIO_BLK_BASE + 0x1000;

pub struct CpuContext<'a> {
    pub csr: &'a mut Csr,
    pub pending_interrupts: &'a mut IrqQueue,

    pub is_walk: bool,
    pub access_type: AccessType,
}

pub struct DeviceContext<'a> {
    pub pending_interrupts: &'a mut IrqQueue,
    pub memory: &'a mut Memory,
}

// 仮想デバイスについてのトレイト
pub trait DeviceTrait {
    fn read(&mut self, offset: u32, size: u32, ctx: DeviceContext) -> Option<u32>;
    fn write(&mut self, offset: u32, size: u32, value: u32, ctx: DeviceContext) -> Option<()>;

    fn irq(&self) -> Irq;

    // 割り込みが起こったときのみ行う必要があるもののフラグの切り替えに使用する関数
    fn prepare_interrupt(&mut self) {}

    // イベントループからメッセージをデバイスに通知するときに使われる関数
    // そのデバイス向けではない場合は受け取るべきではない。
    #[cfg(target_arch = "wasm32")]
    fn handle_incoming(&mut self, message: &DeviceMessage) {}

    // tickごとに実行される関数
    // 外部割り込みが有効な場合に実行される
    fn tick(&mut self, _: &mut Memory) -> bool {
        false
    }
}

pub struct BusDevice {
    device: Box<dyn DeviceTrait>,
    range: Range<u32>,
}

pub struct Bus {
    memory: Memory,

    devices: Vec<BusDevice>,

    #[cfg(target_arch = "wasm32")]
    incoming_messages: VecDeque<DeviceMessage>,
}

impl BusDevice {
    pub fn new(device: Box<dyn DeviceTrait>, range: Range<u32>) -> Self {
        Self { device, range }
    }
}

impl<'a> CpuContext<'a> {
    #[inline]
    pub fn make_trap(&self) -> Trap {
        self.access_type.into_trap(self.is_walk)
    }
}

impl Default for Bus {
    fn default() -> Self {
        let memory = Memory::default();

        Self {
            memory,
            devices: Vec::new(),
            #[cfg(target_arch = "wasm32")]
            incoming_messages: VecDeque::new(),
        }
    }
}

impl Bus {
    #[inline]
    pub fn read(&mut self, addr: u32, size: u32, ctx: CpuContext) -> Result<u32> {
        for i in 0..self.devices.len() {
            if self.devices[i].range.contains(&addr) {
                let offset = addr - self.devices[i].range.start;
                if let Some(v) = self.devices[i].device.read(
                    offset,
                    size,
                    DeviceContext {
                        pending_interrupts: ctx.pending_interrupts,
                        memory: &mut self.memory,
                    },
                ) {
                    return Ok(v);
                }
            }
        }

        Err(ctx.make_trap())
    }

    #[inline]
    pub fn write(&mut self, addr: u32, size: u32, value: u32, ctx: CpuContext) -> Result<()> {
        for i in 0..self.devices.len() {
            if self.devices[i].range.contains(&addr) {
                let offset = addr - self.devices[i].range.start;
                if let Some(_) = self.devices[i].device.write(
                    offset,
                    size,
                    value,
                    DeviceContext {
                        pending_interrupts: ctx.pending_interrupts,
                        memory: &mut self.memory,
                    },
                ) {
                    return Ok(());
                }
            }
        }

        Err(ctx.make_trap())
    }

    pub fn add_device(&mut self, device: BusDevice) -> &mut Self {
        self.devices.push(device);

        self
    }

    #[inline]
    pub fn tick(&mut self, pending_interrupts: &mut IrqQueue) {
        #[cfg(target_arch = "wasm32")]
        let message = self
            .incoming_messages
            .pop_front()
            .unwrap_or(DeviceMessage::None);

        for device in &mut self.devices {
            #[cfg(target_arch = "wasm32")]
            device.device.handle_incoming(&message);

            let is_interrupting = device.device.tick(&mut self.memory);

            if is_interrupting {
                pending_interrupts.push(device.device.irq());
            }
        }
    }

    #[inline]
    pub fn prepare_interrupt(&mut self, irq: Irq) {
        for i in 0..self.devices.len() {
            if self.devices[i].device.irq() == irq {
                self.devices[i].device.prepare_interrupt();
                return;
            }
        }

        unreachable!();
    }

    pub fn memory(&mut self) -> &mut Memory {
        &mut self.memory
    }

    #[cfg(target_arch = "wasm32")]
    pub fn push_messaeg(&mut self, message: DeviceMessage) {
        self.incoming_messages.push_back(message);
    }
}
