use std::fmt::Display;

use crate::{
    Priv, Result, Trap,
    csr::Csr,
    illegal, into_addr,
    memory::{MEMORY_SIZE, Memory},
};

// デバッグ用マクロ
macro_rules! unimplemented {
    () => {
        return Err(Trap::UnimplementedInstruction)
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
            f.write_str(&format!("[{:02}]: 0x{:08x}\n", idx, reg))?;
        }

        Ok(())
    }
}

impl Registers {
    pub fn init(&mut self) {
        self.regs.fill(0);
    }

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

    reserved_address: Option<u32>, // For LR.W or SC.W

    is_debug: bool,
    riscv_tests_exit_memory_address: u32,
    riscv_tests_finished: bool,
}

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("---------- DUMP ----------\n")?;

        f.write_str(&format!("PC  : 0x{:08x}\n", self.pc))?;
        f.write_str(&format!("Priv: {:?}\n", self.prv))?;
        f.write_str(&format!("{}\n", self.regs))?;

        let opcode = self.inst & 0x7f;
        let rd = (self.inst >> 7) & 0x1f;
        let rs1 = (self.inst >> 15) & 0x1f;
        let rs2 = (self.inst >> 20) & 0x1f;
        let funct3 = (self.inst >> 12) & 0x7;
        let funct7 = self.inst >> 25;
        f.write_str(&format!("inst: 0x{:08x}\n", self.inst))?;
        f.write_str(&format!(
            "opcode: 0b{:07b} funct3: 0b{:03b} funct7: 0b{:07b}\n",
            opcode, funct3, funct7
        ))?;
        f.write_str(&format!(
            "rd: 0b{:05b} rs1: 0b{:05b} rs2: 0b{:05b}\n",
            rd, rs1, rs2
        ))?;

        f.write_str(&format!("{:x?}\n", self.csr))?;

        if self.is_debug {
            f.write_str(&format!(
                "[riscv_tests_value]: 0x{:08x}\n",
                u32::from_le_bytes(
                    self.memory
                        .raw_read(into_addr(self.riscv_tests_exit_memory_address))
                )
            ))?;
        }

        f.write_str("---------- DUMP END ----------\n")
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
            reserved_address: None,
            is_debug: false,
            riscv_tests_exit_memory_address: 0,
            riscv_tests_finished: false,
        }
    }

    pub fn init(&mut self) {
        self.prv = Priv::Machine;
        self.pc = 0;
        self.inst = 0;
        self.csr = Csr::default();
        self.is_debug = false;
        self.riscv_tests_exit_memory_address = 0;
        self.riscv_tests_finished = false;

        self.regs.init();
        self.memory.init();
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

            println!("[PC]: 0x{:08x}", self.pc);

            match self.step() {
                Err(e) => self.handle_trap(e),
                Ok(is_jump) => {
                    self.csr.progress_instret();

                    if let Some(e) = self.check_intrrupt_active() {
                        self.handle_trap(e);
                    } else if !is_jump {
                        // JUMP系の命令でない場合にPCを更新する。
                        self.pc += 4;
                    }
                }
            }

            self.csr.progress_cycle();
        }

        let address = self.riscv_tests_exit_memory_address;
        let bytes = self.memory.raw_read(into_addr(address));

        let flag = bytes == [1, 0, 0, 0];

        if !flag {
            println!("{}", self);
        }

        flag
    }

    // jump命令: Ok(true) 他の命令: Ok(false)
    // [todo] テストを通すためにテストで明示的に指定されるillegalな命令でillegal!を呼ぶが
    // テストが全て終わり、rv32imaの命令がすべて実装し終わったらunimplemented!をillegal!
    // に変更する。
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

        if self.inst == 0 {
            illegal!();
        }

        let opcode = self.inst & 0x7f;
        let rd = (self.inst >> 7) & 0x1f;
        let rs1 = (self.inst >> 15) & 0x1f;
        let rs2 = (self.inst >> 20) & 0x1f;
        let funct3 = (self.inst >> 12) & 0x7;

        let mut is_jump = false;

        match opcode {
            0b0000011 => {
                let imm = ((self.inst as i32) >> 20) as u32;

                match funct3 {
                    //[todo] refactor
                    0b000 => {
                        // LB

                        let byte = self.read_memory(reg!(rs1).wrapping_add(imm))?;

                        let value = u8::from_le_bytes(byte) as u32;
                        let value = (((value << 24) as i32) >> 24) as u32;

                        reg!(rd, value);
                    }
                    0b001 => {
                        // LH
                        let byte = self.read_memory(reg!(rs1).wrapping_add(imm))?;

                        let value = u16::from_le_bytes(byte) as u32;
                        let value = (((value << 16) as i32) >> 16) as u32;

                        reg!(rd, value);
                    }
                    0b010 => {
                        // LW
                        let bytes = self.read_memory(reg!(rs1).wrapping_add(imm))?;

                        let value = u32::from_le_bytes(bytes) as u32;

                        reg!(rd, value);
                    }
                    0b100 => {
                        // LBU
                        let byte = self.read_memory(reg!(rs1).wrapping_add(imm))?;

                        let value = u8::from_le_bytes(byte) as u32;

                        reg!(rd, value);
                    }
                    0b101 => {
                        // LHU
                        let byte = self.read_memory(reg!(rs1).wrapping_add(imm))?;

                        let value = u16::from_le_bytes(byte) as u32;

                        reg!(rd, value);
                    }
                    _ => unimplemented!(),
                }
            }
            0b0001111 => {
                match funct3 {
                    0 => match self.inst {
                        0x8330000f | 0x0100000f => illegal!(), // FENCE.TSO PAUSE
                        _ => {}                                // FENCE
                    },
                    1 => {} // FENCE.I
                    _ => unimplemented!(),
                }
            }
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
                    0b010 => {
                        // SLTI
                        let imm = (self.inst as i32) >> 20;
                        reg!(rd, if imm > reg!(rs1) as i32 { 1 } else { 0 })
                    }
                    0b011 => {
                        // SLTIU
                        let imm = ((self.inst as i32) >> 20) as u32;
                        reg!(rd, if imm > reg!(rs1) { 1 } else { 0 })
                    }
                    0b100 => {
                        // XORI
                        let imm = ((self.inst as i32) >> 20) as u32;
                        reg!(rd, reg!(rs1) ^ imm);
                    }
                    0b101 => {
                        let imm = (self.inst >> 20) & 0x1f;
                        let funct7 = self.inst >> 25;

                        match funct7 {
                            0b0000000 => reg!(rd, reg!(rs1) >> imm), // SRLI
                            0b0100000 => reg!(rd, ((reg!(rs1) as i32) >> imm) as u32), // SRAI
                            _ => unimplemented!(),
                        }
                    }
                    0b110 => {
                        // ORI
                        let imm = ((self.inst as i32) >> 20) as u32;
                        reg!(rd, reg!(rs1) | imm);
                    }
                    0b111 => {
                        // ANDI
                        let imm = ((self.inst as i32) >> 20) as u32;
                        reg!(rd, reg!(rs1) & imm);
                    }
                    _ => unimplemented!(),
                }
            }
            0b0010111 => reg!(rd, self.pc.wrapping_add(self.inst & 0xfffff000)), // AUIPC
            0b0100011 => {
                let imm = ((self.inst >> (25 - 5)) & 0xfe0) | ((self.inst >> 7) & 0x1f);
                let imm = (((imm << 20) as i32) >> 20) as u32;
                let address = reg!(rs1).wrapping_add(imm);

                match funct3 {
                    //[todo] refactor
                    0b000 => {
                        //SB
                        let bytes = (reg!(rs2) as u8).to_le_bytes();

                        self.write_memory(address, &bytes)?;
                    }
                    0b001 => {
                        // SH
                        let bytes = (reg!(rs2) as u16).to_le_bytes();

                        self.write_memory(address, &bytes)?;
                    }
                    0b010 => {
                        // SW
                        let bytes = reg!(rs2).to_le_bytes();

                        self.write_memory(address, &bytes)?;
                    }
                    _ => unimplemented!(),
                }
            }
            0b0110011 => {
                let funct7 = self.inst >> 25;

                match (funct3, funct7) {
                    (0b000, 0b0000000) => reg!(rd, reg!(rs1).wrapping_add(reg!(rs2))), // ADD
                    (0b000, 0b0000001) => reg!(rd, reg!(rs1).wrapping_mul(reg!(rs2))), // MUL
                    (0b000, 0b0100000) => reg!(rd, reg!(rs1).wrapping_sub(reg!(rs2))), // SUB
                    (0b001, 0b0000000) => reg!(rd, reg!(rs1) << (reg!(rs2) & 0x1f)),   // SLL
                    (0b001, 0b0000001) => {
                        // MULH
                        let rs1_value = (((reg!(rs1) as u64) << 32) as i64) >> 32;
                        let rs2_value = (((reg!(rs2) as u64) << 32) as i64) >> 32;

                        reg!(rd, ((rs1_value * rs2_value) >> 32) as u32);
                    }
                    (0b010, 0b0000000) => {
                        reg!(
                            rd,
                            if (reg!(rs1) as i32) < (reg!(rs2) as i32) {
                                1
                            } else {
                                0
                            }
                        )
                    } // SLT
                    (0b010, 0b0000001) => {
                        // MULHSU
                        let rs1_value = ((((reg!(rs1) as u64) << 32) as i64) >> 32) as u64;
                        let rs2_value = reg!(rs2) as u64;

                        reg!(rd, (rs1_value.wrapping_mul(rs2_value) >> 32) as u32);
                    }
                    (0b011, 0b0000000) => reg!(rd, if reg!(rs1) < reg!(rs2) { 1 } else { 0 }), // SLTU
                    (0b011, 0b0000001) => {
                        // MULHU
                        let rs1_value = reg!(rs1) as u64;
                        let rs2_value = reg!(rs2) as u64;

                        reg!(rd, (rs1_value.wrapping_mul(rs2_value) >> 32) as u32);
                    }
                    (0b100, 0b0000000) => reg!(rd, reg!(rs1) ^ reg!(rs2)), // XOR
                    (0b100, 0b0000001) => {
                        // DIV
                        let rs1_value = reg!(rs1);
                        let rs2_value = reg!(rs2);

                        let value = if rs1_value == 1 << 31 && rs2_value == !0 {
                            rs1_value
                        } else if rs2_value == 0 {
                            u32::MAX
                        } else {
                            (rs1_value as i32 / rs2_value as i32) as u32
                        };

                        reg!(rd, value);
                    }
                    (0b101, 0b0000000) => reg!(rd, reg!(rs1) >> (reg!(rs2) & 0x1f)), // SRL
                    (0b101, 0b0000001) => {
                        // DIVU
                        let rs1_value = reg!(rs1);
                        let rs2_value = reg!(rs2);

                        reg!(
                            rd,
                            if rs2_value == 0 {
                                u32::MAX
                            } else {
                                rs1_value / rs2_value
                            }
                        );
                    }
                    (0b101, 0b0100000) => {
                        reg!(rd, ((reg!(rs1) as i32) >> (reg!(rs2) & 0x1f)) as u32)
                    } // SRA
                    (0b110, 0b0000000) => reg!(rd, reg!(rs1) | reg!(rs2)), // OR
                    (0b110, 0b0000001) => {
                        // REM
                        let rs1_value = reg!(rs1);
                        let rs2_value = reg!(rs2);

                        let value = if rs1_value == 1 << 31 && rs2_value == !0 {
                            0
                        } else if rs2_value == 0 {
                            rs1_value
                        } else {
                            (rs1_value as i32 % rs2_value as i32) as u32
                        };

                        reg!(rd, value);
                    }
                    (0b111, 0b0000000) => reg!(rd, reg!(rs1) & reg!(rs2)), // AND
                    (0b111, 0b0000001) => {
                        // REMU
                        let rs1_value = reg!(rs1);
                        let rs2_value = reg!(rs2);

                        reg!(
                            rd,
                            if rs2_value == 0 {
                                rs1_value
                            } else {
                                rs1_value % rs2_value
                            }
                        );
                    }
                    _ => unimplemented!(),
                }
            }
            0b0101111 => {
                // AMO系命令
                // hartは１つの想定なのでaq, rlは無視する。
                let address = reg!(rs1);

                if address % 4 != 0 {
                    // アライメントされていない場合
                    return Err(Trap::StoreOrAMOAddressMisaligned);
                }

                let upper_funct7 = self.inst >> 27;

                match (funct3, upper_funct7) {
                    (0b010, 0b00010) => {
                        let value = u32::from_le_bytes(self.read_memory(address)?);

                        reg!(rd, value);
                        self.reserved_address = Some(address);
                    } // LR.W
                    (0b010, 0b00011) => {
                        // SC.W
                        if let Some(reserved_address) = self.reserved_address
                            && reserved_address == address
                        {
                            self.write_memory(address, &reg!(rs2).to_le_bytes())?;
                            reg!(rd, 0);
                        } else {
                            reg!(rd, 1);
                        }

                        self.reserved_address = None;
                    }
                    _ => {
                        let original = u32::from_le_bytes(self.read_memory(address)?);

                        let value = match (funct3, upper_funct7) {
                            (0b010, 0b00000) => original.wrapping_add(reg!(rs2)), // AMOADD.W
                            (0b010, 0b00001) => reg!(rs2),                        // AMOSWAP.W
                            (0b010, 0b00100) => original ^ reg!(rs2),             // AMOOXOR.W
                            (0b010, 0b01000) => original | reg!(rs2),             // AMOOOR.W
                            (0b010, 0b01100) => original & reg!(rs2),             // AMOAND.W
                            (0b010, 0b10000) => (original as i32).min(reg!(rs2) as i32) as u32, // AMOMIN.W
                            (0b010, 0b10100) => (original as i32).max(reg!(rs2) as i32) as u32, // AMOMAX.W
                            (0b010, 0b11000) => original.min(reg!(rs2)), // AMOMINU.W
                            (0b010, 0b11100) => original.max(reg!(rs2)), // AMOMAXU.W
                            _ => unimplemented!(),
                        };

                        reg!(rd, original);
                        self.write_memory(address, &value.to_le_bytes())?;
                    }
                }
            }
            0b0110111 => reg!(rd, self.inst & 0xfffff000), // LUIll
            0b1100011 => {
                let imm = ((self.inst >> 19) & 0x1000)
                    | ((self.inst << 4) & 0x800)
                    | ((self.inst >> 20) & 0x7e0)
                    | ((self.inst >> 7) & 0x1e);
                let imm = (((imm << 19) as i32) >> 19) as u32;
                let flag = match funct3 {
                    0b000 => reg!(rs1) == reg!(rs2),               // BEQ
                    0b001 => reg!(rs1) != reg!(rs2),               // BNE
                    0b100 => reg!(rs2) as i32 > reg!(rs1) as i32,  //BLT
                    0b101 => reg!(rs1) as i32 >= reg!(rs2) as i32, // BGE
                    0b110 => reg!(rs2) > reg!(rs1),                // BLTU
                    0b111 => reg!(rs1) >= reg!(rs2),               // BGEU
                    _ => unimplemented!(),
                };

                if flag {
                    self.pc = self.pc.wrapping_add(imm);
                    is_jump = true;
                }
            }
            0b1100111 => {
                //JALR
                if funct3 != 0 {
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
                //[todo] valueについてはもっと綺麗に描けるかも。
                //リファクタ時にはアクセスの順序に注意する。
                let csr = self.inst >> 20;

                match funct3 {
                    0b001 => {
                        // CSRRW
                        let value = if rd != 0 { self.read_csr(csr)? } else { 0 };

                        self.write_csr(csr, reg!(rs1))?;

                        reg!(rd, value);
                    }
                    0b010 => {
                        // CSRRS
                        let value = self.read_csr(csr)?;
                        let rs1_value = reg!(rs1);

                        if rs1_value != 0 {
                            self.write_csr(csr, value | rs1_value)?;
                        }

                        reg!(rd, value);
                    }
                    0b011 => {
                        // CSRRC
                        let value = self.read_csr(csr)?;
                        let rs1_value = reg!(rs1);

                        if rs1_value != 0 {
                            self.write_csr(csr, value & !rs1_value)?;
                        }

                        reg!(rd, value);
                    }
                    0b101 => {
                        // CSRRWI
                        let value = if rd != 0 { self.read_csr(csr)? } else { 0 };
                        let imm = rs1;

                        self.write_csr(csr, imm)?;

                        reg!(rd, value);
                    }
                    0b110 => {
                        // CSRRSI
                        let value = self.read_csr(csr)?;
                        let imm = rs1;

                        if imm != 0 {
                            self.write_csr(csr, value | imm)?;
                        }

                        reg!(rd, value);
                    }
                    0b111 => {
                        // CSRRCI
                        let value = self.read_csr(csr)?;
                        let imm = rs1;

                        if imm != 0 {
                            self.write_csr(csr, value & !imm)?;
                        }

                        reg!(rd, value);
                    }
                    _ => match self.inst {
                        0x00000073 => {
                            // ECALL
                            match self.prv {
                                Priv::Machine => return Err(Trap::EnvCallFromMachine),
                                Priv::Supervisor => return Err(Trap::EnvCallFromSupervisor),
                                Priv::User => return Err(Trap::EnvCallFromUser),
                            }
                        }
                        0x10500073 => {
                            // WFI
                            if self.csr.is_enabled_mstatus_tw() || self.csr.is_enabled_mstatus_tvm()
                            {
                                illegal!()
                            } else {
                                loop {
                                    if self.csr.is_interrupt_active() {
                                        break;
                                    }
                                }
                            }
                        }
                        0x12000073 => {
                            // SFENCE.VMA
                            if !self.csr.is_paging_enabled() {
                                panic!(
                                    "[ERROR]: SFENCE.VMA is not supported when satp is not Bare mode."
                                );
                            }

                            illegal!();
                        }
                        0x10200073 => {
                            // SRET
                            if self.prv == Priv::User || self.csr.is_enabled_mstatus_tsr() {
                                illegal!()
                            }

                            let spp = self.csr.handle_sret()?;

                            self.change_priv(spp.into());
                            self.pc = self.csr.sepc;
                            is_jump = true;
                        }
                        0x30200073 => {
                            // MRET

                            if self.prv != Priv::Machine {
                                illegal!();
                            }

                            let mpp = self.csr.handle_mret()?;

                            self.change_priv(mpp.into());
                            self.pc = self.csr.mepc;
                            is_jump = true;
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
            Err(Trap::InstructionAddressMisaligned)
        }
    }

    // [todo]: handle_{exception,intrrupt}をまとめてhandle_trapにする。
    #[inline]
    pub fn handle_trap(&mut self, e: Trap) {
        println!("[EXCEPTION]: {:?}", e);

        match e {
            Trap::UnimplementedCSR | Trap::UnimplementedInstruction => {
                println!("{:?}", e);
                panic!("{}", self);
            }
            _ => {
                // mepcはIALIGN==32のみサポートの場合は[0..1]は０
                //[todo] MMU実装時に仮想アドレスを表すものに変更する。
                let (next_pc, next_prv) = self.csr.handle_trap(self.prv, e, self.pc, self.inst);

                self.pc = next_pc;
                self.change_priv(next_prv);
            }
        }
    }

    // 割り込みが起こっているか確認する関数
    // 起こっている場合は割り込みに対応するExceptionを返す。
    #[inline]
    pub fn check_intrrupt_active(&mut self) -> Option<Trap> {
        self.csr.resolve_pending(self.prv)
    }

    #[inline]
    fn change_priv(&mut self, prv: Priv) {
        println!("[Change Priv]: from {:?} to {:?}", self.prv, prv);
        self.prv = prv;
    }

    pub fn load_flat_program(&mut self, code: &[u8]) {
        if code.len() > MEMORY_SIZE {
            panic!("[Error]: the program is too big.");
        }

        self.init();
        self.memory.array.copy_from_slice(code);
    }

    // [todo] lazy_load_flat_program
    // [todo] load_elf_program
}
