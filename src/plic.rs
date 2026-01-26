use crate::{IRQ, Priv, Result, Trap, cpu::Cpu};

const PLIC_MAX_NUM: u32 = 1024;
const PLIC_CONTEXT_MAX_NUM: u32 = 15872;

const PLIC_NUM: u32 = 32;
const PLIC_CONTEXT_NUM: u32 = 2;

const PLIC_PRIORITY_BASE: u32 = 0;
const PLIC_PRIORITY_END: u32 = PLIC_PRIORITY_BASE + PLIC_MAX_NUM * 4;

const PLIC_ENABLE_BASE: u32 = 0x2000;
const PLIC_ENABLE_UNIT: u32 = 0x80;
const PLIC_ENABLE_END: u32 = PLIC_ENABLE_BASE + PLIC_CONTEXT_MAX_NUM * PLIC_ENABLE_UNIT;

const PLIC_THREADSHOLD_BASE: u32 = 0x200000;
const PLIC_THREADSHOLD_UNIT: u32 = 0x1000;

const PLIC_CLAIM_END: u32 =
    PLIC_THREADSHOLD_BASE + PLIC_CONTEXT_MAX_NUM * PLIC_THREADSHOLD_UNIT + 4;

#[derive(Default, Debug)]
pub struct Plic {
    priories: [u32; PLIC_NUM as usize],
    pub pending: [u32; (PLIC_NUM / 32) as usize],
    enables: [[u32; (PLIC_NUM / 32) as usize]; 2],
    threasholds: [u32; PLIC_CONTEXT_NUM as usize],

    interrupting_irq: Option<IRQ>,
    interrupting_ctx: Option<usize>,
}

impl Plic {
    #[inline]
    pub fn set_pending(&mut self, irq: IRQ) {
        let irq = irq as usize;
        let idx = irq / 32;
        let bit = 1 << (irq % 32);

        self.pending[idx] |= bit;
    }

    #[inline]
    pub fn unset_pending(&mut self, irq: IRQ) {
        let irq = irq as usize;
        let idx = irq / 32;
        let bit = 1 << (irq % 32);

        self.pending[idx] &= !bit;
    }

    // 割り込みが起こっているものを調べる関数
    // 何回も呼ぶとめっちゃ重くなるので割り込みが起こっているとわかっている場面で呼ぶべき
    #[inline]
    pub fn find_interrupt_active(&self) -> (u32, IRQ, usize) {
        let mut max_priority = 0;
        let mut target_irq = IRQ::None;
        let mut target_ctx = 0;

        for irq in 0..PLIC_NUM {
            let irq = irq as usize;

            let idx = irq / 32;
            let bit = 1 << (irq % 32);

            if self.pending[idx] & bit == 0 {
                continue;
            }

            let priority = self.priories[irq];

            for ctx_idx in 0..PLIC_CONTEXT_NUM {
                let ctx_idx = ctx_idx as usize;

                if self.enables[ctx_idx][idx] & bit != 0 {
                    if priority > self.threasholds[ctx_idx] {
                        max_priority = priority;
                        target_irq = irq.into();
                        target_ctx = ctx_idx;
                    }
                }
            }
        }

        (max_priority, target_irq, target_ctx)
    }

    #[inline]
    pub fn raise_interrupt(&mut self) -> Option<Priv> {
        let (_, irq, ctx) = self.find_interrupt_active();

        // デバイス的には割り込みが起こってもいい場面で
        // 条件が揃わない場合はNoneを返し、条件が揃った瞬間に外部割り込みを起こす
        if irq == IRQ::None {
            return None;
        }

        self.interrupting_irq = Some(irq);
        self.interrupting_ctx = Some(ctx);

        if ctx == 0 {
            Some(Priv::Machine)
        } else if ctx == 1 {
            Some(Priv::Supervisor)
        } else {
            unimplemented!();
        }
    }

    #[inline]
    pub fn lower_interrupt(&mut self) -> IRQ {
        let irq = self.interrupting_irq.unwrap();

        self.unset_pending(irq);

        irq
    }
}

impl Cpu {
    #[inline]
    pub fn handle_plic_read(&mut self, offset: u32) -> Result<u32> {
        match offset {
            PLIC_ENABLE_BASE..PLIC_ENABLE_END => {
                let offset = (offset - PLIC_ENABLE_BASE) as usize;

                let ctx_idx = offset / PLIC_ENABLE_UNIT as usize;
                let idx = (offset % PLIC_ENABLE_UNIT as usize) / 4;

                if ctx_idx > 1 {
                    unreachable!();
                }

                Ok(self.plic.enables[ctx_idx][idx])
            }
            PLIC_THREADSHOLD_BASE..PLIC_CLAIM_END => {
                let idx = ((offset - PLIC_THREADSHOLD_BASE) / PLIC_THREADSHOLD_UNIT) as usize;

                if idx > 1 {
                    unimplemented!();
                }

                let filter = offset & 0x4;

                if filter == 4 {
                    // Claim

                    if self.plic.interrupting_irq.is_none() {
                        return Ok(0);
                    }

                    let irq = self.plic.lower_interrupt();

                    Ok(irq as u32)
                } else {
                    // Threashold
                    Ok(self.plic.threasholds[idx])
                }
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn handle_plic_write(&mut self, offset: u32, value: u32) -> Result<()> {
        match offset {
            PLIC_PRIORITY_BASE..PLIC_PRIORITY_END => {
                let idx = (offset - PLIC_PRIORITY_BASE) as usize / 4;

                self.plic.priories[idx] = value;
            }
            PLIC_ENABLE_BASE..PLIC_ENABLE_END => {
                let offset = (offset - PLIC_ENABLE_BASE) as usize;

                let ctx_idx = offset / PLIC_ENABLE_UNIT as usize;
                let idx = (offset % PLIC_ENABLE_UNIT as usize) / 4;

                if ctx_idx > 1 {
                    unimplemented!();
                }

                self.plic.enables[ctx_idx][idx] = value;
            }
            PLIC_THREADSHOLD_BASE..PLIC_CLAIM_END => {
                let idx = ((offset - PLIC_THREADSHOLD_BASE) / PLIC_THREADSHOLD_UNIT) as usize;

                if idx > 1 {
                    unimplemented!();
                }

                let filter = offset & 0x4;

                if filter == 4 {
                    // Completion
                    let irq = self.plic.interrupting_irq.unwrap();

                    if irq as u32 == value {
                        let ctx = self.plic.interrupting_ctx.unwrap();

                        self.plic.interrupting_ctx = None;
                        self.plic.interrupting_irq = None;

                        if ctx == 0 {
                            self.csr.set_mip_meip(0);
                        } else if ctx == 1 {
                            self.csr.set_mip_seip(0);
                        } else {
                            unreachable!();
                        }
                    }
                } else {
                    // Threashold
                    self.plic.threasholds[idx] = value;
                }
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    #[inline]
    pub fn raise_irq(&mut self, irq: IRQ) {
        self.plic.set_pending(irq);
    }

    #[inline]
    pub fn raise_plic_interrupt(&mut self) {
        if let Some(prv) = self.plic.raise_interrupt() {
            match prv {
                Priv::Machine => self.csr.set_mip_meip(1),
                Priv::Supervisor => self.csr.set_mip_seip(1),
                _ => unreachable!(),
            }
        }
    }

    #[inline]
    pub fn prepare_plic_interrupt_trap(&mut self) {
        let irq = self.plic.interrupting_irq.unwrap();

        match irq {
            IRQ::UART => self.uart.take_interrupt(),
            IRQ::None => unreachable!(),
        }
    }
}
