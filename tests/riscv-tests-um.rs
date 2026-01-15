use tiny_rv32ima_sim::cpu::Cpu;

use crate::common::{TEST_ELVES_DIR, run_elf_tests, run_tests};

mod common;

#[test]
fn test_um_flats() {
    let mut cpu = Cpu::new();

    let rv32um_p_dir = format!("{}/{}", TEST_ELVES_DIR, "rv32um");
    run_elf_tests(&mut cpu, rv32um_p_dir, 0x80000000 | 0x1000, vec![]);
}
