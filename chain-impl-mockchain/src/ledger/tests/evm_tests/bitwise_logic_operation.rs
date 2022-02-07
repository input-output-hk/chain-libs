use crate::ledger::tests::evm_test_suite::run_evm_test;

#[test]
fn vm_and_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/and.json",
    );
}

#[test]
fn vm_byte_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/byte.json",
    );
}

#[test]
fn vm_eq_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/eq.json",
    );
}

#[test]
fn vm_gt_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/gt.json",
    );
}

#[test]
fn vm_iszero_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/iszero.json",
    );
}

#[test]
fn vm_lt_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/lt.json",
    );
}

#[test]
fn vm_not_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/not.json",
    );
}

#[test]
fn vm_or_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/or.json",
    );
}

#[test]
fn vm_sgt_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/sgt.json",
    );
}

#[test]
fn vm_slt_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/slt.json",
    );
}

#[test]
fn vm_xor_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmBitwiseLogicOperation/xor.json",
    );
}
