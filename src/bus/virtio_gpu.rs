use std::{collections::HashMap, mem::transmute, sync::mpsc::Sender};

use crate::{
    bus::{
        ExternalDevice, ExternalDeviceResponse,
        virtio_mmio::{
            VIRTIO_REG_CONFIG, VIRTIO_REG_NOTIFY, VIRTIO_REG_STATUS, VirtQueueDesc, VirtioMmio,
            VirtioType, read_panic,
        },
    },
    device::gpu::{GpuMessage, GpuOperation, GpuRect},
    memory::Memory,
};

const VIRTIO_GPU_HEADER_SIZE: usize = size_of::<VirtioGpuCtrlHeader>();
const VIRTIO_GPU_RESP_DISPLAY_INFO_SIZE: usize = size_of::<VirtioGpuRespDisplayInfo>();
const VIRTIO_GPU_RESOUCE_ATTACH_BACKING_SIZE: usize = size_of::<VirtioGpuResouceAttachBacking>();
const VRITIO_GPU_MEM_ENTRY_SIZE: usize = size_of::<VirtioGpuMemEntry>();

const VIRTIO_GPU_CONTROL_IDX: u32 = 0;
const VIRTIO_GPU_CURSOR_IDX: u32 = 1;

const FEATURES: [u32; 4] = [0, 1, 0, 0];
const MAX_QUEUE_SIZE: usize = 256;
const SHM_LENS: [u64; 2] = [0x20_0000, 0x20_0000]; // 使われないが定義しないとfailedになる。
const SHM_BASES: [u64; 2] = [0x1001_0000, 0x1003_0000]; // 使われないが定義しないとfailedになる。
const MAX_SCANOUTS: u32 = 1;
const MAX_CAPSETS: u32 = 0;

const SUPPORTED_RECT: VirtioGpuRect = VirtioGpuRect {
    x: 0,
    y: 0,
    width: 800,
    height: 600,
};
const SUPPORTED_SLIDE_SIZE: u32 = 4; // BGRX以外サポートしていないので4byteごと

#[derive(Debug)]
pub struct VirtioGpu {
    virtio: VirtioMmio,

    last_idxes: [u16; 2],
    resources: HashMap<u32, GpuResouce>,
    scanouts: [GpuScanout; MAX_SCANOUTS as usize],

    output_tx: Sender<GpuMessage>,
}

#[derive(Debug)]
pub struct GpuResouce {
    format: u32,
    width: u32,
    height: u32,

    entries: Vec<VirtioGpuMemEntry>,
}

#[derive(Debug, Default)]
pub struct GpuScanout {
    r: VirtioGpuRect,
    resource_id: u32,
}

