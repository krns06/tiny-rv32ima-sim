use std::{error::Error, sync::mpsc::Receiver};

use minifb::{Key, Window, WindowOptions};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const WIDTH: usize = 800;
const HEIHGT: usize = 600;

const BUFFER_SIZE: usize = WIDTH * HEIHGT;

pub struct Gpu {
    buffer: Box<[u32; BUFFER_SIZE]>,
    resource_id: u32,
    gpu_rx: Receiver<GpuMessage>,
}

pub enum GpuOperation {
    Copy,
    Disable,
    Flush,
}

#[derive(Default)]
pub struct GpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct GpuMessage {
    pub operation: GpuOperation,
    pub resource_id: u32,
    pub rect: GpuRect,
    pub buffer: Vec<u32>,
}

impl Gpu {
    pub fn new(gpu_rx: Receiver<GpuMessage>) -> Self {
        Gpu {
            buffer: Box::new([0; BUFFER_SIZE]),
            resource_id: 0,
            gpu_rx,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut window = Window::new("Test", WIDTH, HEIHGT, WindowOptions::default())?;

        window.set_target_fps(60);

        while window.is_open() && !window.is_key_down(Key::Escape) {
            if let Ok(message) = self.gpu_rx.try_recv() {
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

impl GpuRect {
    fn start(&self) -> usize {
        (self.x + self.y * self.width) as usize
    }

    fn end(&self) -> usize {
        self.start() + (self.width * self.height) as usize
    }
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
