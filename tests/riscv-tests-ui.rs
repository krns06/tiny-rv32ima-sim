use tiny_rv32ima_sim::simulator::Simulator;

use crate::common::{RiscvTest, run_flat_isa_tests};

mod common;

#[test]
fn test_ui_flats() {
    let mut simulator = Simulator::new();

    run_flat_isa_tests(
        &mut simulator,
        "rv32ui",
        0x80000000 | 0x1000,
        vec![RiscvTest {
            filename: "rv32ui-p-ld_st",
            exit_address: 0x80000000 | 0x2000,
        }],
    );
}
