use crate::ledger::tests::evm_test_suite::run_evm_test;

// TODO fix this test
#[test]
#[ignore]
fn vm_loop_exp_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmPerformance/loopExp.json",
    );
}

#[test]
#[ignore]
fn vm_loop_mul_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmPerformance/loopMul.json",
    );
}

#[test]
fn vm_performance_tester_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmPerformance/performanceTester.json",
    );
}
