use crate::{Priv, Result, Trap, cpu::Cpu};

const CLINT_BASE: u32 = 0x2000000;
const CLINT_END: u32 = CLINT_BASE + 0x10000;

const PLIC_BASE: u32 = 0xc000000;
const PLIC_END: u32 = PLIC_BASE + 0x4000000;

const UART_BASE: u32 = 0x10000000;
const UART_END: u32 = UART_BASE + 0x100;

const MEMORY_BASE: u32 = 0x80000000;
const MEMORY_END: u32 = 0x90000000;

#[derive(PartialEq, Eq, Debug)]
pub enum MemoryTarget {
    MEMORY,
    UART,
    CLINT,
    PLIC,
}

impl Cpu {
    #[inline]
    pub fn resovle_region(&self, pa: u32) -> Option<MemoryTarget> {
        match pa {
            CLINT_BASE..CLINT_END => Some(MemoryTarget::CLINT),
            PLIC_BASE..PLIC_END => Some(MemoryTarget::PLIC),
            UART_BASE..UART_END => Some(MemoryTarget::UART),
            MEMORY_BASE..=MEMORY_END => Some(MemoryTarget::MEMORY),
            _ => None,
        }
    }

    #[inline]
    pub fn access_memory_read<const SIZE: usize>(&mut self, pa: u32) -> Result<[u8; SIZE]> {
        if let Some(target) = self.resovle_region(pa) {
            if target == MemoryTarget::MEMORY {
                self.memory.read(pa)
            } else {
                let value = match target {
                    MemoryTarget::UART => self.handle_uart_read(pa - UART_BASE),
                    MemoryTarget::CLINT => self.handle_clint_read(pa - CLINT_BASE),
                    MemoryTarget::PLIC => self.handle_plic_read(pa - PLIC_BASE),
                    _ => unreachable!(),
                }?;

                let mut t = [0; SIZE];
                t.copy_from_slice(&value.to_le_bytes()[..SIZE]);

                Ok(t)
            }
        } else {
            Err(Trap::LoadAccessFault)
        }
    }

    #[inline]
    pub fn access_memory_write<const SIZE: usize>(
        &mut self,
        pa: u32,
        buf: &[u8; SIZE],
    ) -> Result<()> {
        if let Some(target) = self.resovle_region(pa) {
            let mut t = [0; 4];
            t[..SIZE].copy_from_slice(buf);

            let value = u32::from_le_bytes(t);

            match target {
                MemoryTarget::UART => self.handle_uart_write(pa - UART_BASE, value),
                MemoryTarget::CLINT => self.handle_clint_write(pa - CLINT_BASE, value),
                MemoryTarget::PLIC => self.handle_plic_write(pa - PLIC_BASE, value),
                MemoryTarget::MEMORY => self.memory.write(pa, buf),
            }
        } else {
            Err(Trap::StoreOrAMOAccessFault)
        }
    }

    #[inline]
    fn handle_clint_read(&self, offset: u32) -> Result<u32> {
        match offset {
            0 => Ok(self.csr.get_mip_msip()),
            _ => Err(Trap::LoadAccessFault),
        }
    }

    #[inline]
    fn handle_clint_write(&mut self, offset: u32, value: u32) -> Result<()> {
        match offset {
            0 => {
                // msip
                let msip = value & 0x1;
                self.csr.set_mip_msip(msip);
            }
            0x4000 => {
                // mtimecmp
                self.csr.set_mtimecmp(value);
            }
            0x4004 => {
                // mtimecmph
                self.csr.set_mtimecmph(value);
            }
            _ => return Err(Trap::LoadAccessFault),
        }

        Ok(())
    }
}
