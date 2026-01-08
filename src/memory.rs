use crate::{Exception, Result, into_addr};

pub const MEMORY_SIZE: usize = 1024 * 1024;

pub struct Memory {
    pub array: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            array: vec![0; MEMORY_SIZE],
        }
    }

    #[inline]
    pub fn raw_read<const SIZE: usize>(&self, address: usize) -> [u8; SIZE] {
        let mut buf = [0; SIZE];

        buf.copy_from_slice(&self.array[address..address + SIZE]);

        buf
    }

    #[inline]
    pub fn raw_write<const SIZE: usize>(&mut self, address: usize, buf: &[u8; SIZE]) -> () {
        self.array[address..address + SIZE].copy_from_slice(buf);

        ()
    }

    #[inline]
    pub fn read<const SIZE: usize>(&self, address: u32) -> Result<[u8; SIZE]> {
        let address = into_addr(address);

        // riscvの仕様書ではvacant address spaceは例外を起こしていいそうなので起こしている。
        if address + SIZE >= MEMORY_SIZE {
            return Err(Exception::LoadAccessFault);
        }

        Ok(self.raw_read(address))
    }

    #[inline]
    pub fn write<const SIZE: usize>(&mut self, address: u32, buf: &[u8; SIZE]) -> Result<()> {
        let address = into_addr(address);

        // riscvの仕様書ではvacant address spaceは例外を起こしていいそうなので起こしている。
        if address + SIZE >= MEMORY_SIZE {
            return Err(Exception::LoadAccessFault);
        }

        Ok(self.raw_write(address, buf))
    }
}
