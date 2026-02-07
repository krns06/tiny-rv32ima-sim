use std::{
    mem::transmute,
    sync::mpsc::{Receiver, Sender},
};

use crate::{
    bus::{
        ExternalDevice, ExternalDeviceResponse, ExternalDeviceResult,
        virtio_mmio::{VIRTIO_REG_CONFIG, VIRTIO_REG_STATUS, VirtioMmio, VirtioType, read_panic},
    },
    memory::Memory,
};

const VIRTIO_NET_HEADER_SIZE: usize = size_of::<VirtioNetHeader>();

const VIRTIO_NET_RECV_IDX: u32 = 0;
const VIRTIO_NET_TRANS_IDX: u32 = 1;

const FEATURES: [u32; 4] = [1 << 5, 1, 0, 0];
const MAC_ADDRESS: [u8; 6] = [2, 0, 0, 1, 2, 3];
const MAX_QUEUE_SIZE: usize = 256;

#[derive(Debug)]
pub struct VirtioNet {
    virtio: VirtioMmio,

    last_idxes: [u16; 2],

    input_rx: Option<Receiver<Vec<u8>>>, //[todo] 将来的にはここは変更しないといけない
    output_tx: Sender<Vec<u8>>,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct VirtioNetHeader {
    flags: u8,
    gos_type: u8,
    hdr_len: u16,
    gso_size: u16,
    csum_start: u16,
    csum_offset: u16,
    num_buffers: u16,
}

impl ExternalDevice for VirtioNet {
    #[inline]
    fn read(&mut self, offset: u32, size: u32, _: &mut Memory) -> ExternalDeviceResult<u32> {
        match offset {
            0..VIRTIO_REG_CONFIG => self.virtio.read(offset, size),
            _ => {
                if size != 1 {
                    unimplemented!();
                }

                let offset = (offset - 0x100) as usize;

                let value = match offset {
                    0..6 => MAC_ADDRESS[offset] as u32,
                    _ => read_panic(offset as u32 + 0x100),
                };

                Ok(ExternalDeviceResponse {
                    value,
                    is_interrupting: false,
                })
            }
        }
    }

    #[inline]
    fn write(
        &mut self,
        offset: u32,
        size: u32,
        value: u32,
        memory: &mut Memory,
    ) -> ExternalDeviceResult<()> {
        match offset {
            0x50 => {
                let is_interrupting = self.handle_notify(value, memory);

                return Ok(ExternalDeviceResponse {
                    value: (),
                    is_interrupting,
                });
            } // Notify
            VIRTIO_REG_STATUS => {
                if value == 0 {
                    self.reset();
                } else {
                    self.virtio.write(offset, size, value)?;
                }
            }
            _ => {
                self.virtio.write(offset, size, value)?;
            }
        };

        Ok(ExternalDeviceResponse {
            value: (),
            is_interrupting: false,
        })
    }

    fn irq(&self) -> crate::IRQ {
        crate::IRQ::VirtioNet
    }

