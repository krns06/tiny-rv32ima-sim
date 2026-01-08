use tiny_rv32ima_sim::cpu::Cpu;

use crate::common::{TEST_DIR, run_tests};

mod common;

#[test]
fn test_um_flats() {
    let mut cpu = Cpu::new();

    let rv32um_p_dir = format!("{}/{}", TEST_DIR, "rv32um-p");
    run_tests(&mut cpu, rv32um_p_dir, 0x1000, vec![]);
}
