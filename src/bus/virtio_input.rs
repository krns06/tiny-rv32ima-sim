use std::mem::transmute;

use crate::{
    IRQ,
    bus::virtio_mmio::{
        VIRTIO_REG_CONFIG, VIRTIO_REG_NOTIFY, VIRTIO_REG_STATUS, VirtioMmio, VirtioType,
        read_panic, write_panic,
    },
    device::{DeviceMessage, DeviceRecieverTrait, DeviceResponse, DeviceTrait},
    memory::Memory,
};

const VIRTIO_INPUT_CFG_ID_NAME: (u8, u8) = (1, 0); // select, subsel
const VIRTIO_INPUT_CFG_ID_SERIAL: (u8, u8) = (2, 0);
const VIRTIO_INPUT_CFG_ID_DEVIDS: (u8, u8) = (3, 0);
const VIRTIO_INPUT_CFG_PROP_BITS: (u8, u8) = (0x10, 0);

const VIRTIO_INPUT_CFG_EV_BITS_0: u8 = 0x11;

const VIRTIO_INPUT_EVENT_SIZE: usize = size_of::<VirtioInputEvent>();

const VIRTIO_INPUT_EVENT_IDX: u32 = 0;
const VIRTIO_INPUT_STATUS_IDX: u32 = 1;

// Linux固有の値っぽい
const EV_SYN: u8 = 0x0;
const EV_KEY: u8 = 0x1;
const EV_REL: u8 = 0x2;
const EV_ABS: u8 = 0x3;
const EV_MSC: u8 = 0x4;
const EV_SW: u8 = 0x5;
const EV_LED: u8 = 0x11;
const EV_SND: u8 = 0x12;
const EV_REP: u8 = 0x14;

const FEATURES: [u32; 4] = [0, 1, 0, 0];
const MAX_QUEUE_SIZE: usize = 256;
const SUPPORT_KEY_LEN: u32 = 136 / 8; // サポートするキーコードの数。F24までサポート
const SUPPORT_BIT_MAP: [u8; SUPPORT_KEY_LEN as usize] = [0xff; SUPPORT_KEY_LEN as usize];

