use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::Path,
};

use tiny_rv32ima_sim::cpu::Cpu;

const TEST_DIR: &str = "tests/isa/flats";

fn run_tests<P: AsRef<Path>>(cpu: &mut Cpu, file_path: P, riscv_tests_exit_memory_address: u32) {
    let file = File::open(file_path).unwrap();
    let mut reader = BufReader::new(file);

    let mut buf = vec![0; 1024 * 1024];
    reader.read(&mut buf).unwrap();

    cpu.load_flat_program(&buf);
    assert!(cpu.debug_run(riscv_tests_exit_memory_address));
}

#[test]
fn test_ui_flats() {
    let mut cpu = Cpu::new();

    let rv32ui_p_dir = format!("{}/{}", TEST_DIR, "rv32ui-p");

    let dir = fs::read_dir(rv32ui_p_dir).unwrap();
    for file in dir.into_iter() {
        let file_path = file.unwrap().path();

        if file_path.extension().unwrap() == "bin" {
            let filename = file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            println!("try: {:?}", filename);

            if filename == "rv32ui-p-ld_st.bin" {
                run_tests(&mut cpu, file_path, 0x2000);
            } else {
                run_tests(&mut cpu, file_path, 0x1000);
            }
            println!("done: {:?}", filename);
        }
    }
}