    fn tick(&mut self, memory: &mut Memory) -> bool {
        if !self.virtio.is_ready(VIRTIO_NET_RECV_IDX) {
            return false;
        }

        let rx = self.input_rx.take().unwrap();

        if let Ok(v) = rx.try_recv() {
            let mut header = VirtioNetHeader::default();
            header.num_buffers = 1;

            let driver = self
                .virtio
                .driver::<MAX_QUEUE_SIZE>(VIRTIO_NET_RECV_IDX, memory);

            let device = self
                .virtio
                .device::<MAX_QUEUE_SIZE>(VIRTIO_NET_RECV_IDX, memory);

            let last_idx = self.last_idxes[VIRTIO_NET_RECV_IDX as usize];

            if driver.idx == last_idx {
                // キューが足りない場合
                self.input_rx = Some(rx);
                return false;
            }

            let desc_base = self.virtio.desc_addr(VIRTIO_NET_RECV_IDX);
            let desc_idx = driver.ring[last_idx as usize % MAX_QUEUE_SIZE];
            let desc = self.virtio.desc(desc_idx, desc_base, memory);

            let data_size = v.len() + VIRTIO_NET_HEADER_SIZE as usize;

            if data_size > desc.len as usize {
                panic!("[ERROR]: size of packet is more than desc.len.");
            }

            let data_ptr = memory.raw_mut_ptr(desc.addr as usize, desc.len as usize);
            let header_data: &[u8; VIRTIO_NET_HEADER_SIZE] =
                unsafe { transmute(&header as *const _) };

            data_ptr[..VIRTIO_NET_HEADER_SIZE].copy_from_slice(header_data);
            data_ptr[VIRTIO_NET_HEADER_SIZE..data_size].copy_from_slice(&v);

            device.elems[last_idx as usize % MAX_QUEUE_SIZE].id = desc_idx as u32;
            device.elems[last_idx as usize % MAX_QUEUE_SIZE].len = data_size as u32;

            device.idx = device.idx.wrapping_add(1);
            self.last_idxes[VIRTIO_NET_RECV_IDX as usize] = last_idx.wrapping_add(1);

            self.input_rx = Some(rx);
            return true;
        }

        self.input_rx = Some(rx);

        false
    }
}

impl VirtioNet {
    pub fn new(input_rx: Receiver<Vec<u8>>, output_tx: Sender<Vec<u8>>) -> Self {
        // MACとVIRTIO_F_VERSION_1
        // キューは送信用と受信用
        let virtio = VirtioMmio::new(VirtioType::Network, FEATURES, 2, MAX_QUEUE_SIZE as u32);

        Self {
            virtio,
            last_idxes: [0; 2],
            input_rx: Some(input_rx),
            output_tx,
        }
    }

    fn reset(&mut self) {
        let input_rx = self.input_rx.take();
        let output_tx = self.output_tx.clone();

        *self = Self::new(input_rx.unwrap(), output_tx);
    }

    // notifyを処理する関数
    // interruptが発生する場合はtrueを返す
    fn handle_notify(&mut self, queue_idx: u32, memory: &mut Memory) -> bool {
        let driver = self.virtio.driver::<MAX_QUEUE_SIZE>(queue_idx, memory);
        let device = self.virtio.device::<MAX_QUEUE_SIZE>(queue_idx, memory);

        let last_idx = self.last_idxes[queue_idx as usize];

        eprintln!("[NOTIFY]");

        match queue_idx {
            0 => {
                // 受信用
            }
            1 => {
                // 送信用
                if driver.idx == last_idx {
                    return false;
                }

                let now_driver_idx = driver.idx;

                let diff = driver.idx.wrapping_sub(last_idx);

                let desc_base = self.virtio.desc_addr(queue_idx);

                for i in 0..diff {
                    let ring_idx = last_idx.wrapping_add(i) as usize % MAX_QUEUE_SIZE;
                    let desc_idx = driver.ring[ring_idx];

                    let desc = self.virtio.desc(desc_idx, desc_base, memory);

                    if desc.is_next() {
                        unimplemented!();
                    }

                    let data_ptr = memory.raw_ptr(desc.addr as usize, desc.len as usize);
                    let virtio_net_header: &VirtioNetHeader =
                        unsafe { transmute(data_ptr.as_ptr()) };

                    if virtio_net_header.num_buffers != 0 {
                        eprintln!(
                            "[WARNING]: virtio_net_header.num_buffers is expected 0 but writen is {}.",
                            virtio_net_header.num_buffers
                        );
                    }

                    let data = &data_ptr[VIRTIO_NET_HEADER_SIZE..];
                    self.output_tx.send(data.to_vec()).unwrap();

                    device.elems[ring_idx].len = 0;
                    device.elems[ring_idx].id = desc_idx as u32;
                    device.idx = device.idx.wrapping_add(1);
                }

                self.last_idxes[queue_idx as usize] = now_driver_idx;

                return true;
            }
            _ => unreachable!(),
        }

        false
    }
}
