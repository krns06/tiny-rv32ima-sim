use std::ops::Range;

pub mod cpu;
mod csr;
mod elf;
mod memory;
mod sbi;
mod uart;

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
}

impl From<AccessType> for Trap {
    #[inline]
    fn from(value: AccessType) -> Self {
        match value {
            AccessType::Read => Trap::LoadPageFault,
            AccessType::Write => Trap::StoreOrAMOPageFault,
            AccessType::Fetch => Trap::InstructionPageFault,
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

//[todo] read/write_memoryの修正後に取る値を適切な型に変更する
pub trait Device {
    fn get_range(&self) -> Range<u32>;

    fn load(&self, address: usize) -> Result<Option<Vec<u8>>>;
    fn store(&mut self, address: usize, value: &[u8]) -> Result<()>;
}

#[macro_export]
macro_rules! illegal {
    () => {
        return Err(Trap::IlligalInstruction)
    };
}

pub type Result<T> = std::result::Result<T, Trap>;

// [todo] 削除する
#[inline]
pub const fn into_addr(x: u32) -> usize {
    x as usize
}
