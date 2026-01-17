use crate::Device;

pub struct Uart;

impl Device for Uart {
    #[inline]
    fn get_range(&self) -> std::ops::Range<u32> {
        0x10000000..0x10000100
    }

    #[inline]
    fn load(&self, address: usize) -> crate::Result<Option<Vec<u8>>> {
        let offset = address & 0xFF;

        if offset == 5 {
            return Ok(Some(vec![0x60]));
        }

        Ok(Some(vec![0]))
    }

    #[inline]
    fn store(&mut self, address: usize, value: &[u8]) -> crate::Result<()> {
        let offset = address & 0xFF;

        if offset == 0 {
            let c = value[0];
            print!("{}", c as char);
        }

        Ok(())
    }
}
