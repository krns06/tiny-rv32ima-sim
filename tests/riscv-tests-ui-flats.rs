use tiny_rv32ima_sim::cpu::Cpu;

use crate::common::{RiscvTest, TEST_DIR, run_tests};

mod common;

#[test]
fn test_ui_flats() {
    let mut cpu = Cpu::new();

    let rv32ui_p_dir = format!("{}/{}", TEST_DIR, "rv32ui-p");
    run_tests(
        &mut cpu,
        rv32ui_p_dir,
        0x1000,
        vec![RiscvTest {
            filename: "rv32ui-p-ld_st.bin",
            exit_address: 0x2000,
        }],
    );
}
