use crate::{
    AccessType, Result, Trap,
    bus::{clint::Clint, plic::Plic, uart::Uart},
    csr::Csr,
    memory::Memory,
};

mod clint;
mod plic;
mod uart;

const CLINT_BASE: u32 = 0x2000000;
const CLINT_END: u32 = CLINT_BASE + 0x10000;

const PLIC_BASE: u32 = 0xc000000;
const PLIC_END: u32 = PLIC_BASE + 0x4000000;

const UART_BASE: u32 = 0x10000000;
const UART_END: u32 = UART_BASE + 0x100;

const MEMORY_BASE: u32 = 0x80000000;
const MEMORY_END: u32 = 0x90000000;

pub struct CpuContext<'a> {
    pub csr: &'a mut Csr,

    pub is_walk: bool,
    pub access_type: AccessType,
}

impl<'a> CpuContext<'a> {
    #[inline]
    pub fn make_trap(&self) -> Trap {
        self.access_type.into_trap(self.is_walk)
    }
}

pub trait MmioOps {
    fn read(&mut self, offset: u32, size: u32, ctx: CpuContext) -> Result<Vec<u8>>;
    fn write(&mut self, offset: u32, array: &[u8], ctx: CpuContext) -> Result<()>;

    fn read_u8(&mut self, offset: u32, ctx: CpuContext) -> Result<u8> {
        let value = self.read(offset, 1, ctx)?;

        Ok(u8::from_le_bytes(value.try_into().unwrap()))
    }

    fn read_u16(&mut self, offset: u32, ctx: CpuContext) -> Result<u16> {
        let value = self.read(offset, 2, ctx)?;

        Ok(u16::from_le_bytes(value.try_into().unwrap()))
    }

    fn read_u32(&mut self, offset: u32, ctx: CpuContext) -> Result<u32> {
        let value = self.read(offset, 4, ctx)?;

        Ok(u32::from_le_bytes(value.try_into().unwrap()))
    }

    fn write_u8(&mut self, offset: u32, value: u8, ctx: CpuContext) -> Result<()> {
        self.write(offset, &value.to_le_bytes(), ctx)
    }

    fn write_u16(&mut self, offset: u32, value: u16, ctx: CpuContext) -> Result<()> {
        self.write(offset, &value.to_le_bytes(), ctx)
    }

    fn write_u32(&mut self, offset: u32, value: u32, ctx: CpuContext) -> Result<()> {
        self.write(offset, &value.to_le_bytes(), ctx)
    }
}

pub struct Bus {
    memory: Memory,

    clint: Clint,
    uart: Uart,
    plic: Plic,
}

impl Default for Bus {
    fn default() -> Self {
        let memory = Memory::default();
        let clint = Clint::default();
        let uart = Uart::default();
        let plic = Plic::default();

        Self {
            memory,
            clint,
            uart,
            plic,
        }
    }
}

macro_rules! read {
    ($fn:ident, $t:ident) => {
        pub fn $fn(&mut self, addr: u32, ctx: CpuContext) -> Result<$t> {
            match addr {
                CLINT_BASE..CLINT_END => self.clint.$fn(addr - CLINT_BASE, ctx),
                UART_BASE..UART_END => self.uart.$fn(addr - UART_BASE, ctx),
                PLIC_BASE..PLIC_END => self.plic.$fn(addr - PLIC_BASE, ctx),
                MEMORY_BASE..MEMORY_END => self.memory.$fn(addr - MEMORY_BASE, ctx),
                //UART_BASE..UART_END => {
                //    self.uart.read(offset, size, ctx.is_walk, ctx.access_type)
                //},
                _ => Err(ctx.make_trap()),
            }
        }
    };
}

macro_rules! write {
    ($fn:ident, $t:ident) => {
        pub fn $fn(&mut self, addr: u32, value: $t, ctx: CpuContext) -> Result<()> {
            match addr {
                CLINT_BASE..CLINT_END => self.clint.$fn(addr - CLINT_BASE, value, ctx),
                UART_BASE..UART_END => self.uart.$fn(addr - UART_BASE, value, ctx),
                PLIC_BASE..PLIC_END => self.plic.$fn(addr - PLIC_BASE, value, ctx),
                MEMORY_BASE..MEMORY_END => self.memory.$fn(addr - MEMORY_BASE, value, ctx),
                //UART_BASE..UART_END => {
                //    self.uart.read(offset, size, ctx.is_walk, ctx.access_type)
                //},
                _ => Err(ctx.make_trap()),
            }
        }
    };
}

impl Bus {
    read!(read_u8, u8);
    read!(read_u16, u16);
    read!(read_u32, u32);

    write!(write_u8, u8);
    write!(write_u16, u16);
    write!(write_u32, u32);

    pub fn memory(&mut self) -> &mut Memory {
        &mut self.memory
    }

    pub fn uart(&mut self) -> &mut Uart {
        &mut self.uart
    }

    pub fn plic(&mut self) -> &mut Plic {
        &mut self.plic
    }
}
