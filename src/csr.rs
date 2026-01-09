use crate::{Exception, Priv, Result, illegal};

// デバッグ用マクロ
macro_rules! unimplemented {
    () => {
        return Err(Exception::UnimplementedCSR)
    };
}

// Machine
const MHARTID: u32 = 0xf14;

const MSTATUS: u32 = 0x300;
const MISA: u32 = 0x301;
const MIE: u32 = 0x304;
const MTVEC: u32 = 0x305;
const MCOUNTEREN: u32 = 0x306;
const MSCRATCH: u32 = 0x340;
const MEPC: u32 = 0x341;
const MCAUSE: u32 = 0x342;
const MTVAL: u32 = 0x343;
const MIP: u32 = 0x344;

const SATP: u32 = 0x180;

const MISA_MXL_SUPPORTED: u32 = 0x1 << 30; // 32bit
const MISA_A: u32 = 1 << ('A' as u32 - 'A' as u32);
const MISA_I: u32 = 1 << ('I' as u32 - 'A' as u32);
const MISA_M: u32 = 1 << ('M' as u32 - 'A' as u32);

const MISA_U: u32 = 1 << ('U' as u32 - 'A' as u32);
const MISA_S: u32 = 1 << ('S' as u32 - 'A' as u32);

const MISA_SUPPORTED_VALUE: u32 = MISA_MXL_SUPPORTED | MISA_A | MISA_I | MISA_M | MISA_U | MISA_S;

const MIE_SSIP: u32 = 0x2;
const MIE_MSIP: u32 = 0x8;
const MIE_STIP: u32 = 0x20;
const MIE_MTIP: u32 = 0x80;
const MIE_SEIP: u32 = 0x200;
const MIE_MEIP: u32 = 0x800;

const MIE_SUPPORTED: u32 = MIE_SSIP | MIE_MSIP | MIE_STIP | MIE_MTIP | MIE_SEIP | MIE_MEIP;

const MSTATUS_SIE_POS: u32 = 1;
const MSTATUS_MIE_POS: u32 = 3;
const MSTATUS_SPIE_POS: u32 = 5;
const MSTATUS_MPIE_POS: u32 = 7;
const MSTATUS_SPP_POS: u32 = 8;
const MSTATUS_MPP_POS: u32 = 11;

const MSTATUS_SIE: u32 = 1 << MSTATUS_SIE_POS;
const MSTATUS_MIE: u32 = 1 << MSTATUS_MIE_POS;
const MSTATUS_SPIE: u32 = 1 << MSTATUS_SPIE_POS;
const MSTATUS_MPIE: u32 = 1 << MSTATUS_MPIE_POS;
const MSTATUS_SPP: u32 = 1 << MSTATUS_SPP_POS;
const MSTATUS_MPP: u32 = 0x3 << MSTATUS_MPP_POS;
const MSTATUS_MPRV: u32 = 1 << 17; //[todo] implement Memory Privilege
const MSTATUS_SUM: u32 = 1 << 18; //[todo] implement when supervisor mode implemented
const MSTATUS_MXR: u32 = 1 << 19; //[todo] implement when virtual address implemented
const MSTATUS_TVM: u32 = 1 << 20; //[todo] implement when supervisor mode implemented
const MSTATUS_TW: u32 = 1 << 21; //[todo] implement when wfi instruction implemented
const MSTATUS_TSR: u32 = 1 << 22; //[todo] implement when sret instruction implemented

const MSTATUS_SUPPORTED: u32 =
    MSTATUS_SIE | MSTATUS_MIE | MSTATUS_SPIE | MSTATUS_MPIE | MSTATUS_SPP | MSTATUS_MPP;

const COUNTEREN_CY: u32 = 1;
const COUNTEREN_TM: u32 = 1 << 1;

const MCOUNTEREN_SUPPORTED: u32 = COUNTEREN_CY | COUNTEREN_TM;

// Supervisor
const SCOUNTEREN: u32 = 0x106;

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

    pub mcounteren: u32,
    pub mcycle: u32,

    pub satp: u32,
    pub scounteren: u32,

    pub timerl: u32,
    pub timerh: u32,
    pub timermatchl: u32,
    pub timermatchh: u32,
}

impl Csr {
    #[inline]
    pub fn read(&self, csr: u32, prv: Priv) -> Result<u32> {
        self.check_csr_access(csr, prv, false)?;

        match csr {
            MHARTID => Ok(0),
            MISA => Ok(MISA_SUPPORTED_VALUE),

            MSTATUS => Ok(self.mstatus),
            MCAUSE => Ok(self.mcause),
            MTVEC => Ok(self.mtvec),
            MIE => Ok(self.mie),
            MEPC => Ok(self.mepc),
            MSCRATCH => Ok(self.mscratch),
            MCOUNTEREN => Ok(self.mcounteren),

            SATP => Ok(self.satp),
            SCOUNTEREN => Ok(self.scounteren),

            CYCLE => {
                self.chceck_cycle_access(prv)?;

                Ok(self.mcycle)
            }

            0x3b0 | 0x302 | 0x7a5 | 0x744 => illegal!(), // 未実装CSR
            _ => unimplemented!(),
        }
    }

    #[inline]
    pub fn write(&mut self, csr: u32, value: u32, prv: Priv) -> Result<()> {
        self.check_csr_access(csr, prv, true)?;

        match csr {
            MSTATUS => {
                // 今の所はSWからサポートされている値を変更されても副作用はなさそう。
                if value & !MSTATUS_SUPPORTED != 0 {
                    unimplemented!();
                }

                self.mstatus = value & MSTATUS_SUPPORTED;
            }
            MTVEC => {
                self.mtvec = 0xfffffffd & value;
            }
            MIE => {
                self.mie = value & MIE_SUPPORTED;
            }
            MEPC => {
                self.mepc = value & !0x3;
            }
            MSCRATCH => {
                self.mscratch = value;
            }
            MCOUNTEREN => {
                // 今のところはCYとTMのみサポートしているが必要である場合は追加する。
                if value & !MCOUNTEREN_SUPPORTED != 0 {
                    unimplemented!();
                }

                self.mcounteren = value & MCOUNTEREN_SUPPORTED;
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

            0x3b0 | 0x302 | 0x7a5 | 0x744 => illegal!(), // 未実装CSR
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

    // mrmetのCSRでの処理を行う関数
    // 返り値はmpp
    #[inline]
    pub fn handle_mret(&mut self) -> Result<u32> {
        if self.mstatus & MSTATUS_TSR != 0 {
            illegal!();
        }

        let mpp = (self.mstatus & MSTATUS_MPP) >> MSTATUS_MPP_POS;
        let mpie = (self.mstatus & MSTATUS_MPIE) >> MSTATUS_MPIE_POS;

        self.mstatus = (self.mstatus & !(MSTATUS_MIE | MSTATUS_MPIE | MSTATUS_MPP))
            | (mpie << MSTATUS_MIE_POS)
            | (1 << MSTATUS_MPIE_POS)
            | ((Priv::User as u32) << MSTATUS_MPP_POS);

        Ok(mpp)
    }
}
