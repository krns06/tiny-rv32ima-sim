use crate::bus::MmioOps;

#[derive(Default)]
pub struct Clint {}

impl MmioOps for Clint {
    #[inline]
    fn read(&mut self, _: u32, _: u32, _: crate::bus::CpuContext) -> crate::Result<Vec<u8>> {
        unreachable!();
    }

    #[inline]
    fn write(&mut self, _: u32, _: &[u8], _: crate::bus::CpuContext) -> crate::Result<()> {
        unreachable!()
    }

    #[inline]
    fn read_u32(&mut self, offset: u32, ctx: crate::bus::CpuContext) -> crate::Result<u32> {
        match offset {
            0 => Ok(ctx.csr.get_mip_msip()),
            _ => Err(ctx.make_trap()),
        }
    }

    #[inline]
    fn write_u32(
        &mut self,
        offset: u32,
        value: u32,
        ctx: crate::bus::CpuContext,
    ) -> crate::Result<()> {
        match offset {
            0 => {
                // msip
                let msip = value & 0x1;
                ctx.csr.set_mip_msip(msip);
            }
            0x4000 => {
                // mtimecmp
                ctx.csr.set_mtimecmp(value);
            }
            0x4004 => {
                // mtimecmph
                ctx.csr.set_mtimecmph(value);
            }
            _ => return Err(ctx.make_trap()),
        }

        Ok(())
    }
}
