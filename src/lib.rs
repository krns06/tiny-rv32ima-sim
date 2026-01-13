pub mod cpu;
mod csr;
mod memory;
mod simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priv {
    User = 0,
    Supervisor = 1,
    Machine = 3,
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

// [todo]; handle_trap関数実装時にExceptionから名前を変更する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trap {
    InstructionAddressMisaligned = 0,
    IlligalInstruction = 2,
    LoadAddressMisaligned = 4,
    StoreOrAMOAddressMisaligned = 6,
    EnvCallFromUser = 8,
    EnvCallFromSupervisor = 9,
    EnvCallFromMachine = 11,
    LoadAccessFault = 13,

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

#[macro_export]
macro_rules! illegal {
    () => {
        return Err(Trap::IlligalInstruction)
    };
}

pub type Result<T> = std::result::Result<T, Trap>;

#[inline]
pub const fn into_addr(x: u32) -> usize {
    x as usize
}
