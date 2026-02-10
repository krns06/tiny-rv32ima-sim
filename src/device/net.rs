use std::{
    error::Error,
    fs::OpenOptions,
    io::{Read, Write},
    os::fd::AsRawFd,
    thread,
    time::Duration,
};

use nix::libc::{self, TUNSETIFF, ioctl};

use crate::device::{HostDevice, NetHostReceiver, NetHostSender};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct HostNet {
    net_rx: NetHostReceiver,
    net_tx: NetHostSender,
}

#[derive(Default)]
struct Ifreq {
    name: [u8; 16],
    flags: i32,
}

impl HostDevice for HostNet {
    fn run(self: Box<Self>) {
        HostNet::run(*self, "tap0").unwrap();
    }
}

impl HostNet {
    pub fn new(net_rx: NetHostReceiver, net_tx: NetHostSender) -> Self {
        Self { net_rx, net_tx }
    }

    pub fn run(self, if_name: &str) -> Result<()> {
        if if_name.len() >= 16 {
            panic!("[ERROR]: if_name is invalid.");
        }

        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;

        let mut ifreq = Ifreq::default();

        ifreq.name[..if_name.len()].copy_from_slice(if_name.as_bytes());
        ifreq.flags = libc::IFF_TAP | libc::IFF_NO_PI;

        unsafe {
            ioctl(fd.as_raw_fd(), TUNSETIFF, &ifreq as *const _);
        }

        let mut fd_for_write = fd.try_clone()?;
        let mut fd_for_read = fd;

        let net_rx = self.net_rx;
        let net_tx = self.net_tx;

        thread::spawn(move || {
            loop {
                if let Ok(v) = net_rx.try_recv() {
                    if let Err(e) = fd_for_write.write(&v) {
                        eprintln!("[WARNING]: {} from run_shell.", e);
                    }
                }
            }
        });

        thread::spawn(move || {
            let mut buf = [0; 1600];

            loop {
                if let Ok(n) = fd_for_read.read(&mut buf) {
                    net_tx.send(buf[..n].to_vec()).unwrap();
                }
            }
        });

        loop {
            thread::sleep(Duration::from_micros(10));
        }
    }
}
