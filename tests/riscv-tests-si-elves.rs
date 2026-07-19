use tiny_rv32ima_sim::simulator::Simulator;

use crate::common::{TEST_DIR, run_elf_test};

mod common;

#[test]
fn test_si_flats() {
    let mut simulator = Simulator::new();

    let required_tests = [
        "rv32si-p-csr",
        "rv32si-p-dirty",
        "rv32si-p-ma_fetch",
        "rv32si-p-scall",
        "rv32si-p-wfi",
    ];

    for test in required_tests {
        println!("TRY: {}", test);
        if run_elf_test(&mut simulator, format!("{}/{}", TEST_DIR, test), 0x80001000) {
            println!("PASS: {}", test);
        }
    }
}
