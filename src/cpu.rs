use std::fmt::Display;

use crate::{
    AccessType, Priv, Result, Trap,
    bus::{Bus, CpuContext},
    csr::Csr,
    illegal,
    tlb::{Tlb, TlbEntry},
};

const PTE_V: u32 = 1;
const PTE_R: u32 = 1 << 1;
const PTE_W: u32 = 1 << 2;
const PTE_X: u32 = 1 << 3;
const PTE_U: u32 = 1 << 4;
const PTE_A: u32 = 1 << 6;
const PTE_D: u32 = 1 << 7;
const PTESIZE: u32 = 4;

const PAGESIZE: u32 = 4096;

const DTB_ADDR: u32 = 0x80100000;

// デバッグ用マクロ
macro_rules! unimplemented {
    () => {
        return Err(Trap::UnimplementedInstruction)
    };
}

// read/write関数以外では操作してはいけない。
#[derive(Debug)]
pub struct Registers {
    regs: [u32; 32],
}

impl Default for Registers {
    fn default() -> Self {
        let mut regs = [0; 32];

        regs[11] = DTB_ADDR; // dtbはここに置いておく

        Self { regs }
    }
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
        *self = Self::default();
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

    csr: Csr,
    tlb: Tlb,

    reserved_addr: Option<u32>, // For LR.W or SC.W
    fault_addr: Option<u32>,
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

        f.write_str("---------- DUMP END ----------\n")
    }
}

impl Default for Cpu {
    fn default() -> Self {
        let prv = Priv::Machine;
        let regs = Registers::default();
        let csr = Csr::default();
        let tlb = Tlb::default();

        Self {
            prv,
            regs,
            pc: 0,
            inst: 0,
            csr,
            tlb,
            reserved_addr: None,
            fault_addr: None,
        }
    }
}

impl Cpu {
    pub fn init(&mut self) {
        *self = Self::default();
    }

    fn read_reg(&self, reg: u32) -> u32 {
        self.regs.read(reg)
    }

    fn write_reg(&mut self, reg: u32, value: u32) {
        self.regs.write(reg, value)
    }

    fn translate_va(&mut self, va: u32, access_type: AccessType, bus: &mut Bus) -> Result<u32> {
        if !self.csr.is_paging_enabled() {
            return Ok(va);
        }

        //[todo] 権限元を使用し、チェック

        let mut local_prv = self.prv;

        if self.csr.is_enabled_mstatus_mprv() && (access_type.is_read() || access_type.is_write()) {
            let mpp_prv = self.csr.get_mstatus_mpp().into();
            local_prv = mpp_prv;
        }

        if local_prv == Priv::Machine {
            return Ok(va);
        }

        let vpn = va >> 12;

        if let Some(entry) = self.tlb.lookup_ppn(va, local_prv) {
            return Ok(entry.ppn() | (va & 0xfff));
        }

        macro_rules! fault {
            ($e:expr) => {{
                self.fault_addr = Some(va);

                return Err($e);
            }};
            () => {
                fault!(access_type.into_trap(true));
            };
        }

        let mut addr = self.csr.get_satp_ppn() * PAGESIZE;
        let mut pte = 0;

        let mut last = None;

        for i in (0..2).rev() {
            let vpn = (vpn >> (10 * i)) & 0x3ff;
            let pte_addr = addr + vpn * PTESIZE;

            pte = bus.read(
                pte_addr,
                4,
                crate::bus::CpuContext {
                    csr: &mut self.csr,
                    is_walk: true,
                    access_type: AccessType::Read,
                },
            )?;

            let v = pte & PTE_V;
            let r = pte & PTE_R;
            let w = pte & PTE_W;
            let x = pte & PTE_X;

            if v == 0 || (r == 0 && w != 0) {
                fault!();
            }

            if r != 0 || x != 0 {
                // PTEを発見

                if (r == access_type as u32)
                    || (w == access_type as u32)
                    || (x == access_type as u32)
                {
                    // 権限の確認

                    if i > 0 && pte & (0x3ff << 10) != 0 {
                        // superpageのエラー
                        fault!();
                    }

                    let u = pte & PTE_U;

                    if (u == 0 && local_prv == Priv::User)
                        || (u != 0
                            && local_prv == Priv::Supervisor
                            && (r != 0 || w != 0)
                            && !self.csr.is_enabled_mstatus_sum())
                    {
                        // U=0かつ権限がUモードの場合と
                        // U=1かつ権限がSモードかつmstatus.SUM=1の場合は例外を出す
                        fault!();
                    }

                    let a = pte & PTE_A;
                    let d = pte & PTE_D;

                    let is_write = access_type.is_write();

                    if a == 0 || (is_write && d == 0) {
                        //自動更新の方ではテストが通らなさそう。

                        if self.csr.is_svadu_enabled() {
                            fault!();
                        } else {
                            todo!();
                        }
                    }

                    last = Some(i);
                    break;
                }
            }

            addr = (pte >> 10) * 4096;
        }

        if let Some(i) = last {
            // 34bitのはずだけど2bitは無視できるっぽい
            let ppn = if i == 1 {
                ((pte << 2) & 0xffc00000) | (va & 0x3ff000)
            } else {
                (pte << 2) & 0xfffff000
            };

            let pa = ppn | (va & 0xfff);

            let entry = TlbEntry::new(va, ppn, local_prv);

            self.tlb.register_entry(entry);

            Ok(pa)
        } else {
            fault!();
        }
    }

