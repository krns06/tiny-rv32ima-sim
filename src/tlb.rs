use crate::Priv;

const TLB_SIZE: usize = 4096;
const TLB_MASK: u32 = (TLB_SIZE - 1) as u32;

pub struct Tlb {
    entries: [Option<TlbEntry>; TLB_SIZE], // vpnがキーに成る。
}

#[derive(Default, Clone, Copy)]
pub struct TlbEntry {
    prv: Priv,
    ppn: u32,
    vpn: u32,
}

impl Default for Tlb {
    fn default() -> Self {
        Self {
            entries: [None; TLB_SIZE],
        }
    }
}

impl Tlb {
    pub fn register_entry(&mut self, entry: TlbEntry) {
        let vpn = entry.vpn;
        let i = (vpn & TLB_MASK) as usize;

        self.entries[i] = Some(entry);
    }

    pub fn lookup_ppn(&self, va: u32, prv: Priv) -> Option<&TlbEntry> {
        let vpn = va >> 12;
        let i = (vpn & TLB_MASK) as usize;

        if let Some(ref entry) = self.entries[i] {
            if entry.vpn == vpn && entry.prv == prv {
                return Some(entry);
            }
        }

        None
    }

    pub fn clear(&mut self) {
        self.entries = [None; TLB_SIZE];
    }
}

impl TlbEntry {
    pub fn new(va: u32, ppn: u32, prv: Priv) -> Self {
        Self {
            prv,
            ppn,
            vpn: va >> 12,
        }
    }

    pub fn ppn(&self) -> u32 {
        self.ppn
    }
}
