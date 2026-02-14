use std::error::Error;

use minifb::{Key, Window, WindowOptions};

use crate::{
    device::DeviceMessage,
    host_device::{GpuOperation, HostDevice},
    native::NativeHostReciever,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const WIDTH: usize = 800;
const HEIHGT: usize = 600;

const BUFFER_SIZE: usize = WIDTH * HEIHGT;

#[derive(Debug)]
pub struct HostGpu {
    buffer: Box<[u32; BUFFER_SIZE]>,
    resource_id: u32,
    gpu_rx: NativeHostReciever,
}

impl HostDevice for HostGpu {
    fn run(self: Box<Self>) {
        let mut gpu = self;
        HostGpu::run(&mut *gpu).unwrap();
    }
}

impl HostGpu {
    pub fn new(gpu_rx: NativeHostReciever) -> Self {
        HostGpu {
            buffer: Box::new([0; BUFFER_SIZE]),
            resource_id: 0,
            gpu_rx,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut window = Window::new("Test", WIDTH, HEIHGT, WindowOptions::default())?;

        window.set_target_fps(60);

        while window.is_open() && !window.is_key_down(Key::Escape) {
            if let Ok(DeviceMessage::Gpu(message)) = self.gpu_rx.try_recv() {
                match message.operation {
                    GpuOperation::Copy => {
                        let start = message.rect.start();
                        let end = message.rect.end();

                        self.buffer[start..end].copy_from_slice(&message.buffer);
                        self.resource_id = message.resource_id;
                    }
                    GpuOperation::Flush => {
                        if self.resource_id != message.resource_id {
                            eprintln!(
                                "[WARNING] GpuMessage resource_id({}) is invalid.",
                                message.resource_id
                            );
                        }
                    }
                    GpuOperation::Disable => {
                        self.resource_id = 0;
                        eprintln!("[WARNING] GpuMessage Disable is not implemented.");
                    }
                }
            }

            window.update_with_buffer(&self.buffer.as_slice(), WIDTH, HEIHGT)?;
        }

        Ok(())
    }
}
