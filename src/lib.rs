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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    InstructionAddressMisaligned = 0,
    IlligalInstruction = 2,
    EnvCallFromUser = 8,
    EnvCallFromMachine = 11,
    LoadAccessFault = 13,

    UnimplementedInstruction, // デバッグ用
    UnimplementedCSR,         // デバッグ用
}

#[macro_export]
macro_rules! illegal {
    () => {
        return Err(Exception::IlligalInstruction)
    };
}

pub type Result<T> = std::result::Result<T, Exception>;

#[inline]
pub const fn into_addr(x: u32) -> usize {
    x as usize
}
