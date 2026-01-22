use std::io::Write;

use crate::{Result, Trap, cpu::Cpu};

#[derive(PartialEq, Eq, Debug)]
pub enum MemoryTarget {
    Memory,
    Uart,
    Clint,
}

impl Cpu {
    #[inline]
    pub fn resovle_region(&self, pa: u32) -> Option<MemoryTarget> {
        match pa {
            0x2000000..=0x2010000 => Some(MemoryTarget::Clint),
            0x10000000..=0x10000100 => Some(MemoryTarget::Uart),
            0x80000000..=0x90000000 => Some(MemoryTarget::Memory),
            _ => None,
        }
    }

    #[inline]
    pub fn access_memory_read<const SIZE: usize>(&self, pa: u32) -> Result<[u8; SIZE]> {
        if let Some(target) = self.resovle_region(pa) {
            if target == MemoryTarget::Memory {
                self.memory.read(pa)
            } else {
                let value = match target {
                    MemoryTarget::Uart => self.handle_uart_read(pa),
                    MemoryTarget::Clint => self.handle_clint_read(pa),
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
                MemoryTarget::Uart => self.handle_uart_write(pa, value),
                MemoryTarget::Clint => self.handle_clint_write(pa, value),
                MemoryTarget::Memory => self.memory.write(pa, buf),
            }
        } else {
            Err(Trap::StoreOrAMOAccessFault)
        }
    }

    fn handle_uart_read(&self, address: u32) -> Result<u32> {
        let offset = address & 0xFF;

        if offset == 5 {
            return Ok(0x60);
        }

        Ok(0)
    }

    #[inline]
    fn handle_uart_write(&self, address: u32, value: u32) -> Result<()> {
        let offset = address & 0xFF;

        if offset == 0 {
            let c = value as u8;
            print!("{}", c as char);
        }

        Ok(())
    }

    #[inline]
    fn handle_clint_read(&self, pa: u32) -> Result<u32> {
        let offset = pa - 0x2000000;

        match offset {
            0 => Ok(self.csr.get_mip_msip()),
            _ => Err(Trap::LoadAccessFault),
        }
    }

    #[inline]
    fn handle_clint_write(&mut self, pa: u32, value: u32) -> Result<()> {
        let offset = pa - 0x2000000;

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