#[derive(Debug)]
enum VirtioGpuCtrlType {
    CmdGetDisplayInfo = 0x0100,
    CmdResourceCreate2D,
    CmdSetScanout = 0x103,
    CmdResourceFlush,
    CmdTransferToHost2D,
    CmdResourceAttachBacking = 0x106,
    RespOkNodata = 0x1100,
    RespOkDisplayInfo,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtioGpuCtrlHeader {
    ctrl_type: u32,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    ring_idx: u8,
    _padding: [u8; 3],
}

#[derive(Debug, Default, PartialEq, PartialOrd, Clone, Copy)]
#[repr(C)]
pub struct VirtioGpuRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct VirtioGpuDisplayOne {
    r: VirtioGpuRect,
    enabled: u32,
    flags: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtioGpuRespDisplayInfo {
    header: VirtioGpuCtrlHeader,
    pmodes: [VirtioGpuDisplayOne; 16],
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtioGpuResourceCreate2D {
    header: VirtioGpuCtrlHeader,
    resource_id: u32,
    format: u32,
    width: u32,
    height: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtioGpuResouceAttachBacking {
    header: VirtioGpuCtrlHeader,
    resouce_id: u32,
    nr_entries: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioGpuMemEntry {
    addr: u64,
    length: u32,
    _padding: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtioGpuSetScanout {
    header: VirtioGpuCtrlHeader,
    r: VirtioGpuRect,
    scanout_id: u32,
    resource_id: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtioGpuTransferToHost2d {
    header: VirtioGpuCtrlHeader,
    r: VirtioGpuRect,
    offset: u64,
    resource_id: u32,
    _padding: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct VitioGpuResourceFlush {
    header: VirtioGpuCtrlHeader,
    r: VirtioGpuRect,
    resource_id: u32,
    _padding: u32,
}

impl ExternalDevice for VirtioGpu {
    fn read(
        &mut self,
        offset: u32,
        size: u32,
        _: &mut crate::memory::Memory,
    ) -> super::ExternalDeviceResult<u32> {
        if size != 4 {
            unimplemented!();
        }

        let value = match offset {
            0xb0 => SHM_LENS[self.virtio.shm_sel()] as u32, // SHM Len Low
            0xb4 => (SHM_LENS[self.virtio.shm_sel()] >> 32) as u32, // SHM Base High
            0xb8 => SHM_BASES[self.virtio.shm_sel()] as u32, // SHM Base Low
            0xbc => (SHM_BASES[self.virtio.shm_sel()] >> 32) as u32, // SHM Base High
            _ => {
                if offset < VIRTIO_REG_CONFIG {
                    return self.virtio.read(offset, size);
                } else {
                    let offset = offset - VIRTIO_REG_CONFIG;

                    match offset {
                        8 => MAX_SCANOUTS,
                        0xc => MAX_CAPSETS,
                        _ => read_panic(offset),
                    }
                }
            }
        };

        Ok(ExternalDeviceResponse {
            value,
            is_interrupting: false,
        })
    }

    fn write(
        &mut self,
        offset: u32,
        size: u32,
        value: u32,
        memory: &mut crate::memory::Memory,
    ) -> super::ExternalDeviceResult<()> {
        if size != 4 {
            unimplemented!();
        }

        match offset {
            VIRTIO_REG_NOTIFY => {
                let is_interrupting = self.handle_notify(value, memory);

                return Ok(ExternalDeviceResponse {
                    value: (),
                    is_interrupting,
                });
            }
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

        Ok(super::ExternalDeviceResponse {
            value: (),
            is_interrupting: false,
        })
    }

    fn irq(&self) -> crate::IRQ {
        crate::IRQ::VirtioGpu
    }
}

fn write_ok_nodata_response(dst_desc: &VirtQueueDesc, memory: &mut Memory) -> u32 {
    let response = VirtioGpuCtrlHeader::new(VirtioGpuCtrlType::RespOkNodata);

    let response_data: &[u8; VIRTIO_GPU_HEADER_SIZE] = unsafe { transmute(&response as *const _) };

    if VIRTIO_GPU_HEADER_SIZE > dst_desc.len as usize {
        unimplemented!()
    }

    let ptr = memory.raw_mut_ptr(dst_desc.addr as usize, VIRTIO_GPU_HEADER_SIZE);

    ptr.copy_from_slice(response_data);

    VIRTIO_GPU_HEADER_SIZE as u32
}

// XRGBに変換する関数
// format = 2(RGBX)のみサポート
fn format_array(format: u32, array: &[u8]) -> Vec<u32> {
    if format != 2 {
        unimplemented!()
    }

    array
        .chunks_exact(4)
        .map(|chunk| {
            let b = chunk[0] as u32;
            let g = chunk[1] as u32;
            let r = chunk[2] as u32;

            (r << 16) | (g << 8) | b
        })
        .collect()
}

impl VirtioGpu {
    pub fn new(output_tx: Sender<GpuMessage>) -> Self {
        let virtio = VirtioMmio::new(VirtioType::Gpu, FEATURES, 2, MAX_QUEUE_SIZE as u32);

        Self {
            virtio,
            last_idxes: [0; 2],
            resources: HashMap::new(),
            output_tx,
            scanouts: [GpuScanout::default()],
        }
    }

    fn reset(&mut self) {
        let output_tx = self.output_tx.clone();

        *self = Self::new(output_tx);
    }

    fn handle_notify(&mut self, queue_idx: u32, memory: &mut Memory) -> bool {
        if queue_idx != VIRTIO_GPU_CONTROL_IDX {
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
            let command_idx = driver.ring[ring_idx];

            let command_desc = self.virtio.desc(command_idx, desc_base, memory);

            if !command_desc.is_next() {
                unimplemented!();
            }

            let second_desc = self.virtio.desc(command_desc.next, desc_base, memory);

            let command_data_ptr =
                memory.raw_ptr(command_desc.addr as usize, command_desc.len as usize);

            let command_data: &VirtioGpuCtrlHeader =
                unsafe { transmute(command_data_ptr.as_ptr()) };

            let ctrl_type = VirtioGpuCtrlType::from(command_data.ctrl_type);

            let len = match ctrl_type {
                VirtioGpuCtrlType::CmdGetDisplayInfo => {
                    if second_desc.is_next() || !second_desc.is_write_only() {
                        unimplemented!();
                    }

                    let response = VirtioGpuRespDisplayInfo::as_response();
                    let response_data: &[u8; VIRTIO_GPU_RESP_DISPLAY_INFO_SIZE] =
                        unsafe { transmute(&response as *const _) };

                    if VIRTIO_GPU_RESP_DISPLAY_INFO_SIZE > second_desc.len as usize {
                        unimplemented!()
                    }

                    let second_ptr = memory
                        .raw_mut_ptr(second_desc.addr as usize, VIRTIO_GPU_RESP_DISPLAY_INFO_SIZE);

                    second_ptr.copy_from_slice(response_data);

                    VIRTIO_GPU_RESP_DISPLAY_INFO_SIZE as u32
                }
                VirtioGpuCtrlType::CmdResourceCreate2D => {
                    if second_desc.is_next() || !second_desc.is_write_only() {
                        unimplemented!();
                    }

                    let resource_create_2d = memory.view_as::<VirtioGpuResourceCreate2D>(
                        command_desc.addr as usize,
                        command_desc.len as usize,
                    );

                    if resource_create_2d.format != 2 {
                        // BGRX以外とりあえずサポートしない
                        unimplemented!()
                    }

                    self.resources.insert(
                        resource_create_2d.resource_id,
                        GpuResouce::from(resource_create_2d),
                    );

                    write_ok_nodata_response(second_desc, memory)
                }
                VirtioGpuCtrlType::CmdResourceAttachBacking => {
                    if !second_desc.is_next() {
                        unimplemented!();
                    }

                    let third_desc = self.virtio.desc(second_desc.next, desc_base, memory);

                    if third_desc.is_next() || !third_desc.is_write_only() {
                        // 1以上の場合はサポートしない
                        unimplemented!()
                    }

                    let resource_attach_backing = memory.view_as::<VirtioGpuResouceAttachBacking>(
                        command_desc.addr as usize,
                        command_desc.len as usize,
                    );

                    let resource = self
                        .resources
                        .get_mut(&resource_attach_backing.resouce_id)
                        .unwrap();

                    for i in 0..resource_attach_backing.nr_entries {
                        let entry = memory.view_as::<VirtioGpuMemEntry>(
                            second_desc.addr as usize + i as usize * size_of::<VirtioGpuMemEntry>(),
                            second_desc.len as usize,
                        );

                        resource.entries.push(entry.clone());
                    }

                    write_ok_nodata_response(third_desc, memory)
                }
                VirtioGpuCtrlType::CmdSetScanout => {
                    if second_desc.is_next() || !second_desc.is_write_only() {
                        unimplemented!();
                    }

                    let set_scanout = memory.view_as::<VirtioGpuSetScanout>(
                        command_desc.addr as usize,
                        command_desc.len as usize,
                    );

                    let resource_id = set_scanout.resource_id;

                    if resource_id == 0 {
                        let message = GpuMessage::new(GpuOperation::Disable, resource_id);
                        self.output_tx.send(message).unwrap();
                    } else {
                        if set_scanout.scanout_id > MAX_SCANOUTS {
                            unimplemented!();
                        }

                        self.scanouts[set_scanout.scanout_id as usize] = GpuScanout {
                            r: set_scanout.r,
                            resource_id: set_scanout.resource_id,
                        };
                    }

                    write_ok_nodata_response(second_desc, memory)
                }
                VirtioGpuCtrlType::CmdTransferToHost2D => {
                    if second_desc.is_next() || !second_desc.is_write_only() {
                        unimplemented!();
                    }

                    let transfer_to_host_2d = memory.view_as::<VirtioGpuTransferToHost2d>(
                        command_desc.addr as usize,
                        command_desc.len as usize,
                    );

                    if transfer_to_host_2d.r != SUPPORTED_RECT {
                        unimplemented!();
                    }

                    let array_size = SUPPORTED_RECT.size();
                    let mut array = vec![0; array_size];

                    let resource_id = transfer_to_host_2d.resource_id;
                    let resource = self.resources.get(&resource_id).unwrap();

                    let mut copied_size: usize = 0;

                    for entry in &resource.entries {
                        let entry_len = entry.length as usize;
                        let entry_ptr = memory.raw_ptr(entry.addr as usize, entry_len);

                        let actual_len = if copied_size + entry_len > array_size {
                            array_size - copied_size
                        } else {
                            entry_len
                        };

                        array[copied_size..copied_size + actual_len]
                            .copy_from_slice(&entry_ptr[..actual_len]);
                        copied_size += actual_len;
                    }

                    let buffer = format_array(resource.format, &array);

                    let message = GpuMessage {
                        operation: GpuOperation::Copy,
                        resource_id,
                        rect: GpuRect::from(transfer_to_host_2d.r),
                        buffer,
                    };

                    self.output_tx.send(message).unwrap();

                    write_ok_nodata_response(second_desc, memory)
                }
                VirtioGpuCtrlType::CmdResourceFlush => {
                    if second_desc.is_next() || !second_desc.is_write_only() {
                        unimplemented!();
                    }

                    let resource_flush = memory.view_as::<VitioGpuResourceFlush>(
                        command_desc.addr as usize,
                        command_desc.len as usize,
                    );

                    if resource_flush.r != SUPPORTED_RECT {
                        unimplemented!();
                    }

                    let message = GpuMessage {
                        operation: GpuOperation::Flush,
                        resource_id: resource_flush.resource_id,
                        rect: GpuRect::from(resource_flush.r),
                        buffer: Vec::new(),
                    };

                    self.output_tx.send(message).unwrap();

                    write_ok_nodata_response(second_desc, memory)
                }
                _ => unimplemented!(),
            };

            device.elems[ring_idx].len = len;
            device.elems[ring_idx].id = command_idx as u32;
            device.idx = device.idx.wrapping_add(1);
        }

        self.last_idxes[queue_idx as usize] = now_driver_idx;

        true
    }
}

impl From<&VirtioGpuResourceCreate2D> for GpuResouce {
    fn from(value: &VirtioGpuResourceCreate2D) -> Self {
        Self {
            format: value.format,
            width: value.width,
            height: value.height,
            entries: Vec::new(),
        }
    }
}

impl From<u32> for VirtioGpuCtrlType {
    fn from(value: u32) -> Self {
        match value {
            0x100 => Self::CmdGetDisplayInfo,
            0x101 => Self::CmdResourceCreate2D,
            0x103 => Self::CmdSetScanout,
            0x104 => Self::CmdResourceFlush,
            0x105 => Self::CmdTransferToHost2D,
            0x106 => Self::CmdResourceAttachBacking,
            0x1100 => Self::RespOkNodata,
            0x1101 => Self::CmdGetDisplayInfo,
            _ => panic!(
                "[ERROR] VirtioGpuCtrlType 0x{:x} is not implemented. ",
                value
            ),
        }
    }
}

impl From<VirtioGpuRect> for GpuRect {
    fn from(value: VirtioGpuRect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

impl VirtioGpuRect {
    pub const fn size(&self) -> usize {
        (self.width * self.height * SUPPORTED_SLIDE_SIZE) as usize
    }
}

impl VirtioGpuCtrlHeader {
    const fn new(ctrl_type: VirtioGpuCtrlType) -> Self {
        VirtioGpuCtrlHeader {
            ctrl_type: ctrl_type as u32,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            _padding: [0; 3],
        }
    }
}

impl VirtioGpuDisplayOne {
    pub const ZERO: Self = Self {
        r: VirtioGpuRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        enabled: 0,
        flags: 0,
    };
}

impl VirtioGpuRespDisplayInfo {
    const fn as_response() -> Self {
        let mut pmodes = [VirtioGpuDisplayOne::ZERO; 16];

        pmodes[0] = VirtioGpuDisplayOne {
            r: SUPPORTED_RECT,
            enabled: 1,
            flags: 0,
        };

        Self {
            header: VirtioGpuCtrlHeader::new(VirtioGpuCtrlType::RespOkDisplayInfo),
            pmodes,
        }
    }
}
