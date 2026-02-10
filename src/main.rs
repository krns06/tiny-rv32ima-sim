use std::{
    fs::File,
    io::{BufReader, Read},
};

use tiny_rv32ima_sim::simulator::Simulator;

const FW_SIZE: usize = 1024 * 1024;
const DTB_SIZE: usize = 64 * 1024;
const KERNEL_SIZE: usize = 36 * 1024 * 1024;

fn read_file(filename: &str, size: usize) -> Vec<u8> {
    let file = File::open(filename).unwrap();
    let mut reader = BufReader::new(file);

    let mut buf = vec![0; size];
    reader.read(&mut buf).unwrap();

    buf
}

fn main() {
    let mut simulator = Simulator::new().setup_native_devices();

    let buf = read_file("firmware/fw_jump.bin", FW_SIZE);
    simulator.load_flat(&buf, 0x80000000);

    let buf = read_file("platform.dtb", DTB_SIZE);
    simulator.load_flat(&buf, 0x80100000);

    let buf = read_file("Image5", KERNEL_SIZE);
    simulator.load_flat(&buf, 0x80400000);

    simulator.set_entry_point(0x80000000).run();
}
