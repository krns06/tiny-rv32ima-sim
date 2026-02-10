use std::mem::transmute;

use crate::{
    bus::{ExternalDeviceResponse, ExternalDeviceResult},
    memory::Memory,
};

pub const VIRTIO_REG_QUEUE_READY: u32 = 0x44;
pub const VIRTIO_REG_NOTIFY: u32 = 0x50;
pub const VIRTIO_REG_STATUS: u32 = 0x70;
pub const VIRTIO_REG_CONFIG: u32 = 0x100;

pub const VIRTIO_QUEUE_DESC_SIZE: usize = size_of::<VirtQueueDesc>();
pub const VIRTIO_QUEUE_DRIVER_BASE_SIZE: usize = 4;
pub const VIRTIO_QUEUE_DRIVER_RING_SIZE: usize = 2;
pub const VIRTIO_QUEUE_DEVICE_BASE_SIZE: usize = 4;
pub const VIRTIO_QUEUE_DEVICE_ELEM_SIZE: usize = size_of::<VirtQueueDeviceElem>();

#[derive(Debug, Clone, Copy)]
pub enum VirtioType {
    Network = 1,
    Gpu = 16,
}

type FeatureType = [u32; 4];

// MMIOでのVirtioの共通部分について処理を行う構造体
#[derive(Debug)]
pub struct VirtioMmio {
    device_type: VirtioType,
    status: u32,
    features_sel: usize,
    features_supported: FeatureType, // 128以降はサポートしない
    driver_features_sel: usize,
    driver_features: FeatureType,
    queue_sel: usize,
    queue_size_max: u32,
    queue_sizes: Vec<u32>,
    readies: Vec<bool>,
    desc_addrs: Vec<u64>,
    driver_addrs: Vec<u64>,
    device_addrs: Vec<u64>,
    shm_sel: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtQueueDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtQueueDriver<const L: usize> {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; L],
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtQueueDevice<const L: usize> {
    pub flags: u16,
    pub idx: u16,
    pub elems: [VirtQueueDeviceElem; L],
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtQueueDeviceElem {
    pub id: u32,
    pub len: u32,
}

impl VirtioMmio {
    pub fn new(
        device_type: VirtioType,
        features_supported: FeatureType,
        queue_num: usize,
        queue_size_max: u32,
    ) -> Self {
        let readies = vec![false; queue_num];
        let queue_sizes = vec![0; queue_num];

        let desc_addrs = vec![0; queue_num];
        let driver_addrs = vec![0; queue_num];
        let device_addrs = vec![0; queue_num];

        Self {
            device_type,
            status: 0,
            features_sel: 0,
            features_supported,
            driver_features_sel: 0,
            driver_features: [0; 4],
            queue_sel: 0,
            queue_size_max,
            queue_sizes,
            readies,
            desc_addrs,
            driver_addrs,
            device_addrs,
            shm_sel: 0,
        }
    }

    #[inline]
    pub fn read(&mut self, offset: u32, size: u32) -> ExternalDeviceResult<u32> {
        if size != 4 {
            unimplemented!();
        }

        let value = match offset {
            0 => 0x74726976,                                    // Magic Value
            0x4 => 2,                                           // Vertion
            0x8 => self.device_type as u32,                     // Device ID
            0x10 => self.features_supported[self.features_sel], // Device Features
            0xc => 0,                                           // Vendor ID
            0x34 => self.queue_size_max,
            VIRTIO_REG_QUEUE_READY => {
                if self.readies[self.queue_sel] {
                    1
                } else {
                    0
                }
            }
            0x60 => 1, // Interrupt Status
            VIRTIO_REG_STATUS => self.status,
            0xfc => 0, // Config Generation 設定を変更する場合は要変更
            _ => read_panic(offset),
        };

        Ok(ExternalDeviceResponse {
            value,
            is_interrupting: false,
        })
    }

    #[inline]
    pub fn write(&mut self, offset: u32, size: u32, value: u32) -> ExternalDeviceResult<()> {
        if size != 4 {
            unimplemented!()
        }

        match offset {
            0x14 => self.features_sel = value as usize, // Device Features Sel
            0x20 => {
                if value != self.features_supported[self.driver_features_sel] {
                    self.set_failed();
                } else {
                    self.driver_features[self.driver_features_sel] = value;
                }
            } // Driver Features
            0x24 => self.driver_features_sel = value as usize, // Driver Features Sel
            0x30 => self.queue_sel = value as usize,
            0x38 => {
                if value != self.queue_size_max {
                    eprintln!(
                        "[WARNING]: max queue_size of {} queue is {} but writen is  {}.",
                        self.queue_sel, self.queue_size_max, value
                    );
                }

                self.queue_sizes[self.queue_sel] = value
            }
            VIRTIO_REG_QUEUE_READY => {
                self.readies[self.queue_sel] = match value {
                    0 => false,
                    1 => true,
                    _ => write_panic(offset, value),
                };
            }
            0x64 => {
                if value != 1 {
                    unimplemented!()
                }
            } // Interrupt ACK
            VIRTIO_REG_STATUS => match value {
                1 | 3 | 0xb | 0xf => self.status = value, // ACK, DRIVER, Features OK
                _ => write_panic(offset, value),
            },
            0x80 => self.desc_addrs[self.queue_sel] = value as u64, // Queue Desc Low
            0x84 => {
                if value != 0 {
                    // 32bit only
                    unimplemented!();
                }
            } // Queue Desc High
            0x90 => self.driver_addrs[self.queue_sel] = value as u64, // Queue Driver Low
            0x94 => {
                if value != 0 {
                    // 32bit only
                    unimplemented!();
                }
            } // Queue Driver High
            0xa0 => self.device_addrs[self.queue_sel] = value as u64, // Queue Device Low
            0xa4 => {
                if value != 0 {
                    // 32bit only
                    unimplemented!();
                }
            } // Queue Device High
            0xac => self.shm_sel = value,
            _ => write_panic(offset, value),
        }

        Ok(ExternalDeviceResponse {
            value: (),
            is_interrupting: false,
        })
    }

    pub fn set_failed(&mut self) {
        self.status |= 1 << 7;
    }

    pub fn desc_addr(&self, queue_idx: u32) -> usize {
        self.desc_addrs[queue_idx as usize] as usize
    }

    pub fn driver_addr(&self, queue_idx: u32) -> usize {
        self.driver_addrs[queue_idx as usize] as usize
    }

    pub fn device_addr(&self, queue_idx: u32) -> usize {
        self.device_addrs[queue_idx as usize] as usize
    }

    pub fn shm_sel(&self) -> usize {
        self.shm_sel as usize
    }

    pub fn driver<const L: usize>(
        &self,
        queue_idx: u32,
        memory: &mut Memory,
    ) -> &VirtQueueDriver<L> {
        let driver_ptr = memory.raw_ptr(
            self.driver_addr(queue_idx),
            VIRTIO_QUEUE_DRIVER_BASE_SIZE + VIRTIO_QUEUE_DRIVER_RING_SIZE * L,
        );

        unsafe { transmute(driver_ptr.as_ptr()) }
    }

    pub fn device<const L: usize>(
        &self,
        queue_idx: u32,
        memory: &mut Memory,
    ) -> &mut VirtQueueDevice<L> {
        let device_ptr = memory.raw_mut_ptr(
            self.device_addr(queue_idx),
            VIRTIO_QUEUE_DEVICE_BASE_SIZE + VIRTIO_QUEUE_DEVICE_ELEM_SIZE * L,
        );

        unsafe { transmute(device_ptr.as_ptr()) }
    }

    pub fn desc(&self, desc_idx: u16, desc_base: usize, memory: &mut Memory) -> &VirtQueueDesc {
        let desc_ptr = memory.raw_ptr(
            desc_base + calc_desc_offset(desc_idx as usize),
            VIRTIO_QUEUE_DESC_SIZE,
        );
        unsafe { transmute(desc_ptr.as_ptr()) }
    }

    pub fn is_ready(&self, queue_idx: u32) -> bool {
        self.readies[queue_idx as usize]
    }
}

impl VirtQueueDesc {
    pub fn is_next(&self) -> bool {
        self.flags & 1 != 0
    }

    pub fn is_write_only(&self) -> bool {
        self.flags & 2 != 0
    }

    pub fn is_indirect(&self) -> bool {
        self.flags & 4 != 0
    }
}

pub fn calc_desc_offset(desc_idx: usize) -> usize {
    VIRTIO_QUEUE_DESC_SIZE * desc_idx
}

pub fn read_panic(offset: u32) -> ! {
    panic!(
        "[VIRTIO]: Reading offset 0x{:x?} is not implemented.",
        offset
    )
}

pub fn write_panic(offset: u32, value: u32) -> ! {
    panic!(
        "[VIRTIO]: Writing offset 0x{:x?} with 0x{:x?} is not implemented.",
        offset, value
    )
}
