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
const MSTATUSH: u32 = 0x310;
const MVENDORID: u32 = 0xf11;
const MARCHID: u32 = 0xf12;
const MIMPID: u32 = 0xf13;
const MEDELEG: u32 = 0x302;
const MIDELEG: u32 = 0x303;
const MIE: u32 = 0x304;
const MTVEC: u32 = 0x305;

const MEDELEG_SUPPORTED: u32 = 0xcbbff;
const MIDELEG_SUPPORTED: u32 = IP_SSIP | IP_MSIP | IP_STIP | IP_MTIP | IP_MEIP | IP_SEIP;

const TVEC_MODE: u32 = 0x3;

const MCOUNTEREN: u32 = 0x306;
const MSCRATCH: u32 = 0x340;
const MEPC: u32 = 0x341;
const MCAUSE: u32 = 0x342;
const MTVAL: u32 = 0x343;
const MIP: u32 = 0x344;
const MENVCFG: u32 = 0x30a;
const MENVCFGH: u32 = 0x31a;

const MIP_SUPPORTED: u32 = IP_SEIP | IP_SSIP;

const MISA_MXL_SUPPORTED: u32 = 0x1 << 30; // 32bit
const MISA_A: u32 = 1 << ('A' as u32 - 'A' as u32);
const MISA_I: u32 = 1 << ('I' as u32 - 'A' as u32);
const MISA_M: u32 = 1 << ('M' as u32 - 'A' as u32);

const MISA_U: u32 = 1 << ('U' as u32 - 'A' as u32);
const MISA_S: u32 = 1 << ('S' as u32 - 'A' as u32);

const MISA_SUPPORTED_VALUE: u32 = MISA_MXL_SUPPORTED | MISA_A | MISA_I | MISA_M | MISA_U | MISA_S;

const MENVCFGH_POS: u64 = 32;
const MENVCFG_FIOM: u32 = 1;
const MENVCFG_ADUE: u32 = 1 << 29;

const MCYCLE: u32 = 0xb00;
const MINSTRET: u32 = 0xb02;
const MINSTRETH: u32 = 0xb82;
const MHPMCOUNTER3: u32 = 0xb03; // 0固定でもいいっぽい。何をカウントしてもいいっぽい。将来的には使用する可能性あり。
const MHPMCOUNTER31: u32 = 0xb1f;
const MHPMCOUNTER3H: u32 = 0xb83;
const MHPMCOUNTER31H: u32 = 0xb9f;
const MCOUNTINHIBIT: u32 = 0x320;

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

const IP_MSIP_POS: u32 = 3;
const IP_MEIP_POS: u32 = 11;
const IP_SEIP_POS: u32 = 9;

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
const STATUS_MPRV: u32 = 1 << 17;
const STATUS_SUM: u32 = 1 << 18;
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
    | STATUS_TSR
    | STATUS_MPRV
    | STATUS_SUM;

const COUNTEREN_CY: u32 = 1;
const COUNTEREN_TM: u32 = 1 << 1;
const COUNTEREN_IR: u32 = 1 << 2;

const MCOUNTEREN_SUPPORTED: u32 = COUNTEREN_CY | COUNTEREN_TM;
const MCOUNTINHIBIT_INITIAL: u32 = !0x7;
const MCOUNTINHIBIT_SUPPORTED: u32 = COUNTEREN_CY | COUNTEREN_CY;

// Supervisor
const SSTATUS: u32 = 0x100;
const SIE: u32 = 0x104;
const STVEC: u32 = 0x105;
const SCOUNTEREN: u32 = 0x106;
const SSCRATCH: u32 = 0x140;
const SEPC: u32 = 0x141;
const SCAUSE: u32 = 0x142;
const STVAL: u32 = 0x143;
const SIP: u32 = 0x144;
const SATP: u32 = 0x180;
const STIMECMP: u32 = 0x14d;
const STIMECMPH: u32 = 0x15d;

