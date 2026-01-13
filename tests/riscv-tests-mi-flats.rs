use tiny_rv32ima_sim::cpu::Cpu;

use crate::common::{TEST_DIR, run_test};

mod common;

#[test]
fn test_ua_flats() {
    let mut cpu = Cpu::new();

    let rv32mi_p_dir = format!("{}/{}", TEST_DIR, "rv32mi-p");

    let required_tests = ["rv32mi-p-csr.bin", "rv32mi-p-illegal.bin"];

    for test in required_tests {
        println!("TRY: {}", test);
        run_test(&mut cpu, format!("{}/{}", rv32mi_p_dir, test), 0x1000);
        println!("PASS: {}", test);
    }
}
