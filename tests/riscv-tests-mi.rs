use tiny_rv32ima_sim::simulator::Simulator;

use crate::common::{TEST_DIR, run_elf_test};

mod common;

#[test]
fn test_mi_flats() {
    let mut simulator = Simulator::new();

    let required_tests = [
        "rv32mi-p-csr",
        "rv32mi-p-illegal",
        // "rv32mi-p-instret_overflow", // flatバイナリでは存在するのでなんでだろう？
        "rv32mi-p-lh-misaligned",
        "rv32mi-p-lw-misaligned",
        "rv32mi-p-ma_addr",
        "rv32mi-p-ma_fetch",
        "rv32mi-p-mcsr",
        "rv32mi-p-scall",
        "rv32mi-p-sh-misaligned",
        "rv32mi-p-shamt",
        "rv32mi-p-sw-misaligned",
        "rv32mi-p-zicntr",
    ];

    for test in required_tests {
        println!("TRY: {}", test);
        if run_elf_test(
            &mut simulator,
            format!("{}/{}", TEST_DIR, test),
            0x80000000 | 0x1000,
        ) {
            println!("PASS: {}", test);
        }
    }
}
