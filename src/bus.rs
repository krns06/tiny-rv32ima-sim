use std::{ops::Range, sync::mpsc::Receiver};

use crate::{
    AccessType, IRQ, Priv, Result, Trap,
    bus::{clint::Clint, plic::Plic, uart::Uart},
    csr::Csr,
    memory::Memory,
};

mod clint;
mod plic;
mod uart;
mod virtio_mmio;
mod virtio_net;

pub const MEMORY_BASE: u32 = 0x80000000;
pub const MEMORY_END: u32 = 0x90000000;

const CLINT_BASE: u32 = 0x2000000;
const CLINT_END: u32 = CLINT_BASE + 0x10000;

const PLIC_BASE: u32 = 0xc000000;
const PLIC_END: u32 = PLIC_BASE + 0x4000000;

const UART_BASE: u32 = 0x10000000;
const UART_END: u32 = UART_BASE + 0x100;

pub struct CpuContext<'a> {
    pub csr: &'a mut Csr,

    pub is_walk: bool,
    pub access_type: AccessType,
}

pub struct ExternalDeviceResponse<T> {
    pub value: T,
    pub is_interrupting: bool,
}

pub type ExternalDeviceResult<T> = Result<ExternalDeviceResponse<T>>;

// 外部割り込みを起こす可能性があるデバイス
pub trait ExternalDevice: std::fmt::Debug {
    fn read(&mut self, offset: u32, size: u32, memory: &mut Memory) -> ExternalDeviceResult<u32>;
    fn write(
        &mut self,
        offset: u32,
        size: u32,
        value: u32,
        memory: &mut Memory,
    ) -> ExternalDeviceResult<()>;

    fn irq(&self) -> IRQ;

    // 割り込みが起こったときのみ行う必要があるもののフラグの切り替えに使用する関数
    fn take_interrupt(&mut self) {}

    // tickごとに実行される関数
    // 外部割り込みが有効な場合に実行される
    fn tick(&mut self) -> TickStatus {
        TickStatus::None
    }
}

#[derive(PartialEq, Eq)]
pub enum TickStatus {
    Enable,
    Disable,
    None,
}

#[derive(Debug)]
pub struct Device {
    device: Box<dyn ExternalDevice>,
    range: Range<u32>,
}

pub struct Bus {
    memory: Memory,

    clint: Clint,
    plic: Plic,

    devices: Vec<Device>,
}

impl Device {
    pub fn new(device: Box<dyn ExternalDevice>, range: Range<u32>) -> Self {
        Self { device, range }
    }
}

impl<'a> CpuContext<'a> {
    #[inline]
    pub fn make_trap(&self) -> Trap {
        self.access_type.into_trap(self.is_walk)
    }
}

impl Bus {
    pub fn new(uart_rx: Receiver<char>) -> Self {
        let memory = Memory::default();
        let clint = Clint::default();
        let plic = Plic::default();

        let mut devices = Vec::new();
        devices.push(Device::new(
            Box::new(Uart::new(uart_rx)),
            UART_BASE..UART_END,
        ));

        Self {
            memory,
            clint,
            plic,
            devices,
        }
    }

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
                            .read(offset, size, &mut self.memory)?;

                        if res.is_interrupting {
                            let irq = self.devices[i].device.irq();
                            self.raise_irq(irq);
                            self.raise_interrupt(ctx.csr);
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
                            self.raise_irq(irq);
                            self.raise_interrupt(ctx.csr);
                        }
                        return Ok(res.value);
                    }
                }

                Err(ctx.make_trap())
            }
        }
    }

    #[inline]
    pub fn tick(&mut self, prv: Priv, csr: &mut Csr) {
        if !csr.can_external_interrupt(prv) {
            return;
        }

        let mut next_status = TickStatus::None;
        let mut irq = IRQ::None;

        for device in &mut self.devices {
            let status = device.device.tick();

            match next_status {
                TickStatus::Enable => {}
                TickStatus::Disable | TickStatus::None => {
                    if status != TickStatus::None {
                        if status == TickStatus::Enable {
                            irq = device.device.irq();
                        }

                        next_status = status;
                    }
                }
            }
        }

        match next_status {
            TickStatus::Enable => {
                self.raise_irq(irq);
                self.raise_interrupt(csr);
            }
            TickStatus::Disable => csr.set_mip_seip(0),
            TickStatus::None => {}
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

    pub fn plic(&mut self) -> &mut Plic {
        &mut self.plic
    }

    pub fn devices(&mut self) -> &mut Vec<Device> {
        &mut self.devices
    }
}
