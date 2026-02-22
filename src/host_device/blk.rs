use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    thread,
};

use crate::{DeviceMessage, host_device::HostDevice, native::NativeHostReciever};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct HostBlk {
    blk_rx: NativeHostReciever,
    file: File,
}

impl HostDevice for HostBlk {
    fn run(self: Box<Self>) {
        HostBlk::run(*self);
    }
}

impl HostBlk {
    pub fn new(filepath: &str, blk_rx: NativeHostReciever) -> Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(filepath)?;

        Ok(Self { file, blk_rx })
    }

    pub fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();

        self.file.read_to_end(&mut bytes)?;

        Ok(bytes)
    }

    pub fn run(self) {
        let mut file = self.file;

        thread::spawn(move || {
            loop {
                if let Ok(DeviceMessage::Blk(v)) = self.blk_rx.recv() {
                    file.seek(SeekFrom::Start(0)).unwrap();
                    file.write(&v).unwrap();
                }
            }
        });
    }
}
