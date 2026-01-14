use crate::{Priv, Result, Trap, illegal};

// デバッグ用マクロ
macro_rules! unimplemented {
    () => {
        return Err(Trap::UnimplementedCSR)
    };
}

// Machine
const MHARTID: u32 = 0xf14;

const MSTATUS: u32 = 0x300;
const MISA: u32 = 0x301;
const MVENDORID: u32 = 0xf11;
const MARCHID: u32 = 0xf12;
const MIMPID: u32 = 0xf13;
const MEDELEG: u32 = 0x302;

const MEDELEG_SUPPORTED: u32 = 0xcbbff;

const MIDELEG: u32 = 0x303;
const MIE: u32 = 0x304;
const MTVEC: u32 = 0x305;

const TVEC_MODE: u32 = 0x3;

const MCOUNTEREN: u32 = 0x306;
const MSCRATCH: u32 = 0x340;
const MEPC: u32 = 0x341;
const MCAUSE: u32 = 0x342;
const MTVAL: u32 = 0x343;
const MIP: u32 = 0x344;

const MIP_SUPPORTED: u32 = IP_SEIP | IP_STIP | IP_SSIP;

const MISA_MXL_SUPPORTED: u32 = 0x1 << 30; // 32bit
const MISA_A: u32 = 1 << ('A' as u32 - 'A' as u32);
const MISA_I: u32 = 1 << ('I' as u32 - 'A' as u32);
const MISA_M: u32 = 1 << ('M' as u32 - 'A' as u32);

const MISA_U: u32 = 1 << ('U' as u32 - 'A' as u32);
const MISA_S: u32 = 1 << ('S' as u32 - 'A' as u32);

const MISA_SUPPORTED_VALUE: u32 = MISA_MXL_SUPPORTED | MISA_A | MISA_I | MISA_M | MISA_U | MISA_S;

const MCYCLE: u32 = 0xb00;
const MINSTRET: u32 = 0xb02;
const MINSTRETH: u32 = 0xB82;

const MINSTRET_MASK: u64 = 0xffffffff;
const MINSTRETH_POS: u64 = 31;

const IE_SSIE: u32 = 0x2;
const IE_MSIE: u32 = 0x8;
const IE_STIE: u32 = 0x20;
const IE_MTIE: u32 = 0x80;
const IE_SEIE: u32 = 0x200;
const IE_MEIE: u32 = 0x800;

const MIE_SUPPORTED: u32 = IE_SSIE | IE_MSIE | IE_STIE | IE_MTIE | IE_SEIE | IE_MEIE;

const IP_SSIP: u32 = 0x2;
const IP_MSIP: u32 = 0x8;
const IP_STIP: u32 = 0x20;
const IP_MTIP: u32 = 0x80;
const IP_SEIP: u32 = 0x200;
const IP_MEIP: u32 = 0x800;

const STATUS_SIE_POS: u32 = 1;
const STATUS_MIE_POS: u32 = 3;
const STATUS_SPIE_POS: u32 = 5;
const STATUS_MPIE_POS: u32 = 7;
const STATUS_SPP_POS: u32 = 8;
const STATUS_MPP_POS: u32 = 11;

const STATUS_SIE: u32 = 1 << STATUS_SIE_POS;
const STATUS_MIE: u32 = 1 << STATUS_MIE_POS;
const STATUS_SPIE: u32 = 1 << STATUS_SPIE_POS;
const STATUS_MPIE: u32 = 1 << STATUS_MPIE_POS;
const STATUS_SPP: u32 = 1 << STATUS_SPP_POS;
const STATUS_MPP: u32 = 0x3 << STATUS_MPP_POS;
const STATUS_MPRV: u32 = 1 << 17; //[todo] implement Memory Privilege
const STATUS_SUM: u32 = 1 << 18; //[todo] implement when supervisor mode implemented
const STATUS_MXR: u32 = 1 << 19; //[todo] implement when virtual address implemented
const STATUS_TVM: u32 = 1 << 20; //[todo] implement when supervisor mode implemented
const STATUS_TW: u32 = 1 << 21; //[todo] implement when wfi instruction implemented
const STATUS_TSR: u32 = 1 << 22; //[todo] implement when sret instruction implemented

const MSTATUS_SUPPORTED: u32 = STATUS_SIE
    | STATUS_MIE
    | STATUS_SPIE
    | STATUS_MPIE
    | STATUS_SPP
    | STATUS_MPP
    | STATUS_TVM
    | STATUS_TSR;

const COUNTEREN_CY: u32 = 1;
const COUNTEREN_TM: u32 = 1 << 1;

