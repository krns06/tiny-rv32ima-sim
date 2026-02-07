use std::{
    error::Error,
    fs::OpenOptions,
    io::{Read, Write},
    os::fd::AsRawFd,
    sync::mpsc::{Receiver, Sender},
    thread,
    time::Duration,
};

use nix::libc::{self, TUNSETIFF, ioctl};

#[derive(Default)]
struct Ifreq {
    name: [u8; 16],
    flags: i32,
}

pub fn run_net(
    if_name: &str,
    virtio_net_tx: Sender<Vec<u8>>,
    virtio_net_rx: Receiver<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
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

    thread::spawn(move || {
        loop {
            if let Ok(v) = virtio_net_rx.try_recv() {
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
                virtio_net_tx.send(buf[..n].to_vec()).unwrap();
            }
        }
    });

    loop {
        thread::sleep(Duration::from_micros(10));
    }
}
