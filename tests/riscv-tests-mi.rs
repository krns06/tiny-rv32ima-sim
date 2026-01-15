use tiny_rv32ima_sim::cpu::Cpu;

use crate::common::{TEST_ELVES_DIR, run_elf_test};

mod common;

#[test]
fn test_ua_flats() {
    let mut cpu = Cpu::new();

    let rv32mi_p_dir = format!("{}/{}", TEST_ELVES_DIR, "rv32mi");

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
        run_elf_test(
            &mut cpu,
            format!("{}/{}", rv32mi_p_dir, test),
            0x80000000 | 0x1000,
        );
        println!("PASS: {}", test);
    }
}