const MCOUNTEREN_SUPPORTED: u32 = COUNTEREN_CY | COUNTEREN_TM;

// Supervisor
const SATP: u32 = 0x180;
const SSTATUS: u32 = 0x100;
const SCOUNTEREN: u32 = 0x106;
const SEPC: u32 = 0x141;

const SSTATUS_SUPPORTED: u32 = STATUS_SIE | STATUS_SPIE | STATUS_SPP | STATUS_MXR | STATUS_SUM;

// Unprivileged
const CYCLE: u32 = 0xC00;
const TIME: u32 = 0xC01;

#[derive(Default, Debug)]
pub struct Csr {
    pub mstatus: u32,
    pub mscratch: u32,
    pub mtvec: u32,
    pub mie: u32,
    pub mip: u32,
    pub mepc: u32,
    pub mtval: u32,
    pub mcause: u32,
    pub medeleg: u32,
    pub mideleg: u32,

    pub mcounteren: u32,
    pub mcycle: u32,

    pub instret: u64, // 64bitのinstret 0-31がminstretで32-63がminstreth

    pub satp: u32,
    pub scounteren: u32,
    pub stvec: u32,
    pub scause: u32,
    pub stval: u32,
    pub sepc: u32,

    pub timerl: u32,
    pub timerh: u32,
    pub timermatchl: u32,
    pub timermatchh: u32,

    pub suppress_minsret: bool, // CSR命令でminsretが書き込まれた時にretireするときにincしないためのフラグ
}

impl Csr {
    #[inline]
    pub fn read(&self, csr: u32, prv: Priv) -> Result<u32> {
        self.check_csr_access(csr, prv, false)?;

        match csr {
            MHARTID => Ok(0),
            MISA => Ok(MISA_SUPPORTED_VALUE),
            MIMPID => Ok(1), //とりあえずバージョンは1
            MARCHID => Ok(1),
            MVENDORID => Ok(0),

            MSTATUS => Ok(self.mstatus),
            MCAUSE => Ok(self.mcause),
            MTVEC => Ok(self.mtvec),
            MIE => Ok(self.mie),
            MIP => Ok(self.mip),
            MEPC => Ok(self.mepc),
            MSCRATCH => Ok(self.mscratch),
            MCOUNTEREN => Ok(self.mcounteren),
            MTVAL => Ok(self.mtval),
            MEDELEG => Ok(self.medeleg),
            MIDELEG => Ok(self.mideleg),

            MINSTRET => Ok(self.instret as u32),
            MINSTRETH => Ok((self.instret >> MINSTRETH_POS) as u32),

            SCOUNTEREN => Ok(self.scounteren),

            SSTATUS => Ok(self.mstatus & SSTATUS_SUPPORTED),
            SEPC => Ok(self.sepc),
            SATP => Ok(self.satp),

            CYCLE => {
                self.chceck_cycle_access(prv)?;

                Ok(self.mcycle)
            }

            0x3b0 | 0x7a5 | 0x744 => illegal!(), // 未実装CSR
            _ => unimplemented!(),
        }
    }

