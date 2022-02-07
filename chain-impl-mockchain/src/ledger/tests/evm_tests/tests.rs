use crate::ledger::tests::evm_test_suite::run_evm_test;

// TODO fix this test
#[test]
#[ignore]
fn vm_block_info_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/blockInfo.json");
}

#[test]
fn vm_call_data_copy_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/calldatacopy.json",
    );
}

#[test]
fn vm_call_data_load_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/calldataload.json",
    );
}

#[test]
fn vm_dup_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/dup.json");
}

// TODO: fix this test
#[test]
#[ignore]
fn vm_env_info_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/envInfo.json");
}

#[test]
fn vm_push_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/push.json");
}

#[test]
fn vm_random_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/random.json");
}

#[test]
fn vm_sha3_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/sha3.json");
}

#[test]
fn vm_suicide_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/suicide.json");
}

#[test]
fn vm_swap_test() {
    run_evm_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmTests/swap.json");
}
