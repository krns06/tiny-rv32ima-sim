use tiny_rv32ima_sim::simulator::Simulator;

use crate::common::run_flat_isa_tests;

mod common;

#[test]
fn test_ua_flats() {
    let mut simulator = Simulator::new();

    run_flat_isa_tests(&mut simulator, "rv32ua", 0x80000000 | 0x1000, vec![]);
}
