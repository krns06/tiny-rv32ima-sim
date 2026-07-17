#[cfg(not(target_arch = "wasm32"))]
pub mod shell;

#[cfg(not(target_arch = "wasm32"))]
pub trait HostDevice: Send {
    fn run(self: Box<Self>);
}

#[cfg(target_arch = "wasm32")]
pub trait HostDevice: std::fmt::Debug {}

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
