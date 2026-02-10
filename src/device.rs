use std::{
    error::Error,
    sync::mpsc::{Receiver, Sender},
};

use crate::device::gpu::GpuMessage;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub mod gpu;
pub mod net;
pub mod shell;

pub type UartGustReciever = Receiver<char>;
pub type UartHostSender = Sender<char>;

pub type NetGuestReceiver = Receiver<Vec<u8>>;
pub type NetGuestSender = Sender<Vec<u8>>;

pub type NetHostReceiver = Receiver<Vec<u8>>;
pub type NetHostSender = Sender<Vec<u8>>;

pub type GpuGuestSender = Sender<GpuMessage>;
pub type GpuHostReciever = Receiver<GpuMessage>;

pub trait HostDevice: Send {
    fn run(self: Box<Self>);
}

#[derive(Default)]
pub struct DeviceManager {
    devices: Vec<Box<dyn HostDevice>>,
}

impl DeviceManager {
    pub fn devices(self) -> Vec<Box<dyn HostDevice>> {
        self.devices
    }

    pub fn add_device(&mut self, device: Box<dyn HostDevice>) -> &mut Self {
        self.devices.push(device);

        self
    }
}
