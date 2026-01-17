use crate::{
    AccessType, Result, Trap,
    elf::{Elf32Ehdr, Elf32Phdr},
    into_addr,
};

pub const MEMORY_SIZE: usize = 1024 * 1024 * 128;

#[derive(Default)]
pub struct Memory {
    pub array: Vec<u8>,
    pub base_address: u32,
}

impl Memory {
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

    //#[inline]
    //pub fn write_for_translation<const SIZE: usize>(
    //    &mut self,
    //    address: u32,
    //    buf: &[u8; SIZE],
    //    access_type: AccessType,
    //) -> Result<()> {
    //    let address = into_addr(address);

    //    if self.is_invalid_range(address, SIZE) {
    //        return Err(access_type.into());
    //    }

    //    Ok(self.raw_write(address, buf))
    //}

    pub fn load_flat_binary<const SIZE: usize>(&mut self, buf: &[u8; SIZE], address: usize) {
        if SIZE > MEMORY_SIZE {
            panic!("[Error]: the program is too big.");
        }

        self.raw_write(address, buf);
    }

    // [todo] lazy_load_flat_program

    pub fn load_elf_binary(&mut self, array: &[u8]) -> u32 {
        let ehdr_size = core::mem::size_of::<Elf32Ehdr>();
        let ehdr = unsafe { *(&array[..ehdr_size] as *const _ as *const Elf32Ehdr) };

        if !ehdr.is_valid() {
            panic!("invalid ELF32");
        }

        let phnum = ehdr.e_phnum as usize;
        let phoff = ehdr.e_phoff as usize;
        let phentsize = ehdr.e_phentsize as usize;
        let phdr_size = core::mem::size_of::<Elf32Phdr>();

        for i in 0..phnum {
            let offset = phoff + i * phentsize;
            let phdr: Elf32Phdr =
                unsafe { *(&array[offset..offset + phdr_size] as *const _ as *const Elf32Phdr) };

            if !phdr.is_load_seg() {
                continue;
            }

            let file_off = phdr.p_offset as usize;
            let file_end = file_off + phdr.p_filesz as usize;

            let mem_addr = (phdr.p_paddr as u32 - self.base_address) as usize;
            let mem_end = mem_addr + phdr.p_filesz as usize;

            self.array[mem_addr..mem_end].copy_from_slice(&array[file_off..file_end]);

            let bss_end = mem_addr + phdr.p_memsz as usize;
            self.array[mem_end..bss_end].fill(0);
        }

        ehdr.e_entry
    }
}
