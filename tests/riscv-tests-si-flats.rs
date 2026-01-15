use tiny_rv32ima_sim::cpu::Cpu;

use crate::common::{TEST_ELVES_DIR, run_elf_test, run_test};

mod common;

#[test]
fn test_si_flats() {
    let mut cpu = Cpu::new();

    let rv32si_p_dir = format!("{}/{}", TEST_ELVES_DIR, "rv32si-p");

    let required_tests = ["rv32si-p-csr", "rv32si-p-dirty"];

    for test in required_tests {
        println!("TRY: {}", test);
        run_elf_test(&mut cpu, format!("{}/{}", rv32si_p_dir, test), 0x80001000);
        println!("PASS: {}", test);
    }
}
