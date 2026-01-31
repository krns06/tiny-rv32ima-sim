use crate::{
    Result,
    bus::{MEMORY_BASE, MmioOps},
    elf::{Elf32Ehdr, Elf32Phdr},
};

pub const MEMORY_SIZE: usize = 1024 * 1024 * 512;

pub struct Memory {
    pub array: Vec<u8>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            array: vec![0; MEMORY_SIZE],
        }
    }
}

impl MmioOps for Memory {
    #[inline]
    fn read(&mut self, offset: u32, size: u32, ctx: crate::bus::CpuContext) -> Result<Vec<u8>> {
        let offset = offset as usize;
        let size = size as usize;

        if self.is_invalid_range(offset, size) {
            return Err(ctx.make_trap());
        }

        Ok(self.raw_read(offset, size))
    }

    #[inline]
    fn write(&mut self, offset: u32, array: &[u8], ctx: crate::bus::CpuContext) -> Result<()> {
        let offset = offset as usize;
        let size = array.len();

        if self.is_invalid_range(offset, size) {
            return Err(ctx.make_trap());
        }

        Ok(self.raw_write(offset, array))
    }
}

impl Memory {
    #[inline]
    fn raw_read(&self, offset: usize, size: usize) -> Vec<u8> {
        let mut buf = vec![0; size];

        buf.copy_from_slice(&self.array[offset..offset + size]);

        buf
    }

    #[inline]
    fn raw_write(&mut self, offset: usize, array: &[u8]) -> () {
        self.array[offset..offset + array.len()].copy_from_slice(array);

        ()
    }

    fn is_invalid_range(&self, address: usize, size: usize) -> bool {
        let is_over_memory = address + size > MEMORY_SIZE;

        is_over_memory
    }

    pub fn load_flat_binary<const SIZE: usize>(&mut self, array: &[u8; SIZE], addr: u32) {
        let addr = addr as usize;

        if SIZE > MEMORY_SIZE {
            panic!("[Error]: the program is too big.");
        }

        let offset = addr - MEMORY_BASE as usize;

        self.raw_write(offset, array);
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

            let mem_addr = (phdr.p_paddr as u32 - MEMORY_BASE) as usize;
            let mem_end = mem_addr + phdr.p_filesz as usize;

            self.array[mem_addr..mem_end].copy_from_slice(&array[file_off..file_end]);

            let bss_end = mem_addr + phdr.p_memsz as usize;
            self.array[mem_end..bss_end].fill(0);
        }

        ehdr.e_entry
    }
}
