use std::fmt::Debug;

#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;
#[cfg(not(target_arch = "wasm32"))]
pub mod net;
#[cfg(not(target_arch = "wasm32"))]
pub mod shell;

#[cfg(not(target_arch = "wasm32"))]
pub trait HostDevice: Send {
    fn run(self: Box<Self>);
}

#[cfg(target_arch = "wasm32")]
pub trait HostDevice: Debug {}

#[derive(Default)]
pub struct HostDeviceManager {
    devices: Vec<Box<dyn HostDevice>>,
}

impl HostDeviceManager {
    pub fn devices(self) -> Vec<Box<dyn HostDevice>> {
        self.devices
    }

    pub fn add_device(&mut self, device: Box<dyn HostDevice>) -> &mut Self {
        self.devices.push(device);

        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GpuOperation {
    Copy,
    Disable,
    Flush,
}

#[derive(Debug, Default)]
pub struct GpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl GpuRect {
    fn start(&self) -> usize {
        (self.x + self.y * self.width) as usize
    }

    fn end(&self) -> usize {
        self.start() + (self.width * self.height) as usize
    }
}

#[derive(Debug)]
pub struct GpuMessage {
    pub operation: GpuOperation,
    pub resource_id: u32,
    pub rect: GpuRect,
    pub buffer: Vec<u32>,
}

impl GpuMessage {
    pub fn new(operation: GpuOperation, resource_id: u32) -> Self {
        Self {
            operation,
            resource_id,
            rect: GpuRect::default(),
            buffer: Vec::new(),
        }
    }
}
