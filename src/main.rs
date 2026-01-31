use std::{
    fs::File,
    io::{BufReader, Read},
};

use tiny_rv32ima_sim::cpu::Cpu;

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
    let mut cpu = Cpu::default();

    let buf = read_file("firmware/fw_jump.bin", FW_SIZE);

    cpu.load_flat_program::<FW_SIZE>(buf.as_slice().try_into().unwrap());

    let buf = read_file("platform.dtb", DTB_SIZE);
    cpu.load_flat_binary::<DTB_SIZE>(buf.as_slice().try_into().unwrap(), 0x80100000);

    let buf = read_file("Image4", KERNEL_SIZE);
    cpu.load_flat_binary::<KERNEL_SIZE>(buf.as_slice().try_into().unwrap(), 0x80400000);

    cpu.run();
}
