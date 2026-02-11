mod bus;
mod cpu;
mod csr;
mod device;
mod elf;
mod memory;
pub mod simulator;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AccessType {
    Read = 1 << 1,
    Write = 1 << 2,
    Fetch = 1 << 3,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priv {
    User = 0,
    Supervisor = 1,
    Machine = 3,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IRQ {
    None = 0,
    VirtioNet = 1,
    VirtioGpu = 2,
    Uart = 0xa,
}

impl From<usize> for IRQ {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::None,
            1 => Self::VirtioNet,
            2 => Self::VirtioGpu,
            0xa => Self::Uart,
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! illegal {
    () => {
        return Err(Trap::IlligalInstruction)
    };
}

pub type Result<T> = std::result::Result<T, Trap>;