#[derive(Debug)]
pub struct VirtioInput<R>
where
    R: DeviceRecieverTrait,
{
    virtio: VirtioMmio,

    last_idxes: [u16; 2],

    name_bytes: Vec<u8>,

    select: u8,
    subsel: u8,

    reciever: R,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtioInputEvent {
    event_type: u16,
    code: u16,
    value: u32,
}

impl<R: DeviceRecieverTrait> DeviceTrait for VirtioInput<R> {
    fn read(
        &mut self,
        offset: u32,
        size: u32,
        memory: &mut crate::memory::Memory,
    ) -> crate::device::DeviceResult<u32> {
        match offset {
            0..VIRTIO_REG_CONFIG => self.virtio.read(offset, size),
            VIRTIO_REG_CONFIG.. => {
                if size != 1 {
                    unimplemented!();
                }

                let offset = offset - VIRTIO_REG_CONFIG;

                let value = match offset {
                    2 => match (self.select, self.subsel) {
                        VIRTIO_INPUT_CFG_ID_NAME => self.name_bytes.len() as u32,
                        VIRTIO_INPUT_CFG_ID_SERIAL => 0,
                        VIRTIO_INPUT_CFG_ID_DEVIDS => 0,
                        VIRTIO_INPUT_CFG_PROP_BITS => 0,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_KEY) => SUPPORT_KEY_LEN,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_REL) => 0,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_ABS) => 0,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_MSC) => 0,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_SW) => 0,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_LED) => 0,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_SND) => 0,
                        (VIRTIO_INPUT_CFG_EV_BITS_0, EV_REP) => 0,
                        _ => panic!(
                            "[VIRTIO_INPUT]: Reading size by select({}) and subsel({}) is not implemented.",
                            self.select, self.subsel
                        ),
                    },
                    8..=136 => {
                        let offset = offset - 8;

                        match (self.select, self.subsel) {
                            VIRTIO_INPUT_CFG_ID_NAME => self.name_bytes[offset as usize] as u32,
                            (VIRTIO_INPUT_CFG_EV_BITS_0, EV_KEY) => {
                                if offset > SUPPORT_KEY_LEN {
                                    0
                                } else {
                                    SUPPORT_BIT_MAP[offset as usize] as u32
                                }
                            }
                            _ => panic!(
                                "[VIRTIO_INPUT]: Reading value by select({}) and subsel({}) is not implemented.",
                                self.select, self.subsel
                            ),
                        }
                    }
                    _ => read_panic(offset + VIRTIO_REG_CONFIG),
                };

                Ok(DeviceResponse {
                    value,
                    is_interrupting: false,
                })
            }
            _ => read_panic(offset),
        }
    }

    fn write(
        &mut self,
        offset: u32,
        size: u32,
        value: u32,
        memory: &mut crate::memory::Memory,
    ) -> crate::device::DeviceResult<()> {
        match offset {
            VIRTIO_REG_STATUS => {
                if value == 0 {
                    self.reset();
                } else {
                    self.virtio.write(offset, size, value)?;
                }
            }
            VIRTIO_REG_NOTIFY => {
                let is_interrupting = self.handle_notify(value, memory);

                return Ok(DeviceResponse {
                    value: (),
                    is_interrupting,
                });
            }
            VIRTIO_REG_CONFIG.. => {
                if size != 1 {
                    unimplemented!();
                }

                let offset = offset - VIRTIO_REG_CONFIG;

                match offset {
                    0 => self.select = value as u8,
                    1 => self.subsel = value as u8,
                    _ => write_panic(offset + VIRTIO_REG_CONFIG, value),
                }
            }
            _ => {
                self.virtio.write(offset, size, value)?;
            }
            _ => read_panic(offset),
        };

        Ok(DeviceResponse {
            value: (),
            is_interrupting: false,
        })
    }

    fn irq(&self) -> crate::IRQ {
        IRQ::VirtioInput
    }

    fn tick(&mut self, memory: &mut Memory) -> bool {
        if !self.virtio.is_ready(VIRTIO_INPUT_EVENT_IDX) {
            return false;
        }

        if let Ok(DeviceMessage::Input(e)) = self.reciever.try_recv_from_host() {
            eprintln!("{:?}", e);
            let driver = self
                .virtio
                .driver::<MAX_QUEUE_SIZE>(VIRTIO_INPUT_EVENT_IDX, memory);

            let device = self
                .virtio
                .device::<MAX_QUEUE_SIZE>(VIRTIO_INPUT_EVENT_IDX, memory);

            let last_idx = self.last_idxes[VIRTIO_INPUT_EVENT_IDX as usize];

            let diff = driver.idx.wrapping_sub(last_idx);

            if diff < 2 {
                unimplemented!();
            }

            let events = [
                VirtioInputEvent {
                    event_type: e.event_type as u16,
                    code: e.code,
                    value: e.value,
                },
                VirtioInputEvent {
                    event_type: EV_SYN as u16,
                    code: 0,
                    value: 0,
                },
            ];

            for i in 0..2 {
                let last_idx = last_idx.wrapping_add(i);

                let desc_base = self.virtio.desc_addr(VIRTIO_INPUT_EVENT_IDX);
                let desc_idx = driver.ring[last_idx as usize % MAX_QUEUE_SIZE];
                let desc = self.virtio.desc(desc_idx, desc_base, memory);

                if VIRTIO_INPUT_EVENT_SIZE > desc.len as usize {
                    panic!("[ERROR]: size of input is more than desc.len");
                }

                let data_ptr = memory.raw_mut_ptr(desc.addr as usize, desc.len as usize);

                let input_event_data: &[u8; VIRTIO_INPUT_EVENT_SIZE] =
                    unsafe { transmute(&events[i as usize] as *const _) };
                data_ptr[..VIRTIO_INPUT_EVENT_SIZE].copy_from_slice(input_event_data);

                device.elems[last_idx as usize % MAX_QUEUE_SIZE].id = desc_idx as u32;
                device.elems[last_idx as usize % MAX_QUEUE_SIZE].len =
                    VIRTIO_INPUT_EVENT_SIZE as u32;

                device.idx = device.idx.wrapping_add(1);
                self.last_idxes[VIRTIO_INPUT_EVENT_IDX as usize] = last_idx.wrapping_add(1);
            }

            return true;
        }

        false
    }
}

impl<R: DeviceRecieverTrait> VirtioInput<R> {
    pub fn new(name: &str, reciever: R) -> Self {
        if name.len() > 127 {
            panic!("[VIRTIO_INPUT]: len of name({}) must be under 127.", name);
        }

        let virtio = VirtioMmio::new(VirtioType::Input, FEATURES, 2, MAX_QUEUE_SIZE as u32);

        let mut name_bytes = vec![0; name.len() + 1];
        name_bytes[..name.len()].copy_from_slice(name.as_bytes());

        Self {
            virtio,
            last_idxes: [0; 2],
            name_bytes,
            select: 0,
            subsel: 0,
            reciever,
        }
    }
    fn reset(&mut self) {
        let virtio = VirtioMmio::new(VirtioType::Input, FEATURES, 2, MAX_QUEUE_SIZE as u32);

        self.virtio = virtio;

        let reciever = std::mem::take(&mut self.reciever);
        self.reciever = reciever;

        self.last_idxes = [0; 2];

        self.select = 0;
        self.subsel = 0;
    }

    fn handle_notify(&mut self, queue_idx: u32, _: &mut Memory) -> bool {
        if queue_idx == VIRTIO_INPUT_STATUS_IDX {
            unimplemented!();
        }

        false
    }
}
