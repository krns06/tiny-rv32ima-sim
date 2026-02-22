use std::mem::transmute;

use crate::{
    DeviceMessage, DeviceSenderTrait,
    bus::{
        DeviceContext, DeviceTrait,
        virtio_mmio::{
            VIRTIO_REG_CONFIG, VIRTIO_REG_NOTIFY, VIRTIO_REG_STATUS, VirtioMmio, VirtioType,
            read_panic,
        },
    },
    memory::Memory,
    plic::Irq,
};

const FEATURES: [u32; 4] = [1 << 9, 1, 0, 0]; // VIRTIO_BLK_F_FLUSH
const MAX_QUEUE_SIZE: usize = 256;

const SECTOR_SIZE: u64 = 512; //[todo] セクターのサイズ。将来的には変更にも対応する。

const VIRTIO_BLK_REQ_SIZE: usize = size_of::<VirtioBlkReq>();

#[derive(Debug)]
pub struct VirtioBlk<S>
where
    S: DeviceSenderTrait,
{
    virtio: VirtioMmio,

    last_idxes: [u16; 1],
    bytes: Vec<u8>,

    sender: S,
}

enum VirtioBlkReqType {
    In = 0,
    Out = 1,
    Flush = 4,
}

// VIRTIO_BLK_T_ZONE_APPEND以外は以下をrequest_descが用いる。
// 以下にdata[]が続く。
#[derive(Debug)]
#[repr(C)]
pub struct VirtioBlkReq {
    request_type: u32,
    _reserved: u32,
    sector: u64,
}

impl<S: DeviceSenderTrait> DeviceTrait for VirtioBlk<S> {
    #[inline]
    fn read(&mut self, offset: u32, size: u32, ctx: DeviceContext) -> Option<u32> {
        match offset {
            0..VIRTIO_REG_CONFIG => self.virtio.read(offset, size),
            _ => {
                let offset = offset - VIRTIO_REG_CONFIG;

                let value = match offset {
                    0 => ((self.bytes_size() + SECTOR_SIZE - 1) / SECTOR_SIZE) as u32, // opacity[0..31](512バイト単位)
                    0x4 => (((self.bytes_size() + SECTOR_SIZE - 1) / SECTOR_SIZE) >> 32) as u32, // opacity[32..63](512バイト単位)
                    _ => read_panic(offset + VIRTIO_REG_CONFIG),
                };

                Some(value)
            }
        }
    }

    #[inline]
    fn write(&mut self, offset: u32, size: u32, value: u32, ctx: DeviceContext) -> Option<()> {
        match offset {
            VIRTIO_REG_STATUS => {
                if value == 0 {
                    self.reset();
                } else {
                    self.virtio.write(offset, size, value)?;
                }
            }
            VIRTIO_REG_NOTIFY => {
                let is_interrupting = self.handle_notify(value, ctx.memory);

                if is_interrupting {
                    ctx.pending_interrupts.push(self.irq());
                }
            }
            _ => {
                self.virtio.write(offset, size, value)?;
            }
        };

        Some(())
    }

    fn irq(&self) -> Irq {
        Irq::VirtioBlk
    }
}

impl<S: DeviceSenderTrait> VirtioBlk<S> {
    pub fn new(sender: S, bytes: Vec<u8>) -> Self {
        let virtio = VirtioMmio::new(VirtioType::Block, FEATURES, 1, MAX_QUEUE_SIZE as u32);

        Self {
            virtio,
            last_idxes: [0],
            sender,
            bytes,
        }
    }
    pub fn reset(&mut self) {
        let sender = std::mem::take(&mut self.sender);
        let bytes = std::mem::take(&mut self.bytes);

        *self = Self::new(sender, bytes);
    }

