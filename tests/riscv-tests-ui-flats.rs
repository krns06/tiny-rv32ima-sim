use std::{
    fs::File,
    io::{BufReader, Read},
};

use tiny_rv32ima_sim::cpu::Cpu;

const TEST_DIR: &str = "tests/isa/flats";

#[test]
fn test_ui_flats() {
    let first = format!("{}/{}", TEST_DIR, "rv32ui-p-add.bin");

    let file = File::open(&first).unwrap();
    let mut reader = BufReader::new(file);

    let mut buf = vec![0; 1024 * 1024];

    reader.read(&mut buf).unwrap();

    let mut cpu = Cpu::new();

    cpu.load_flat_program(&buf);

    let res = cpu.debug_run(0x1000);

    println!("{}", res);
}
