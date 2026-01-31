use std::io::Write;

use crate::{Result, bus::MmioOps};

const IER_ERBFI: u8 = 1; // 受け取ったときの例外のIEのbit
const IER_ETBEI: u8 = 0x2; // 出力したときの例外のIEのbit

const IIR_NIP: u8 = 1;
const IIR_THRE: u8 = 0x2;
const IIR_RDA: u8 = 0x4;
const IIR_ID: u8 = 0x6;

const LSR_THRE: u8 = 1 << 5;
const LSR_TEMT: u8 = 1 << 6;
const LSR_DR: u8 = 1;

#[derive(Debug)]
pub struct Uart {
    lcr: u8,
    dlm: u8,
    dll: u8,
    lsr: u8,
    ier: u8,
    rbr: u8,
    iir: u8,

    is_interrupting: bool,
    is_taken_interrupt: bool,
}

impl Default for Uart {
    fn default() -> Self {
        Uart {
            lcr: 0,
            dlm: 0,
            dll: 0,
            lsr: LSR_TEMT | LSR_THRE,
            ier: 0,
            rbr: 0,
            iir: IIR_NIP,
            is_interrupting: false,
            is_taken_interrupt: false,
        }
    }
}

impl MmioOps for Uart {
    #[inline]
    fn read(&mut self, _: u32, _: u32, _: crate::bus::CpuContext) -> Result<Vec<u8>> {
        unreachable!();
    }

    #[inline]
    fn write(&mut self, _: u32, _: &[u8], _: crate::bus::CpuContext) -> Result<()> {
        unreachable!();
    }

    #[inline]
    fn read_u8(&mut self, offset: u32, _: crate::bus::CpuContext) -> Result<u8> {
        let offset = offset & 0xFF;

        let v = match offset {
            0 => {
                if self.is_dlab_enabled() {
                    // DLL
                    self.dll
                } else {
                    // RBR
                    let rbr = self.rbr;

                    self.rbr = 0;
                    self.lsr &= !LSR_DR;

                    if self.is_interrupting {
                        self.lower_interrupt();
                    }

                    rbr
                }
            }
            1 => {
                if self.is_dlab_enabled() {
                    // DLM
                    self.dlm
                } else {
                    //IER
                    self.ier
                }
            }
            2 => {
                // IIR

                let iir = self.iir;

                if self.is_taken_interrupt {
                    if iir & IIR_ID == IIR_THRE {
                        self.lower_interrupt();
                    }
                }

                iir
            }
            3 => self.lcr,
            5 => self.lsr,
            _ => 0,
        };

        Ok(v)
    }

    #[inline]
    fn write_u8(&mut self, offset: u32, value: u8, _: crate::bus::CpuContext) -> Result<()> {
        let offset = offset & 0xFF;

        match offset {
            0 => {
                if self.is_dlab_enabled() {
                    // DLL
                    self.dll = value;
                } else {
                    // THR
                    let c = value as u8;
                    print!("{}", c as char);
                    std::io::stdout().flush().unwrap();

                    if self.ier & IER_ETBEI != 0 {
                        self.raise_interrupt(IIR_THRE);
                    }
                }
            }
            1 => {
                if self.is_dlab_enabled() {
                    // DLM
                    self.dlm = value;
                } else {
                    //IER
                    let changed = (self.ier ^ value) & 0xf;
                    self.ier = value;

                    if changed & IER_ETBEI != 0 {
                        if self.ier & IER_ETBEI != 0 {
                            self.raise_interrupt(IIR_THRE);
                        } else {
                            self.lower_interrupt();
                        }
                    }
                }
            }
            3 => {
                // LCR
                self.lcr = value;
            }
            _ => {}
        }

        Ok(())
    }
}

impl Uart {
    #[inline]
    fn is_dlab_enabled(&self) -> bool {
        self.lcr >> 7 == 1
    }

    #[inline]
    pub fn push_char(&mut self, c: char) {
        self.rbr = c as u8;
        self.lsr |= LSR_DR;
        self.raise_interrupt(IIR_RDA);
    }

    #[inline]
    fn raise_interrupt(&mut self, iir: u8) {
        self.is_interrupting = true;
        self.is_taken_interrupt = false;
        self.iir = iir;
    }

    #[inline]
    fn lower_interrupt(&mut self) {
        self.is_interrupting = false;
        self.iir = IIR_NIP;
        self.lsr = LSR_THRE | LSR_TEMT;
    }

    #[inline]
    pub fn is_interrupting(&self) -> bool {
        self.is_interrupting
    }

    #[inline]
    pub fn take_interrupt(&mut self) {
        self.is_taken_interrupt = true;
    }

    #[inline]
    pub fn is_ready_for_recieving(&self) -> bool {
        self.iir == 1 && self.ier & 0x4 != 0
    }
}