    #[inline]
    pub fn read_memory(&mut self, addr: u32, size: u32, bus: &mut Bus) -> Result<u32> {
        let access_type = AccessType::Read;
        let pa = self.translate_va(addr, access_type, bus)?;
        let ctx = CpuContext {
            csr: &mut self.csr,
            is_walk: false,
            access_type,
        };

        bus.read(pa, size, ctx)
    }

    #[inline]
    pub fn read_memory_u8(&mut self, addr: u32, bus: &mut Bus) -> Result<u32> {
        self.read_memory(addr, 1, bus)
    }

    #[inline]
    pub fn read_memory_u16(&mut self, addr: u32, bus: &mut Bus) -> Result<u32> {
        self.read_memory(addr, 2, bus)
    }

    #[inline]
    pub fn read_memory_u32(&mut self, addr: u32, bus: &mut Bus) -> Result<u32> {
        self.read_memory(addr, 4, bus)
    }

    #[inline]
    pub fn write_memory(&mut self, addr: u32, size: u32, value: u32, bus: &mut Bus) -> Result<()> {
        let access_type = AccessType::Write;

        let pa = self.translate_va(addr, access_type, bus)?;
        let ctx = CpuContext {
            csr: &mut self.csr,
            is_walk: false,
            access_type,
        };

        bus.write(pa, size, value, ctx)
    }

    #[inline]
    pub fn write_memory_u8(&mut self, addr: u32, value: u32, bus: &mut Bus) -> Result<()> {
        self.write_memory(addr, 1, value, bus)
    }

    #[inline]
    pub fn write_memory_u16(&mut self, addr: u32, value: u32, bus: &mut Bus) -> Result<()> {
        self.write_memory(addr, 2, value, bus)
    }

    #[inline]
    pub fn write_memory_u32(&mut self, addr: u32, value: u32, bus: &mut Bus) -> Result<()> {
        self.write_memory(addr, 4, value, bus)
    }

    #[inline]
    pub fn read_csr(&self, csr: u32) -> Result<u32> {
        self.csr.read(csr, self.prv)
    }

    #[inline]
    pub fn write_csr(&mut self, csr: u32, value: u32) -> Result<()> {
        self.csr.write(csr, value, self.prv)
    }

