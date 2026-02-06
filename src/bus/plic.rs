use crate::{IRQ, Priv, Result, csr::Csr};

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
    pending: [u32; (PLIC_NUM / 32) as usize],
    enables: [[u32; (PLIC_NUM / 32) as usize]; 2],
    threasholds: [u32; PLIC_CONTEXT_NUM as usize],

    interrupting_irq: Option<IRQ>,
    interrupting_ctx: Option<usize>,
}

impl Plic {
    #[inline]
    pub fn read(&mut self, offset: u32, size: u32, _: &mut Csr) -> Result<u32> {
        if size != 4 {
            unimplemented!()
        }

        match offset {
            PLIC_ENABLE_BASE..PLIC_ENABLE_END => {
                let offset = (offset - PLIC_ENABLE_BASE) as usize;

                let ctx_idx = offset / PLIC_ENABLE_UNIT as usize;
                let idx = (offset % PLIC_ENABLE_UNIT as usize) / 4;

                if ctx_idx > 1 {
                    unreachable!();
                }

                Ok(self.enables[ctx_idx][idx])
            }
            PLIC_THREADSHOLD_BASE..PLIC_CLAIM_END => {
                let idx = ((offset - PLIC_THREADSHOLD_BASE) / PLIC_THREADSHOLD_UNIT) as usize;

                if idx > 1 {
                    unimplemented!();
                }

                let filter = offset & 0x4;

                if filter == 4 {
                    // Claim

                    if self.interrupting_irq.is_none() {
                        return Ok(0);
                    }

                    let irq = self.lower_interrupt();

                    Ok(irq as u32)
                } else {
                    // Threashold
                    Ok(self.threasholds[idx])
                }
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write(&mut self, offset: u32, size: u32, value: u32, csr: &mut Csr) -> Result<()> {
        if size != 4 {
            unimplemented!();
        }

        match offset {
            PLIC_PRIORITY_BASE..PLIC_PRIORITY_END => {
                let idx = (offset - PLIC_PRIORITY_BASE) as usize / 4;

                self.priories[idx] = value;
            }
            PLIC_ENABLE_BASE..PLIC_ENABLE_END => {
                let offset = (offset - PLIC_ENABLE_BASE) as usize;

                let ctx_idx = offset / PLIC_ENABLE_UNIT as usize;
                let idx = (offset % PLIC_ENABLE_UNIT as usize) / 4;

                if ctx_idx > 1 {
                    unimplemented!();
                }

                self.enables[ctx_idx][idx] = value;
            }
            PLIC_THREADSHOLD_BASE..PLIC_CLAIM_END => {
                let idx = ((offset - PLIC_THREADSHOLD_BASE) / PLIC_THREADSHOLD_UNIT) as usize;

                if idx > 1 {
                    unimplemented!();
                }

                let filter = offset & 0x4;

                if filter == 4 {
                    // Completion
                    let irq = self.interrupting_irq.unwrap();

                    if irq as u32 == value {
                        let i_ctx = self.interrupting_ctx.unwrap();

                        self.interrupting_ctx = None;
                        self.interrupting_irq = None;

                        if i_ctx == 0 {
                            csr.set_mip_meip(0);
                        } else if i_ctx == 1 {
                            csr.set_mip_seip(0);
                        } else {
                            unreachable!();
                        }
                    }
                } else {
                    // Threashold
                    self.threasholds[idx] = value;
                }
            }
            _ => unreachable!(),
        }

        Ok(())
    }
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
    fn unset_pending(&mut self, irq: IRQ) {
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

    #[inline]
    pub fn interrupting_irq(&self) -> Option<IRQ> {
        self.interrupting_irq
    }
}
