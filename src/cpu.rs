use std::fmt::Display;

use crate::{
    Exception, Priv, Result,
    csr::Csr,
    illegal, into_addr,
    memory::{MEMORY_SIZE, Memory},
};

// デバッグ用マクロ
macro_rules! unimplemented {
    () => {
        return Err(Exception::UnimplementedInstruction)
    };
}

// read/write関数以外では操作してはいけない。
#[derive(Default)]
pub struct Registers {
    regs: [u32; 32],
}

impl Display for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, reg) in self.regs.iter().enumerate() {
            f.write_str(&format!("[{:02}]: 0x{:08x}", idx, reg))?;
        }

        Ok(())
    }
}

impl Registers {
    #[inline]
    pub fn read(&self, reg: u32) -> u32 {
        let reg = reg as usize;

        self.regs[reg]
    }

    #[inline]
    pub fn write(&mut self, reg: u32, value: u32) {
        let reg = reg as usize;

        if reg == 0 {
            return;
        } else {
            self.regs[reg] = value;
        }
    }
}

pub struct Cpu {
    prv: Priv, // privは予約済みらしい
    regs: Registers,
    pc: u32, // 当面はVirtual Address想定

    // 現在実行中の命令列
    inst: u32,

    memory: Memory,
    csr: Csr,

