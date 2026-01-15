use crate::{AccessType, Result, Trap, into_addr};

pub const MEMORY_SIZE: usize = 1024 * 1024;

pub struct Memory {
    pub array: Vec<u8>,
    pub base_address: u32,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            array: vec![0; MEMORY_SIZE],
            base_address: 0,
        }
    }

    pub fn init(&mut self) {
        self.array.fill(0);
    }

    #[inline]
    pub fn raw_read<const SIZE: usize>(&self, address: usize) -> [u8; SIZE] {
        let address = address - self.base_address as usize;
        let mut buf = [0; SIZE];

        buf.copy_from_slice(&self.array[address..address + SIZE]);

        buf
    }

    #[inline]
    pub fn raw_write<const SIZE: usize>(&mut self, address: usize, buf: &[u8; SIZE]) -> () {
        let address = address - self.base_address as usize;
        self.array[address..address + SIZE].copy_from_slice(buf);

        ()
    }

    fn is_invalid_range(&self, address: usize, size: usize) -> bool {
        let is_over_memory = address + size > self.base_address as usize + MEMORY_SIZE;
        let is_under_memory = address < self.base_address as usize;

        is_over_memory || is_under_memory
    }

    #[inline]
    pub fn read<const SIZE: usize>(&self, address: u32) -> Result<[u8; SIZE]> {
        let address = into_addr(address);

        // riscvの仕様書ではvacant address spaceは例外を起こしていいそうなので起こしている。
        if self.is_invalid_range(address, SIZE) {
            return Err(Trap::LoadAccessFault);
        }

        Ok(self.raw_read(address))
    }

    #[inline]
    pub fn write<const SIZE: usize>(&mut self, address: u32, buf: &[u8; SIZE]) -> Result<()> {
        let address = into_addr(address);

        // riscvの仕様書ではvacant address spaceは例外を起こしていいそうなので起こしている。
        if self.is_invalid_range(address, SIZE) {
            return Err(Trap::StoreOrAMOAccessFault);
        }

        Ok(self.raw_write(address, buf))
    }

    #[inline]
    pub fn read_for_translation<const SIZE: usize>(
        &self,
        address: u32,
        access_type: AccessType,
    ) -> Result<[u8; SIZE]> {
        let address = into_addr(address);

        if self.is_invalid_range(address, SIZE) {
            return Err(access_type.into());
        }

        Ok(self.raw_read(address))
    }

    #[inline]
    pub fn write_for_translation<const SIZE: usize>(
        &mut self,
        address: u32,
        buf: &[u8; SIZE],
        access_type: AccessType,
    ) -> Result<()> {
        let address = into_addr(address);

        if self.is_invalid_range(address, SIZE) {
            return Err(access_type.into());
        }

        Ok(self.raw_write(address, buf))
    }
}
