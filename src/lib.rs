use std::fmt::Debug;

mod bus;
mod cpu;
mod csr;
mod elf;
mod host_device;
mod memory;
mod native;
mod plic;
pub mod simulator;
mod tlb;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub type Result<T> = std::result::Result<T, Trap>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AccessType {
    Read = 1 << 1,
    Write = 1 << 2,
    Fetch = 1 << 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priv {
    User = 0,
    Supervisor = 1,
    Machine = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trap {
    InstructionAddressMisaligned = 0,
    IlligalInstruction = 2,
    BreakPoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreOrAMOAddressMisaligned = 6,
    StoreOrAMOAccessFault = 7,
    EnvCallFromUser = 8,
    EnvCallFromSupervisor = 9,
    EnvCallFromMachine = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StoreOrAMOPageFault = 15,

    SupervisorSoftwareInterrupt = 1 << 31 | 1,
    SupervisorTimerInterrupt = 1 << 31 | 5,
    SupervisorExternalInterrupt = 1 << 31 | 9,

    UnimplementedInstruction, // デバッグ用
    UnimplementedCSR,         // デバッグ用
}

#[derive(Debug, PartialEq, Eq)]
pub enum GpuOperation {
    Copy,
    Disable,
    Flush,
}

#[derive(Debug, Default)]
pub struct GpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct GpuMessage {
    pub operation: GpuOperation,
    pub resource_id: u32,
    pub rect: GpuRect,
    pub buffer: Vec<u32>,
}

// 仮想デバイスとホストデバイスとの通信に使用する列挙体
pub enum DeviceMessage {
    Uart(char),
    Net(Vec<u8>),
    Gpu(GpuMessage),
    Blk(Vec<u8>),
    None,
}

// 仮想デバイスからホストデバイスに対してのsenderに関してのトレイト
pub trait DeviceSenderTrait: Default {
    type E: Debug;

    fn send_to_host(&mut self, message: DeviceMessage) -> std::result::Result<(), Self::E>;
}

// 仮想デバイスがホストデバイスからのメッセージを受け取るためのトレイト
pub trait DeviceRecieverTrait: Default {
    type E: Debug;

    fn try_recv_from_host(&self) -> std::result::Result<DeviceMessage, Self::E>;
}

#[macro_export]
macro_rules! illegal {
    () => {
        return Err(Trap::IlligalInstruction)
    };
}

impl AccessType {
    #[inline]
    pub fn is_read(&self) -> bool {
        *self == Self::Read
    }

    #[inline]
    pub fn is_write(&self) -> bool {
        *self == Self::Write
    }

    #[inline]
    pub fn is_exec(&self) -> bool {
        *self == Self::Fetch
    }

    #[inline]
    pub fn into_trap(&self, is_walk: bool) -> Trap {
        if is_walk {
            match self {
                Self::Fetch => Trap::InstructionPageFault,
                Self::Read => Trap::LoadPageFault,
                Self::Write => Trap::StoreOrAMOPageFault,
            }
        } else {
            match self {
                Self::Fetch => todo!(),
                Self::Read => Trap::LoadAccessFault,
                Self::Write => Trap::StoreOrAMOAccessFault,
            }
        }
    }
}

impl Default for Priv {
    fn default() -> Self {
        Priv::Machine
    }
}

impl From<u32> for Priv {
    fn from(value: u32) -> Self {
        match value {
            0 => Priv::User,
            1 => Priv::Supervisor,
            3 => Priv::Machine,
            _ => unreachable!(),
        }
    }
}

impl Trap {
    pub fn is_interrupt(&self) -> bool {
        if (*self as u32) >> 31 == 1 {
            true
        } else {
            false
        }
    }

    pub fn cause(&self) -> u32 {
        (*self as u32) & !(1 << 31)
    }
}

impl GpuRect {
    fn start(&self) -> usize {
        (self.x + self.y * self.width) as usize
    }

    fn end(&self) -> usize {
        self.start() + (self.width * self.height) as usize
    }
}

impl GpuMessage {
    pub fn new(operation: GpuOperation, resource_id: u32) -> Self {
        Self {
            operation,
            resource_id,
            rect: GpuRect::default(),
            buffer: Vec::new(),
        }
    }
}