    #[inline]
    pub fn write(&mut self, csr: u32, value: u32, prv: Priv) -> Result<()> {
        self.check_csr_access(csr, prv, true)?;

        println!("[CSR] write addr: {:08x} value: {:08x}", csr, value);

        match csr {
            MISA => {} // 書き込みは実装しない
            MSTATUS => {
                if value & !MSTATUS_SUPPORTED != 0 {
                    unimplemented!();
                }

                self.mstatus = value & MSTATUS_SUPPORTED;
            }
            MTVEC => self.mtvec = 0xfffffffd & value,
            MIE => self.mie = value & MIE_SUPPORTED,
            MIP => self.mip = value & MIP_SUPPORTED, // MEIP, MTIPの直接書き込みは無視する。
            MEPC => self.mepc = value & !0x3,
            MSCRATCH => self.mscratch = value,
            MCOUNTEREN => {
                // 今のところはCYとTMのみサポートしているが必要である場合は追加する。
                if value & !MCOUNTEREN_SUPPORTED != 0 {
                    unimplemented!();
                }

                self.mcounteren = value & MCOUNTEREN_SUPPORTED;
            }
            MTVAL => self.mtval = value,
            MEDELEG => self.medeleg = value & MEDELEG_SUPPORTED,
            MIDELEG => self.mideleg = value & MIP_SUPPORTED,

            MINSTRET => {
                self.instret = (self.instret & !MINSTRET_MASK) | (value as u64);

                self.suppress_minsret = true;
            }

            MINSTRETH => {
                self.instret = (self.instret & MINSTRET_MASK) | ((value as u64) << MINSTRETH_POS);

                self.suppress_minsret = true;
            }

            SATP => {
                if value != 0 {
                    panic!("[ERROR]: BARE mode of satp is supported.");
                }

                self.satp = value;
            }

            SCOUNTEREN => {
                // 今のところはCYとTMのみサポートしているが必要である場合は追加する。
                if value & !MCOUNTEREN_SUPPORTED != 0 {
                    unimplemented!();
                }

                self.scounteren = value & MCOUNTEREN_SUPPORTED;
            }

            SSTATUS => {
                if value & !SSTATUS_SUPPORTED != 0 {
                    unimplemented!();
                }

                if value & STATUS_MXR != 0 {
                    println!("[WARNING]: sstatus.MXR is not supported.");
                }

                if value & STATUS_SUM != 0 {
                    println!("[WARNING]: sstatus.SUM is not supported.");
                }

                self.mstatus = (self.mstatus & !SSTATUS_SUPPORTED)
                    | (value & (SSTATUS_SUPPORTED & !STATUS_SUM & !STATUS_MXR));
            }

            SEPC => self.sepc = value & !0x3,

            0x3b0 | 0x7a5 | 0x744 => illegal!(), // 未実装CSR
            _ => unimplemented!(),
        }

        Ok(())
    }

    // アクセスについての権限等をチェックする関数
    // マクロにすべきかもしれない
    // is_write: true->write false->read
    #[inline]
    fn check_csr_access(&self, csr: u32, prv: Priv, is_write: bool) -> Result<()> {
        let access = (csr >> 10) & 0x3;
        let req_prv = (csr >> 8) & 0x3;

        if is_write && access == 0b11 {
            illegal!();
        }

        if req_prv == 0b10 {
            illegal!();
        }

        if req_prv > prv as u32 {
            illegal!();
        }

        Ok(())
    }

    #[inline]
    fn chceck_cycle_access(&self, prv: Priv) -> Result<()> {
        match prv {
            Priv::Machine => Ok(()),
            Priv::Supervisor => {
                if self.mcounteren & COUNTEREN_CY != 0 {
                    Ok(())
                } else {
                    illegal!()
                }
            }
            Priv::User => {
                if self.mcounteren & COUNTEREN_CY != 0 && self.scounteren & COUNTEREN_CY != 0 {
                    Ok(())
                } else {
                    illegal!()
                }
            }
        }
    }

