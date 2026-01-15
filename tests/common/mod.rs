pub(crate) use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::Path,
};

use tiny_rv32ima_sim::cpu::Cpu;

pub const TEST_DIR: &str = "tests/isa/flats";
pub const TEST_ELVES_DIR: &str = "tests/isa/elves";

pub struct RiscvTest<'a> {
    pub filename: &'a str,
    pub exit_address: u32,
}

pub fn run_test<P: AsRef<Path>>(cpu: &mut Cpu, file_path: P, exit_address: u32) {
    let file = File::open(file_path).unwrap();
    let mut reader = BufReader::new(file);

    let mut buf = vec![0; 1024 * 1024];
    reader.read(&mut buf).unwrap();

    cpu.load_flat_program(&buf);
    assert!(cpu.debug_run(exit_address));
}

pub fn run_elf_test<P: AsRef<Path>>(cpu: &mut Cpu, file_path: P, exit_address: u32) {
    let file = File::open(file_path).unwrap();
    let mut reader = BufReader::new(file);

    let mut buf = vec![0; 1024 * 1024];
    reader.read(&mut buf).unwrap();

    cpu.set_memory_base_address(0x80000000);
    cpu.load_elf_program(&buf);
    assert!(cpu.debug_run(exit_address));
}

pub fn run_tests<P: AsRef<Path>>(
    cpu: &mut Cpu,
    dir_path: P,
    default_exit_address: u32,
    excludes: Vec<RiscvTest>,
) {
    let dir = fs::read_dir(dir_path).unwrap();
    for file in dir.into_iter() {
        let file_path = file.unwrap().path();

        if file_path.extension().unwrap() == "bin" {
            let filename = file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let mut exit_address = default_exit_address;
            for exclude in &excludes {
                if filename == exclude.filename {
                    exit_address = exclude.exit_address;
                }
            }

            println!("TRY: {}", filename);
            run_test(cpu, file_path, exit_address);
            println!("PASS: {}", filename);
        }
    }
}