const STIMECMPH_POS: u32 = 32;

const SATP_ASID: u32 = 0x1ff << 22;
const SATP_PPN: u32 = 0x3fffff;

const SSTATUS_SUPPORTED: u32 = STATUS_SIE | STATUS_SPIE | STATUS_SPP | STATUS_MXR | STATUS_SUM;
const SIE_SUPPORTED: u32 = IE_SSIE | IE_STIE | IE_SEIE;

// Unprivileged
const CYCLE: u32 = 0xc00;
const CYCLEH: u32 = 0xc80;
const TIME: u32 = 0xc01;
const TIMEH: u32 = 0xc81;
const INSTRET: u32 = 0xc02;
const INSTRETH: u32 = 0xc82;

const CYCLEH_POS: u64 = 32;
const TIMEH_POS: u64 = 32;
const INSTRETH_POS: u64 = 32;

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
    pub menvcfg: u64,

    pub mcounteren: u32,
    pub mcountinhibit: u32,
    pub mtimecmp: u64,

    pub cycle: u64,
    pub instret: u64, // 64bitのinstret 0-31がminstretで32-63がminstreth
    pub time: u64,

    pub satp: u32,
    pub scounteren: u32,
    pub stvec: u32,
    pub scause: u32,
    pub stval: u32,
    pub sepc: u32,
    pub sscratch: u32,
    pub stimecmp: u64,

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
            MSTATUSH => Ok(0), // littleエンディアンのみ
            MENVCFG => Ok(self.menvcfg as u32),
            MENVCFGH => Ok((self.menvcfg >> MENVCFGH_POS) as u32),

            MINSTRET => Ok(self.instret as u32),
            MINSTRETH => Ok((self.instret >> MINSTRETH_POS) as u32),
            MHPMCOUNTER3..=MHPMCOUNTER31 | MHPMCOUNTER3H..=MHPMCOUNTER31H => Ok(0),
            MCOUNTINHIBIT => Ok(self.mcountinhibit | MCOUNTINHIBIT_INITIAL),

            SCOUNTEREN => Ok(self.scounteren),

            SSTATUS => Ok(self.mstatus & SSTATUS_SUPPORTED),
            SEPC => Ok(self.sepc),
            SATP => {
                // mstatus.TVM && Supervisorのときは例外を起こすべきらしいけどなぜかそれだとテストが通らなかったので仕様変更されている？
                if prv == Priv::Supervisor && self.is_enabled_mstatus_tvm() {
                    illegal!();
                }
                Ok(self.satp)
            }
            // ""
            STVEC => Ok(self.stvec),
            SSCRATCH => Ok(self.sscratch),
            SCAUSE => Ok(self.scause),
            STVAL => Ok(self.stval),
            SIE => Ok(self.mie & SIE_SUPPORTED),
            SIP => Ok(self.mip & SIE_SUPPORTED),
            STIMECMP => Ok(self.stimecmp as u32),
            STIMECMPH => Ok((self.stimecmp >> STIMECMPH_POS) as u32),

            CYCLE => {
                self.chceck_cycle_access(prv)?;

                Ok(self.cycle as u32)
            }

            CYCLEH => {
                self.chceck_cycle_access(prv)?;

                Ok((self.cycle >> CYCLEH_POS) as u32)
            }

            TIME => {
                self.chceck_time_access(prv)?;

                Ok(self.time as u32)
            }
            TIMEH => {
                self.chceck_time_access(prv)?;

                Ok((self.time >> TIMEH_POS) as u32)
            }

            INSTRET => {
                self.chceck_instret_access(prv)?;

                Ok(self.instret as u32)
            }
            INSTRETH => {
                self.chceck_instret_access(prv)?;

                Ok((self.instret >> INSTRETH_POS) as u32)
            }

            0x3b0 | 0x7a5 | 0x744 | 0x3a0 | 0xda0 | 0xfb0 | 0x30c | 0x10c | 0x321 | 0x7a0 => {
                illegal!()
            } // 未実装CSR
            _ => unimplemented!(),
        }
    }

    #[inline]
    pub fn write(&mut self, csr: u32, value: u32, prv: Priv) -> Result<()> {
        self.check_csr_access(csr, prv, true)?;

        match csr {
            MISA | MSTATUSH | MHPMCOUNTER3..=MHPMCOUNTER31 | MHPMCOUNTER3H..=MHPMCOUNTER31H => {} // 書き込みは実装しない
            MSTATUS => self.mstatus = value & MSTATUS_SUPPORTED,
            MTVEC => self.mtvec = 0xfffffffd & value,
            MIE => self.mie = value & MIE_SUPPORTED,
            MIP => self.mip = value & MIP_SUPPORTED, // MEIP, MTIPの直接書き込みは無視する。
            MEPC => self.mepc = value & !0x3,
            MSCRATCH => self.mscratch = value,
            MCOUNTEREN => {
                // 今のところはCYとTMのみサポートしているが必要である場合は追加する。
                //if value & !MCOUNTEREN_SUPPORTED != 0 {
                //    unimplemented!();
                //}

                self.mcounteren = value & MCOUNTEREN_SUPPORTED;
            }
            MTVAL => self.mtval = value,
            MEDELEG => self.medeleg = value & MEDELEG_SUPPORTED,
            MIDELEG => self.mideleg = value & MIDELEG_SUPPORTED,
            MENVCFG => self.menvcfg = (self.menvcfg & 0xffff0000) | (value & MENVCFG_FIOM) as u64,
            MENVCFGH => self.menvcfg = self.menvcfg | ((value & MENVCFG_ADUE) << 31) as u64,
            MINSTRET => {
                self.instret = (self.instret & !MINSTRET_MASK) | (value as u64);

                self.suppress_minsret = true;
            }

            MINSTRETH => {
                self.instret = (self.instret & MINSTRET_MASK) | ((value as u64) << MINSTRETH_POS);

                self.suppress_minsret = true;
            }
            MCOUNTINHIBIT => {
                // mphmcounterNをまともに実装していない場合について記述がなかったのでとりあえずこのようにする。
                //if value & MCOUNTINHIBIT_INITIAL != 0 {
                //    unimplemented!();
                //}
                self.mcountinhibit = (self.mcountinhibit | MCOUNTINHIBIT_INITIAL)
                    | (value & MCOUNTINHIBIT_SUPPORTED);
            }

            SATP => {
                if prv == Priv::Supervisor && self.is_enabled_mstatus_tvm() {
                    illegal!();
                }

                // ASID[8:7]=3 && BAREの場合はカスタムユースらしいが無視する。
                self.satp = value & !SATP_ASID
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
                    panic!("[WARNING]: sstatus.MXR is not supported.");
                }

                self.mstatus = (self.mstatus & !SSTATUS_SUPPORTED) | (value & SSTATUS_SUPPORTED);
            }

            SEPC => self.sepc = value & !0x3,
            STVEC => self.stvec = 0xfffffffd & value,
            SSCRATCH => self.sscratch = value,
            SIE => self.mie = (self.mie & !SIE_SUPPORTED) | (value & SIE_SUPPORTED),
            SIP => self.mip = (self.mip & !IE_SSIE) | (value & IE_SSIE),
            STVAL => self.stval = value,
            SCAUSE => {
                let is_interrupt = value >> 31 == 1;
                let cause = (value << 1) >> 1;

                if is_interrupt {
                    // 割り込み
                    match cause {
                        1 | 5 | 9 | 13 => self.scause = value,
                        _ => {}
                    }
                } else {
                    // 例外

                    match cause {
                        0..=9 | 12 | 13 | 15 | 18 | 19 => self.scause = value,
                        _ => {}
                    }
                }
            }
            STIMECMP => {
                self.chceck_time_access(prv)?;

                let stimecmp = (self.stimecmp & (0xffffffff << 32)) | (value as u64);

                if stimecmp > self.time {
                    self.mip = self.mip & !IP_STIP;
                } else {
                    self.mip = self.mip | IP_STIP;
                }

                self.stimecmp = stimecmp;
            }

            STIMECMPH => {
                self.chceck_time_access(prv)?;

                let stimecmp = (self.stimecmp & 0xffffffff) | ((value as u64) << STIMECMPH_POS);

                if stimecmp > self.time {
                    self.mip = self.mip & !IP_STIP;
                } else {
                    self.mip = self.mip | IP_STIP;
                }

                self.stimecmp = stimecmp;
            }

            0x3b0 | 0x7a5 | 0x744 | 0x3a0 => illegal!(), // 未実装CSR
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
        if prv == Priv::Machine {
            return Ok(());
        }

        if self.mcounteren & COUNTEREN_CY != 0 {
            if prv == Priv::Supervisor || self.scounteren & COUNTEREN_CY != 0 {
                return Ok(());
            }
        }

        illegal!();
    }

    #[inline]
    fn chceck_time_access(&self, prv: Priv) -> Result<()> {
        if prv == Priv::Machine {
            return Ok(());
        }

        if self.mcounteren & COUNTEREN_TM != 0 {
            if prv == Priv::Supervisor || self.scounteren & COUNTEREN_TM != 0 {
                return Ok(());
            }
        }

        illegal!();
    }

    #[inline]
    fn chceck_instret_access(&self, prv: Priv) -> Result<()> {
        if prv == Priv::Machine {
            return Ok(());
        }

        if self.mcounteren & COUNTEREN_IR != 0 {
            if prv == Priv::Supervisor || self.scounteren & COUNTEREN_IR != 0 {
                return Ok(());
            }
        }

        illegal!();
    }

    #[inline]
    pub fn progress_cycle(&mut self) {
        if self.mcountinhibit & COUNTEREN_CY == 0 {
            self.cycle = self.cycle.wrapping_add(1);
        }
    }

    #[inline]
    pub fn progress_time(&mut self) {
        self.time = self.time.wrapping_add(1);

        if self.time >= self.mtimecmp {
            self.mip = self.mip | IP_MTIP;
        } else {
            self.mip = self.mip & !IP_MTIP;
        }

        if self.time >= self.stimecmp {
            self.mip = self.mip | IP_STIP;
        } else {
            self.mip = self.mip & !IP_STIP;
        }
    }

    #[inline]
    pub fn progress_instret(&mut self) {
        if !self.suppress_minsret {
            if self.mcountinhibit & COUNTEREN_IR == 0 {
                self.instret = self.instret.wrapping_add(1);
            }
        } else {
            self.suppress_minsret = false;
        }
    }

    #[inline]
    pub fn get_satp_ppn(&self) -> u32 {
        self.satp & SATP_PPN
    }

    #[inline]
    pub fn get_mstatus_mpp(&self) -> u32 {
        (self.mstatus & STATUS_MPP) >> STATUS_MPP_POS
    }

    #[inline]
    pub fn get_mip_msip(&self) -> u32 {
        (self.mip & IP_MSIP) >> IP_MSIP_POS
    }

    #[inline]
    pub fn set_mip_msip(&mut self, msip: u32) {
        self.mip = (self.mip & !IP_MSIP) | ((msip & 0x1) << IP_MSIP_POS);
    }

    #[inline]
    pub fn set_mip_meip(&mut self, meip: u32) {
        self.mip = (self.mip & !IP_MEIP) | ((meip & 0x1) << IP_MEIP_POS);
    }

    #[inline]
    pub fn set_mip_seip(&mut self, seip: u32) {
        self.mip = (self.mip & !IP_SEIP) | ((seip & 0x1) << IP_SEIP_POS);
    }

    #[inline]
    pub fn set_mtimecmp(&mut self, mtimecmp: u32) {
        let mtimecmp = (self.mtimecmp & (0xffffffff << 32)) | (mtimecmp as u64);

        if mtimecmp > self.time {
            self.mip = self.mip & !IP_MTIP;
        } else {
            self.mip = self.mip | IP_MTIP;
        }

        self.mtimecmp = mtimecmp;
    }

    #[inline]
    pub fn set_mtimecmph(&mut self, mtimecmph: u32) {
        let mtimecmp = (self.mtimecmp & 0xffffffff) | ((mtimecmph as u64) << 32);

        if mtimecmp > self.time {
            self.mip = self.mip & !IP_MTIP;
        } else {
            self.mip = self.mip & IP_MTIP;
        }

        self.mtimecmp = mtimecmp;
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
    pub fn is_enabled_mstatus_mprv(&self) -> bool {
        self.mstatus & STATUS_MPRV != 0
    }

    #[inline]
    pub fn is_enabled_mstatus_sum(&self) -> bool {
        self.mstatus & STATUS_SUM != 0
    }

    #[inline]
    pub fn is_enabled_mstatus_tsr(&self) -> bool {
        self.mstatus & STATUS_TSR != 0
    }

    #[inline]
    pub fn is_paging_enabled(&self) -> bool {
        self.satp >> 31 == 1
    }

    #[inline]
    pub fn is_svadu_enabled(&self) -> bool {
        (self.menvcfg >> MENVCFGH_POS) & MENVCFG_ADUE as u64 == 0
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

            self.mepc = va & !0x3;

            if is_interrupt && self.mtvec & TVEC_MODE != 0 {
                ((self.mtvec & !TVEC_MODE) + cause * 4, Priv::Machine)
            } else {
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

            self.sepc = va & !0x3;

            if is_interrupt && self.stvec & TVEC_MODE != 0 {
                ((self.stvec & !TVEC_MODE) + cause * 4, Priv::Supervisor)
            } else {
                (self.stvec & !TVEC_MODE, Priv::Supervisor)
            }
        }
    }

    #[inline]
    pub fn can_external_interrupt(&self, from_prv: Priv) -> bool {
        match from_prv {
            Priv::Machine => {
                if self.mstatus & STATUS_MIE == 0 {
                    return false;
                }
            }
            Priv::Supervisor => {
                if self.mideleg & IP_SEIP != 0 {
                    if self.mstatus & STATUS_SIE == 0 {
                        return false;
                    }
                }
            }
            Priv::User => {}
        }

        true
    }

    // [todo]: 複数割り込みの順番の実装
    #[inline]
    pub fn resolve_pending(&mut self, from_prv: Priv) -> Option<Trap> {
        let active_bit = self.mip & self.mie;

        if active_bit == 0 {
            return None;
        }

        let active_bit = {
            match from_prv {
                Priv::Machine => {
                    if self.mstatus & STATUS_MIE == 0 {
                        return None;
                    }
                }
                Priv::Supervisor => {
                    if active_bit & self.mideleg != 0 {
                        // 委譲
                        if self.mstatus & STATUS_SIE == 0 {
                            return None;
                        }
                    }
                }
                Priv::User => {}
            }
            active_bit
        };

        if active_bit == 0 {
            return None;
        }

        if active_bit & 0x200 != 0 {
            return Some(Trap::SupervisorExternalInterrupt);
        }

        if active_bit & 0x2 != 0 {
            return Some(Trap::SupervisorSoftwareInterrupt);
        }

        if active_bit & 0x20 != 0 {
            return Some(Trap::SupervisorTimerInterrupt);
        }

        panic!(
            "[ERROR]: Unknown or invalid interrupt({}) occured.",
            active_bit
        );
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

        if mpp != Priv::Machine as u32 {
            self.mstatus = self.mstatus & !STATUS_MPRV;
        }

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

        if spp != Priv::Machine as u32 {
            self.mstatus = self.mstatus & !STATUS_MPRV;
        }

        Ok(spp)
    }
}
