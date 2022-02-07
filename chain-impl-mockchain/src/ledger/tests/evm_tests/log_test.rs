use crate::ledger::tests::evm_test_suite::run_evm_test;

#[test]
fn vm_log0_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmLogTest/log0.json");
}

#[test]
fn vm_log1_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmLogTest/log1.json");
}

#[test]
fn vm_log2_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmLogTest/log2.json");
}

#[test]
fn vm_log3_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmLogTest/log3.json");
}

#[test]
fn vm_log4_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmLogTest/log4.json");
}
