use std::{
    fs::File,
    io::{BufReader, Read},
};

use tiny_rv32ima_sim::cpu::Cpu;

const FW_SIZE: usize = 1024 * 1024;
const DTB_SIZE: usize = 64 * 1024;

fn read_file<const SIZE: usize>(filename: &str) -> [u8; SIZE] {
    let file = File::open(filename).unwrap();
    let mut reader = BufReader::new(file);

    let mut buf = [0; SIZE];
    reader.read(&mut buf).unwrap();

    buf
}

fn main() {
    let mut cpu = Cpu::default();

    let buf: [u8; FW_SIZE] = read_file("firmware/fw_jump.bin");
    cpu.load_flat_program(&buf, 0x80000000);

    let buf: [u8; DTB_SIZE] = read_file("platform.dtb");
    cpu.load_flat_binary(&buf, 0x80100000);

    cpu.run();
}