    #[inline]
    pub fn check_instruction_adress_misaligned(&mut self, addr: u32) -> Result<()> {
        if addr % 4 != 0 {
            self.mtval = addr;

            Err(Trap::InstructionAddressMisaligned)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn progress_cycle(&mut self) {
        self.mcycle = self.mcycle.wrapping_add(1);
    }

    #[inline]
    pub fn progress_instret(&mut self) {
        if !self.suppress_minsret {
            self.instret = self.instret.wrapping_add(1);
        } else {
            self.suppress_minsret = false;
        }
    }

    // mstatus.TWが有効かどうかを判定する関数
    #[inline]
    pub fn is_enabled_mstatus_tw(&self) -> bool {
        self.mstatus & STATUS_TW != 0
    }

    // mstatus.TVMが有効かどうかを判定する関数
    // satpとsfence.vma or sinval.vmaを実行するときにillegal-instructionを出す。
    #[inline]
    pub fn is_enabled_mstatus_tvm(&self) -> bool {
        self.mstatus & STATUS_TVM != 0
    }

    #[inline]
    pub fn is_enabled_mstatus_tsr(&self) -> bool {
        self.mstatus & STATUS_TSR != 0
    }

    #[inline]
    pub fn is_paging_enabled(&self) -> bool {
        self.satp >> 31 == 0
    }

    // トラップを処理する関数
    // mstatusを変更するのでmstatusの前の値を使用する場合はこの関数を呼び出す前にその処理を行う。
    // vaはtrapが起こったVirtual Addressを渡す。
    // 移動先のアドレスを返す。
    // [todo]: refactor
    #[inline]
    pub fn handle_trap(&mut self, from_prv: Priv, e: Trap, va: u32, xtval: u32) -> (u32, Priv) {
        let is_interrupt = e.is_interrupt();
        let cause = e.cause();

        if from_prv == Priv::Machine
            || (is_interrupt && ((self.mideleg >> cause) & 0x1) == 0)
            || (!is_interrupt && ((self.medeleg >> cause) & 0x1) == 0)
        {
            // 委譲しない場合

            self.mcause = e as u32;

            if e != Trap::EnvCallFromMachine
                && e != Trap::EnvCallFromSupervisor
                && e != Trap::EnvCallFromUser
            {
                self.mtval = xtval;
            }

            let mie = (self.mstatus & STATUS_MIE) >> STATUS_MIE_POS;

            self.mstatus = (self.mstatus & !(STATUS_MPIE | STATUS_MIE | STATUS_MPP))
                | (mie << STATUS_MPIE_POS)
                | ((from_prv as u32) << STATUS_MPP_POS);

            if is_interrupt && self.mtvec & TVEC_MODE != 0 {
                self.mepc = (va & !0x3) + 4;

                ((self.mtvec & !TVEC_MODE) + cause * 4, Priv::Machine)
            } else {
                self.mepc = va & !0x3;
                (self.mtvec & !TVEC_MODE, Priv::Machine)
            }
        } else {
            // 委譲する場合

            self.scause = e as u32;

            if e != Trap::EnvCallFromMachine
                && e != Trap::EnvCallFromSupervisor
                && e != Trap::EnvCallFromUser
            {
                self.stval = xtval;
            }

            let spp = if from_prv == Priv::User { 0 } else { 1 };
            let sie = (self.mstatus & STATUS_SIE) >> STATUS_SIE_POS;

            self.mstatus = (self.mstatus & !(STATUS_SPIE | STATUS_SIE | STATUS_SPP))
                | (sie << STATUS_SPIE_POS)
                | (0 << STATUS_SIE)
                | (spp << STATUS_SPP_POS);

            if is_interrupt && self.stvec & TVEC_MODE != 0 {
                self.sepc = (va & !0x3) + 4;

                ((self.stvec & !TVEC_MODE) + cause * 4, Priv::Supervisor)
            } else {
                self.sepc = va & !0x3;
                (self.stvec & !TVEC_MODE, Priv::Supervisor)
            }
        }
    }

    // 委譲や{M,S}IEをきにせず割り込みが起こっているかどうかを判定する関数
    #[inline]
    pub fn is_interrupt_active(&self) -> bool {
        self.mie & self.mip != 0
    }

    // [todo]: 複数割り込みの順番の実装
    #[inline]
    pub fn resolve_pending(&mut self, from_prv: Priv) -> Option<Trap> {
        if self.mstatus & STATUS_MIE == 0 {
            return None;
        }

        let active_bit = if from_prv == Priv::Machine {
            self.mip & self.mie
        } else {
            let active_bit = self.mip & self.mie;

            if active_bit & self.mideleg != 0 {
                // 委譲が有効の場合
                if self.mstatus & STATUS_SIE == 0 {
                    return None;
                }

                active_bit & self.mideleg
            } else {
                active_bit
            }
        };

        // [todo]: const {}が使えないっぽいのでこうなっているがいい感じにする方法があれば変更する。
        match active_bit {
            0 => None,
            0b10 => return Some(Trap::SupervisorSoftwareInterrupt),
            _ => panic!("[ERROR]: Unknown or invalid interrupt occured."),
        }
    }

    // mrmetのCSRでの処理を行う関数
    // 返り値はmpp
    #[inline]
    pub fn handle_mret(&mut self) -> Result<u32> {
        let mpp = (self.mstatus & STATUS_MPP) >> STATUS_MPP_POS;
        let mpie = (self.mstatus & STATUS_MPIE) >> STATUS_MPIE_POS;

        self.mstatus = (self.mstatus & !(STATUS_MIE | STATUS_MPIE | STATUS_MPP))
            | (mpie << STATUS_MIE_POS)
            | (1 << STATUS_MPIE_POS)
            | ((Priv::User as u32) << STATUS_MPP_POS);

        Ok(mpp)
    }

    // srmetのCSRでの処理を行う関数
    // 返り値はspp
    #[inline]
    pub fn handle_sret(&mut self) -> Result<u32> {
        let spp = (self.mstatus & STATUS_SPP) >> STATUS_SPP_POS;
        let spie = (self.mstatus & STATUS_SPIE) >> STATUS_SPIE_POS;

        self.mstatus = (self.mstatus & !(STATUS_SIE | STATUS_SPIE | STATUS_SPP | STATUS_MPRV))
            | (spie << STATUS_SIE_POS)
            | (1 << STATUS_SPIE_POS)
            | ((Priv::User as u32) << STATUS_SPP_POS);

        Ok(spp)
    }
}