    // jump命令: Ok(true) 他の命令: Ok(false)
    // [todo] テストを通すためにテストで明示的に指定されるillegalな命令でillegal!を呼ぶが
    // テストが全て終わり、rv32imaの命令がすべて実装し終わったらunimplemented!をillegal!
    // に変更する。
    #[inline]
    pub fn step(&mut self, bus: &mut Bus) -> Result<bool> {
        macro_rules! reg {
            ($reg:expr) => {
                self.read_reg($reg)
            };
            ($reg:expr, $value:expr) => {
                self.write_reg($reg, $value)
            };
        }

        self.inst = self.fetch(bus)?;

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
                let addr = reg!(rs1).wrapping_add(imm);

                let value = match funct3 {
                    //[todo] refactor
                    0b000 => {
                        // LB
                        let value = self.read_memory_u8(addr, bus)?;
                        (((value << 24) as i32) >> 24) as u32
                    }
                    0b001 => {
                        // LH
                        let value = self.read_memory_u16(addr, bus)?;
                        (((value << 16) as i32) >> 16) as u32
                    }
                    0b010 => {
                        // LW
                        self.read_memory_u32(addr, bus)?
                    }
                    0b100 => {
                        // LBU
                        self.read_memory_u8(addr, bus)?
                    }
                    0b101 => {
                        // LHU
                        self.read_memory_u16(addr, bus)?
                    }
                    _ => unimplemented!(),
                };

                reg!(rd, value);
            }
            0b0001111 => {
                match funct3 {
                    0 => match self.inst {
                        0x8330000f => illegal!(), // FENCE.TSO
                        0x0100000f => {}          // PAUSE Zinhintpause拡張
                        _ => {}                   // FENCE
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
                let addr = reg!(rs1).wrapping_add(imm);

                match funct3 {
                    //[todo] refactor
                    0b000 => {
                        //SB
                        let value = reg!(rs2) as u8;

                        self.write_memory_u8(addr, value as u32, bus)?;
                    }
                    0b001 => {
                        // SH
                        let value = reg!(rs2) as u16;

                        self.write_memory_u16(addr, value as u32, bus)?;
                    }
                    0b010 => {
                        // SW
                        let value = reg!(rs2);

                        self.write_memory_u32(addr, value, bus)?;
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
                let addr = reg!(rs1);

                if addr % 4 != 0 {
                    // アライメントされていない場合
                    return Err(Trap::StoreOrAMOAddressMisaligned);
                }

                let upper_funct7 = self.inst >> 27;

                match (funct3, upper_funct7) {
                    (0b010, 0b00010) => {
                        let value = self.read_memory_u32(addr, bus)?;

                        reg!(rd, value);
                        self.reserved_addr = Some(addr);
                    } // LR.W
                    (0b010, 0b00011) => {
                        // SC.W
                        if let Some(reserved_addr) = self.reserved_addr
                            && reserved_addr == addr
                        {
                            self.write_memory_u32(addr, reg!(rs2), bus)?;
                            reg!(rd, 0);
                        } else {
                            reg!(rd, 1);
                        }

                        self.reserved_addr = None;
                    }
                    _ => {
                        let original = self.read_memory_u32(addr, bus)?;

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
                        self.write_memory_u32(addr, value, bus)?;
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
                    let next_pc = self.pc.wrapping_add(imm);

                    self.check_misaligned_addr(next_pc)?;

                    self.pc = next_pc;
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

                let imm = (self.inst as i32) >> 20;
                let pc = self.pc;
                let next_pc = (imm as u32).wrapping_add(reg!(rs1)) & !1;

                self.check_misaligned_addr(next_pc)?;

                self.pc = next_pc;

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
                let next_pc = pc.wrapping_add(imm as u32);

                self.check_misaligned_addr(next_pc)?;

                self.pc = next_pc;
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
                    _ => {
                        let funct7 = self.inst >> 25;

                        // [todo] リファクタリングをいつかする。
                        match funct7 {
                            0b0001001 => {
                                // SFENCE.VMA
                                if self.csr.is_enabled_mstatus_tvm() && self.prv == Priv::Supervisor
                                {
                                    illegal!()
                                }

                                self.tlb.clear();
                            }
                            _ => match self.inst {
                                0x00000073 => {
                                    // ECALL
                                    match self.prv {
                                        Priv::Supervisor => {
                                            return Err(Trap::EnvCallFromSupervisor);
                                        }
                                        Priv::User => {
                                            return Err(Trap::EnvCallFromUser);
                                        }
                                        Priv::Machine => return Err(Trap::EnvCallFromMachine),
                                    }
                                }
                                0x00100073 => {
                                    //EBREAK
                                    return Err(Trap::BreakPoint);
                                }
                                0x10500073 => {
                                    // WFI
                                    if self.csr.is_enabled_mstatus_tw()
                                        || self.csr.is_enabled_mstatus_tvm()
                                    {
                                        illegal!()
                                    } else {
                                        //loop {
                                        //    if self.csr.is_interrupt_active() {
                                        //        break;
                                        //    }

                                        //    panic!("{}", self);
                                        //}
                                    }
                                }
                                0x10200073 => {
                                    // SRET
                                    if self.prv == Priv::User || self.csr.is_enabled_mstatus_tsr() {
                                        illegal!()
                                    }

                                    let spp = self.csr.handle_sret()?;

                                    self.change_prv(spp.into());
                                    self.pc = self.csr.sepc;
                                    is_jump = true;
                                }
                                0x30200073 => {
                                    // MRET

                                    if self.prv != Priv::Machine {
                                        illegal!();
                                    }

                                    let mpp = self.csr.handle_mret()?;

                                    self.change_prv(mpp.into());
                                    self.pc = self.csr.mepc;
                                    is_jump = true;
                                }
                                _ => unimplemented!(),
                            },
                        }
                    }
                }
            }
            _ => unimplemented!(),
        }

        Ok(is_jump)
    }

    #[inline]
    fn fetch(&mut self, bus: &mut Bus) -> Result<u32> {
        let next_pc = self.translate_va(self.pc, AccessType::Fetch, bus)?;

        if next_pc % 4 == 0 {
            let inst = bus.read(
                next_pc,
                4,
                crate::bus::CpuContext {
                    csr: &mut self.csr,
                    is_walk: false,
                    access_type: AccessType::Fetch,
                },
            )?;

            Ok(inst)
        } else {
            Err(Trap::InstructionAddressMisaligned)
        }
    }

    // [todo]: handle_{exception,intrrupt}をまとめてhandle_trapにする。
    //[todo] MMU実装時にself.csr.handle_trapに渡すvaを仮想アドレスを表すものに変更する。
    #[inline]
    pub fn handle_trap(&mut self, e: Trap, bus: &mut Bus) {
        let (next_pc, next_prv) = match e {
            Trap::UnimplementedCSR | Trap::UnimplementedInstruction => {
                eprintln!("{:?}", e);
                panic!("{}", self);
            }
            Trap::InstructionAddressMisaligned
            | Trap::LoadPageFault
            | Trap::StoreOrAMOPageFault
            | Trap::InstructionPageFault => {
                let fault_addr = self.fault_addr.unwrap();
                self.fault_addr = None;
                self.csr.handle_trap(self.prv, e, self.pc, fault_addr)
            }
            Trap::IlligalInstruction => self.csr.handle_trap(self.prv, e, self.pc, self.inst),
            Trap::SupervisorExternalInterrupt => {
                self.prepare_external_interrupt(bus);
                self.csr.handle_trap(self.prv, e, self.pc, 0)
            }
            _ => self.csr.handle_trap(self.prv, e, self.pc, 0),
        };

        self.pc = next_pc;
        self.change_prv(next_prv);

        //    if self.csr.time == 0x0d5e517f {
        //        panic!("break");
        //    }
        //}
    }

    #[inline]
    pub fn prepare_external_interrupt(&mut self, bus: &mut Bus) {
        bus.prepare_interrupt();
    }

    // 割り込みが起こっているか確認する関数
    // 起こっている場合は割り込みに対応するExceptionを返す。
    #[inline]
    pub fn check_local_intrrupt_active(&mut self) -> Option<Trap> {
        self.csr.resolve_pending(self.prv)
    }

    #[inline]
    pub fn check_misaligned_addr(&mut self, addr: u32) -> Result<()> {
        if addr % 4 != 0 {
            self.fault_addr = Some(addr);
            Err(Trap::InstructionAddressMisaligned)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn prv(&self) -> Priv {
        self.prv
    }

    #[inline]
    pub fn mut_csr(&mut self) -> &mut Csr {
        &mut self.csr
    }

    #[inline]
    fn change_prv(&mut self, prv: Priv) {
        self.prv = prv;
    }

    #[inline]
    pub fn progress_pc(&mut self) {
        self.pc += 4;
    }

    pub fn set_pc(&mut self, pc: u32) {
        self.pc = pc;
    }
}