    #[inline]
    pub fn handle_notify(&mut self, queue_idx: u32, memory: &mut Memory) -> bool {
        if queue_idx != 0 {
            // requestq1出ない場合はとりあえず終了する。
            unimplemented!()
        }

        let last_idx = self.last_idxes[queue_idx as usize];

        let driver = self.virtio.driver::<MAX_QUEUE_SIZE>(queue_idx, memory);

        if driver.idx == last_idx {
            return false;
        }

        let device = self.virtio.device::<MAX_QUEUE_SIZE>(queue_idx, memory);
        let now_driver_idx = driver.idx;

        let diff = driver.idx.wrapping_sub(last_idx);

        let desc_base = self.virtio.desc_addr(queue_idx);

        for i in 0..diff {
            let ring_idx = last_idx.wrapping_add(i) as usize % MAX_QUEUE_SIZE;
            let request_idx = driver.ring[ring_idx];

            let request_desc = self.virtio.desc(request_idx, desc_base, memory);

            if !request_desc.is_next() || VIRTIO_BLK_REQ_SIZE > request_desc.len as usize {
                unimplemented!();
            }

            let second_desc = self.virtio.desc(request_desc.next, desc_base, memory);

            let request_data_ptr =
                memory.raw_ptr(request_desc.addr as usize, request_desc.len as usize);

            let request_data: &VirtioBlkReq = unsafe { transmute(request_data_ptr.as_ptr()) };

            let req_type = VirtioBlkReqType::from(request_data.request_type);

            eprintln!("{:x?}", request_data);

            let len = match req_type {
                VirtioBlkReqType::In => {
                    if !second_desc.is_next() || !second_desc.is_write_only() {
                        // 書き込み用のdesc(second_desc)で次のstatusのdescが存在しない場合は終了する。
                        unimplemented!();
                    }

                    let status_desc = self.virtio.desc(second_desc.next, desc_base, memory);

                    if status_desc.is_next() || !status_desc.is_write_only() {
                        unimplemented!();
                    }

                    let bytes_offset = (request_data.sector * SECTOR_SIZE) as usize;

                    let second_data_ptr =
                        memory.raw_mut_ptr(second_desc.addr as usize, second_desc.len as usize);

                    second_data_ptr[..second_desc.len as usize].copy_from_slice(
                        &self.bytes[bytes_offset..bytes_offset + second_desc.len as usize],
                    );

                    let status_data_ptr =
                        memory.raw_mut_ptr(status_desc.addr as usize, status_desc.len as usize);
                    status_data_ptr[0] = 0; // VIRTIO_BLK_S_OK

                    second_desc.len + 1 // SECTOR_SIZE + 1(status)
                }
                VirtioBlkReqType::Out => {
                    if !second_desc.is_next() || second_desc.is_write_only() {
                        // 書き込み用のdesc(second_desc)で次のstatusのdescが存在しない場合は終了する。
                        unimplemented!();
                    }

                    let status_desc = self.virtio.desc(second_desc.next, desc_base, memory);

                    if status_desc.is_next() || !status_desc.is_write_only() {
                        unimplemented!();
                    }

                    let bytes_offset = (request_data.sector * SECTOR_SIZE) as usize;

                    let second_data_ptr =
                        memory.raw_ptr(second_desc.addr as usize, second_desc.len as usize);

                    self.bytes[bytes_offset..bytes_offset + second_desc.len as usize]
                        .copy_from_slice(&second_data_ptr[..second_desc.len as usize]);

                    let status_data_ptr =
                        memory.raw_mut_ptr(status_desc.addr as usize, status_desc.len as usize);
                    status_data_ptr[0] = 0; // VIRTIO_BLK_S_OK

                    1 // 1(status)
                }
                VirtioBlkReqType::Flush => {
                    let status_desc = second_desc;

                    if status_desc.is_next() || !status_desc.is_write_only() {
                        unimplemented!();
                    }

                    self.sender
                        .send_to_host(DeviceMessage::Blk(self.bytes.clone()))
                        //[todo] ポインタだけ渡すようにする。
                        .unwrap();

                    let status_data_ptr =
                        memory.raw_mut_ptr(status_desc.addr as usize, status_desc.len as usize);
                    status_data_ptr[0] = 0; // VIRTIO_BLK_S_OK

                    1
                }
            };

            device.elems[ring_idx].len = len;
            device.elems[ring_idx].id = request_idx as u32;
            device.idx = device.idx.wrapping_add(1);
        }

        self.last_idxes[queue_idx as usize] = now_driver_idx;

        true
    }

    fn bytes_size(&self) -> u64 {
        self.bytes.len() as u64
    }
}

impl From<u32> for VirtioBlkReqType {
    fn from(value: u32) -> Self {
        match value {
            0 => VirtioBlkReqType::In,
            1 => VirtioBlkReqType::Out,
            4 => VirtioBlkReqType::Flush,
            _ => panic!(
                "[ERROR] VirtioBlkReqType 0x{:x} is not implemented. ",
                value
            ),
        }
    }
}
