use crate::AccessType;

const TLB_SIZE: usize = 512;
const TLB_MASK: u32 = (TLB_SIZE - 1) as u32;

pub struct Tlb {
    read_entries: [Option<TlbEntry>; TLB_SIZE],
    write_entries: [Option<TlbEntry>; TLB_SIZE],
    exec_entries: [Option<TlbEntry>; TLB_SIZE],
}

#[derive(Clone, Copy)]
pub struct TlbEntry {
    ppn: u32,
    vpn: u32,
}

impl Default for Tlb {
    fn default() -> Self {
        Self {
            read_entries: [None; TLB_SIZE],
            write_entries: [None; TLB_SIZE],
            exec_entries: [None; TLB_SIZE],
        }
    }
}

impl Tlb {
    pub fn register_entry(&mut self, entry: TlbEntry, access_type: AccessType) {
        let vpn = entry.vpn;
        let i = (vpn & TLB_MASK) as usize;

        match access_type {
            AccessType::Read => self.read_entries[i] = Some(entry),
            AccessType::Write => self.write_entries[i] = Some(entry),
            AccessType::Fetch => self.exec_entries[i] = Some(entry),
        }
    }

    pub fn lookup_ppn(&self, va: u32, access_type: AccessType) -> Option<&TlbEntry> {
        let vpn = va >> 12;
        let i = (vpn & TLB_MASK) as usize;

        let entry = match access_type {
            AccessType::Read => &self.read_entries[i],
            AccessType::Write => &self.write_entries[i],
            AccessType::Fetch => &self.exec_entries[i],
        };

        if let Some(entry) = entry {
            if entry.vpn == vpn {
                return Some(entry);
            }
        }

        None
    }

    pub fn clear(&mut self) {
        self.read_entries = [None; TLB_SIZE];
        self.write_entries = [None; TLB_SIZE];
        self.exec_entries = [None; TLB_SIZE];
    }
}

impl TlbEntry {
    pub fn new(va: u32, ppn: u32) -> Self {
        Self { ppn, vpn: va >> 12 }
    }

    pub fn ppn(&self) -> u32 {
        self.ppn
    }
}
