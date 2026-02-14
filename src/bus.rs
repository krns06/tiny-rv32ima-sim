use std::{collections::VecDeque, ops::Range};

#[cfg(target_arch = "wasm32")]
use crate::device::DeviceMessage;
use crate::{
    AccessType, IRQ, Priv, Result, Trap,
    bus::{clint::Clint, plic::Plic},
    csr::Csr,
    device::DeviceTrait,
    memory::Memory,
};

mod clint;
mod plic;

pub mod uart;
pub mod virtio_gpu;
pub mod virtio_mmio;
pub mod virtio_net;

pub const MEMORY_BASE: u32 = 0x80000000;
pub const MEMORY_END: u32 = 0x90000000;

const CLINT_BASE: u32 = 0x2000000;
const CLINT_END: u32 = CLINT_BASE + 0x10000;

const PLIC_BASE: u32 = 0xc000000;
const PLIC_END: u32 = PLIC_BASE + 0x4000000;

pub const UART_BASE: u32 = 0x10000000;
pub const UART_END: u32 = UART_BASE + 0x100;

pub const VIRTIO_NET_BASE: u32 = 0x10008000;
pub const VIRTIO_NET_END: u32 = VIRTIO_NET_BASE + 0x1000;

pub const VIRTIO_GPU_BASE: u32 = 0x10009000;
pub const VIRTIO_GPU_END: u32 = VIRTIO_GPU_BASE + 0x801000;

pub struct CpuContext<'a> {
    pub csr: &'a mut Csr,

    pub is_walk: bool,
    pub access_type: AccessType,
}

pub struct BusDevice {
    device: Box<dyn DeviceTrait>,
    range: Range<u32>,
}

pub struct Bus {
    memory: Memory,

    clint: Clint,
    plic: Plic,

    devices: Vec<BusDevice>,

    irqs_to_raise: VecDeque<IRQ>,

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
        let clint = Clint::default();
        let plic = Plic::default();

        Self {
            memory,
            clint,
            plic,
            devices: Vec::new(),
            irqs_to_raise: VecDeque::new(),
            #[cfg(target_arch = "wasm32")]
            incoming_messages: VecDeque::new(),
        }
    }
}

impl Bus {
    #[inline]
    pub fn read(&mut self, addr: u32, size: u32, ctx: CpuContext) -> Result<u32> {
        match addr {
            CLINT_BASE..CLINT_END => self.clint.read(addr - CLINT_BASE, size, ctx.csr),
            PLIC_BASE..PLIC_END => self.plic.read(addr - PLIC_BASE, size, ctx.csr),
            MEMORY_BASE..MEMORY_END => {
                self.memory
                    .read(addr - MEMORY_BASE, size, ctx.access_type, ctx.is_walk)
            }
            _ => {
                for i in 0..self.devices.len() {
                    if self.devices[i].range.contains(&addr) {
                        let offset = addr - self.devices[i].range.start;
                        let res = self.devices[i]
                            .device
                            //[todo] read内でaccess_type事に例外を出すように変更する。
                            .read(offset, size, &mut self.memory)?;

                        if res.is_interrupting {
                            let irq = self.devices[i].device.irq();
                            self.irqs_to_raise.push_back(irq);
                        }
                        return Ok(res.value);
                    }
                }

                Err(ctx.make_trap())
            }
        }
    }

    #[inline]
    pub fn write(&mut self, addr: u32, size: u32, value: u32, ctx: CpuContext) -> Result<()> {
        match addr {
            CLINT_BASE..CLINT_END => self.clint.write(addr - CLINT_BASE, size, value, ctx.csr),
            PLIC_BASE..PLIC_END => self.plic.write(addr - PLIC_BASE, size, value, ctx.csr),
            MEMORY_BASE..MEMORY_END => self.memory.write(
                addr - MEMORY_BASE,
                size,
                value,
                ctx.access_type,
                ctx.is_walk,
            ),
            _ => {
                for i in 0..self.devices.len() {
                    if self.devices[i].range.contains(&addr) {
                        let offset = addr - self.devices[i].range.start;
                        let res =
                            self.devices[i]
                                .device
                                .write(offset, size, value, &mut self.memory)?;

                        if res.is_interrupting {
                            let irq = self.devices[i].device.irq();
                            self.irqs_to_raise.push_back(irq);
                        }
                        return Ok(res.value);
                    }
                }

                Err(ctx.make_trap())
            }
        }
    }

    pub fn add_device(&mut self, device: BusDevice) -> &mut Self {
        self.devices.push(device);

        self
    }

    #[inline]
    pub fn tick(&mut self, prv: Priv, csr: &mut Csr) {
        if !csr.can_external_interrupt(prv) {
            return;
        }

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
                self.irqs_to_raise.push_back(device.device.irq());
            }
        }

        if self.irqs_to_raise.len() != 0 {
            let irq = self.irqs_to_raise.pop_front().unwrap();
            self.raise_irq(irq);
            self.raise_interrupt(csr);
            return;
        } else {
            csr.set_mip_seip(0);
        }
    }

    #[inline]
    fn raise_irq(&mut self, irq: IRQ) {
        self.plic.set_pending(irq);
    }

    #[inline]
    fn raise_interrupt(&mut self, csr: &mut Csr) {
        if let Some(prv) = self.plic.raise_interrupt() {
            match prv {
                Priv::Machine => csr.set_mip_meip(1),
                Priv::Supervisor => csr.set_mip_seip(1),
                _ => unreachable!(),
            }
        }
    }

    #[inline]
    pub fn prepare_interrupt(&mut self) {
        let irq = self.plic.interrupting_irq().unwrap();

        for i in 0..self.devices.len() {
            if self.devices[i].device.irq() == irq {
                self.devices[i].device.take_interrupt();
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