    is_debug: bool,
    riscv_tests_exit_memory_address: u32,
    riscv_tests_finished: bool,
}

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("---------- DUMP ----------")?;

        f.write_str(&format!("PC  : 0x{:08x}", self.pc))?;
        f.write_str(&format!("Priv: {:?}", self.prv))?;
        f.write_str(&format!("{}", self.regs))?;

        let opcode = self.inst & 0x7f;
        let rd = (self.inst >> 7) & 0x1f;
        let rs1 = (self.inst >> 15) & 0x1f;
        let rs2 = (self.inst >> 20) & 0x1f;
        let funct3 = (self.inst >> 12) & 0x7;
        f.write_str(&format!("inst: 0x{:08x}", self.inst))?;
        f.write_str(&format!(
            "opcode: 0b{:07b} funct3: 0b{:03b}",
            opcode, funct3
        ))?;
        f.write_str(&format!(
            "rd: 0b{:05b} rs1: 0b{:05b} rs2: 0b{:05b}",
            rd, rs1, rs2
        ))?;

        f.write_str(&format!("{:x?}", self.csr))?;

        f.write_str("---------- DUMP END ----------")
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            prv: Priv::Machine,
            regs: Registers::default(),
            pc: 0,
            inst: 0,
            memory: Memory::new(),
            csr: Csr::default(),
            is_debug: false,
            riscv_tests_exit_memory_address: 0,
            riscv_tests_finished: false,
        }
    }

    fn read_reg(&self, reg: u32) -> u32 {
        self.regs.read(reg)
    }

    fn write_reg(&mut self, reg: u32, value: u32) {
        self.regs.write(reg, value)
    }

    // memory読み込みのラッパー関数
    // 将来的にはVirtual Adressとかその他の例外を実装するためにこのようにする。
    #[inline]
    pub fn read_memory<const SIZE: usize>(&self, address: u32) -> Result<[u8; SIZE]> {
        self.memory.read(address)
    }

    #[inline]
    pub fn write_memory<const SIZE: usize>(
        &mut self,
        address: u32,
        buf: &[u8; SIZE],
    ) -> Result<()> {
        if address == self.riscv_tests_exit_memory_address {
            self.riscv_tests_finished = true;
        }

        self.memory.write(address, buf)
    }

    #[inline]
    pub fn read_csr(&self, csr: u32) -> Result<u32> {
        self.csr.read(csr, self.prv)
    }

    #[inline]
    pub fn write_csr(&mut self, csr: u32, value: u32) -> Result<()> {
        self.csr.write(csr, value, self.prv)
    }

    pub fn run(&mut self) {}

    // riscv-testsが通るか検証する関数
    // true: 成功 false: 失敗
    pub fn debug_run(&mut self, riscv_tests_exit_memory_address: u32) -> bool {
        self.is_debug = true;
        self.riscv_tests_exit_memory_address = riscv_tests_exit_memory_address;

        loop {
            if self.riscv_tests_finished {
                break;
            }

            println!("[info]: PC 0x{:08x}", self.pc);
            match self.step() {
                Err(e) => self.handle_exception(e),
                Ok(is_jump) => {
                    if !is_jump {
                        // JUMP系の命令でない場合にPCを更新する。
                        self.pc += 4;
                    }
                }
            }
        }

        let address = self.riscv_tests_exit_memory_address;
        let bytes = (self.memory.raw_read(into_addr(address)));

        bytes == [1, 0, 0, 0]
    }

    // jump命令: Ok(true) 他の命令: Ok(false)
    #[inline]
    pub fn step(&mut self) -> Result<bool> {
        macro_rules! reg {
            ($reg:expr) => {
                self.read_reg($reg)
            };
            ($reg:expr, $value:expr) => {
                self.write_reg($reg, $value)
            };
        }

        self.inst = self.fetch()?;

        let opcode = self.inst & 0x7f;
        let rd = (self.inst >> 7) & 0x1f;
        let rs1 = (self.inst >> 15) & 0x1f;
        let rs2 = (self.inst >> 20) & 0x1f;
        let funct3 = (self.inst >> 12) & 0x7;

        let mut is_jump = false;

        match opcode {
            0b0001111 => match funct3 {
                0 => {
                    match self.inst {
                        0x8330000f | 0x0100000f => unimplemented!(),
                        _ => {
                            // FENCE
                            // キャッシュはまだ実装しないのでなにも行わない。
                        }
                    }
                }
                _ => unimplemented!(),
            },
            0b0010011 => {
                match funct3 {
                    0b000 => {
                        // ADDI
                        let imm = ((self.inst as i32) >> 20) as u32;
                        reg!(rd, reg!(rs1).wrapping_add(imm))
                    }
                    0b001 => {
                        // SLLI
                        let imm = self.inst >> 20;
                        if imm >> 5 != 0 {
                            illegal!();
                        }

                        reg!(rd, reg!(rs1) << ((imm as u32) & 0x1f))
                    }
                    0b110 => {
                        // ORI
                        let imm = ((self.inst as i32) >> 20) as u32;
                        reg!(rd, reg!(rs1) | imm)
                    }
                    _ => unimplemented!(),
                }
            }
            0b0010111 => reg!(rd, self.pc.wrapping_add(self.inst & 0xfffff000)), // AUIPC
            0b0100011 => {
                match funct3 {
                    0b010 => {
                        // SW
                        let imm = ((self.inst >> (25 - 5)) & 0xfe0) | ((self.inst >> 7) & 0x1f);
                        let imm = (((imm << 20) as i32) >> 20) as u32;

                        let bytes = reg!(rs2).to_le_bytes();

                        self.write_memory(reg!(rs1).wrapping_add(imm), &bytes)?;
                    }
                    _ => unimplemented!(),
                }
            }
            0b0110011 => {
                let funct7 = self.inst >> 25;

                match (funct3, funct7) {
                    (0b000, 0b0000000) => reg!(rd, reg!(rs1).wrapping_add(reg!(rs2))), // ADD
                    _ => unimplemented!(),
                }
            }
            0b0110111 => reg!(rd, self.inst & 0xfffff000), // LUI
            0b1100011 => {
                let imm = ((self.inst >> 19) & 0x1000)
                    | ((self.inst << 4) & 0x800)
                    | ((self.inst >> 20) & 0x7e0)
                    | ((self.inst >> 7) & 0x1e);
                let imm = (((imm << 19) as i32) >> 19) as u32;
                let flag = match funct3 {
                    0b000 => reg!(rs1) == reg!(rs2),              // BEQ
                    0b001 => reg!(rs1) != reg!(rs2),              // BNE
                    0b100 => reg!(rs2) as i32 > reg!(rs1) as i32, //BLT
                    _ => unimplemented!(),
                };

                if flag {
                    self.pc = self.pc.wrapping_add(imm);
                    is_jump = true;
                }
            }
            0b1100111 => {
                //JALR
                if (funct3 != 0) {
                    // funct3の検証
                    // これは検証すべきかはわからない。
                    // tinyemuでは無視してた。
                    unimplemented!();
                }

                let pc = self.pc;
                let imm = (self.inst as i32) >> 20;

                self.pc = (imm as u32).wrapping_add(reg!(rs1)) & !1;

                reg!(rd, pc + 4);

                is_jump = true;
            }
            0b1101111 => {
                // JAL
                let imm = ((self.inst >> (31 - 20)) & (1 << 20))
                    | ((self.inst >> (21 - 1)) & 0x7fe)
                    | ((self.inst >> (20 - 11)) & (1 << 11))
                    | (self.inst & 0xff000);

                let imm = ((imm << 11) as i32) >> 11;
                let pc = self.pc;

                self.pc = pc.wrapping_add(imm as u32);
                reg!(rd, pc + 4);

                is_jump = true;
            }
            0b1110011 => {
                //[todo] valueについてはもっと綺麗に描けるかも
                let csr = self.inst >> 20;

                match funct3 {
                    0b001 => {
                        // CSRRW
                        if rd != 0 {
                            let value = self.read_csr(csr)?;
                            reg!(rd, value);
                        }

                        self.write_csr(csr, reg!(rs1))?;
                    }
                    0b010 => {
                        // CSRRS
                        let value = self.read_csr(csr)?;

                        reg!(rd, value);

                        if rs1 != 0 {
                            self.write_csr(csr, value | reg!(rs1))?;
                        }
                    }
                    0b101 => {
                        // CSRRWI
                        if rd != 0 {
                            let value = self.read_csr(csr)?;
                            reg!(rd, value);
                        }

                        self.write_csr(csr, rs1)?;
                    }
                    _ => match self.inst {
                        0x00000073 => {
                            // ECALL
                            match self.prv {
                                Priv::Machine => return Err(Exception::EnvCallFromMachine),
                                Priv::User => return Err(Exception::EnvCallFromUser),
                                _ => panic!("[ERROR]: Ecall from machine mode is only valid."),
                            }
                        }
                        0x30200073 => {
                            // MRET

                            if self.prv != Priv::Machine {
                                illegal!();
                            }

                            let mpp = self.csr.handle_mret()?;

                            self.prv = mpp.into();
                            self.pc = self.csr.mepc;
                        }
                        _ => unimplemented!(),
                    },
                }
            }
            _ => unimplemented!(),
        }

        Ok(is_jump)
    }

    #[inline]
    fn fetch(&mut self) -> Result<u32> {
        if self.pc % 4 == 0 {
            let buf = self.memory.raw_read(into_addr(self.pc));

            Ok(u32::from_le_bytes(buf))
        } else {
            Err(Exception::InstructionAddressMisaligned)
        }
    }

    #[inline]
    pub fn handle_exception(&mut self, e: Exception) {
        self.prv = Priv::Machine;
        self.csr.mcause = e as u32;
        // mepcはIALIGN==32のみサポートの場合は[0..1]は０
        //[todo] MMU実装時に仮想アドレスを表すものに変更する。
        self.csr.mepc = self.pc & !0x3;

        self.csr.mtval = if e == Exception::IlligalInstruction {
            self.inst
        } else {
            // 今はpcは仮想アドレス想定(実質物理アドレス)だが
            //[todo] MMU実装したらここは仮想アドレスに変更する。
            self.pc
        };

        let mode = self.csr.mtvec & 0x3;
        let base = self.csr.mtvec & !0x3;

        if mode == 0 {
            // Direct
            // 同期例外は確定でこっちらしい
            self.pc = base;
        } else {
            // Vectored
            panic!("[ERROR]: Vectored mode of mtvec is not implemented.");
        }

        if e == Exception::UnimplementedCSR || e == Exception::UnimplementedInstruction {
            println!("{:?}", e);
            panic!("{}", self);
        }
    }

    pub fn load_flat_program(&mut self, code: &[u8]) {
        if code.len() > MEMORY_SIZE {
            panic!("[Error]: the program is too big.");
        }

        self.memory.array.copy_from_slice(code);
    }

    // [todo] lazy_load_flat_program
    // [todo] load_elf_program
}
