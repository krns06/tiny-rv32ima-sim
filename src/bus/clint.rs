use crate::{Result, csr::Csr};

#[derive(Default)]
pub struct Clint {}

impl Clint {
    #[inline]
    pub fn read(&mut self, offset: u32, size: u32, csr: &mut Csr) -> Result<u32> {
        if size != 4 {
            unimplemented!();
        }

        match offset {
            0 => Ok(csr.get_mip_msip()),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write(&mut self, offset: u32, size: u32, value: u32, csr: &mut Csr) -> Result<()> {
        if size != 4 {
            unimplemented!();
        }

        match offset {
            0 => {
                // msip
                let msip = value & 0x1;
                csr.set_mip_msip(msip);
            }
            0x4000 => {
                // mtimecmp
                csr.set_mtimecmp(value);
            }
            0x4004 => {
                // mtimecmph
                csr.set_mtimecmph(value);
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}
