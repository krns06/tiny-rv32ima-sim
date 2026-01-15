use tiny_rv32ima_sim::cpu::Cpu;

use crate::common::{RiscvTest, TEST_DIR, TEST_ELVES_DIR, run_elf_tests};

mod common;

#[test]
fn test_ui_elves() {
    let mut cpu = Cpu::new();

    let rv32ui_p_dir = format!("{}/{}", TEST_ELVES_DIR, "rv32ui");
    run_elf_tests(
        &mut cpu,
        rv32ui_p_dir,
        0x80000000 | 0x1000,
        vec![RiscvTest {
            filename: "rv32ui-p-ld_st",
            exit_address: 0x80000000 | 0x2000,
        }],
    );
}
