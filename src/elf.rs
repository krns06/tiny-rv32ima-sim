type Elf32Half = u16;
type Elf32Word = u32;
type Elf32Addr = u32;
type Elf32Off = u32;

const EI_NIDENT: usize = 16;
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

const PT_LOAD: Elf32Word = 1;

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Elf32Ehdr {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: Elf32Half,
    pub e_machine: Elf32Half,
    pub e_version: Elf32Word,
    pub e_entry: Elf32Addr,
    pub e_phoff: Elf32Off,
    pub e_shoff: Elf32Off,
    pub e_flags: Elf32Word,
    pub e_ehsize: Elf32Half,
    pub e_phentsize: Elf32Half,
    pub e_phnum: Elf32Half,
    pub e_shentsize: Elf32Half,
    pub e_shnum: Elf32Half,
    pub e_shstrndx: Elf32Half,
}

impl Elf32Ehdr {
    pub fn is_valid(&self) -> bool {
        self.e_ident.starts_with(&ELF_MAGIC) && self.e_ident[4] == 1 /* ELFCLASS32 */
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Elf32Phdr {
    pub p_type: Elf32Word,
    pub p_offset: Elf32Off,
    pub p_vaddr: Elf32Addr,
    pub p_paddr: Elf32Addr,
    pub p_filesz: Elf32Word,
    pub p_memsz: Elf32Word,
    pub p_flags: Elf32Word,
    pub p_align: Elf32Word,
}

impl Elf32Phdr {
    pub fn is_load_seg(&self) -> bool {
        self.p_type == PT_LOAD
    }
}
