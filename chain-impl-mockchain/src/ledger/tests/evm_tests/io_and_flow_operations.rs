use crate::ledger::tests::evm_test_suite::run_evm_test;

#[test]
fn vm_codecopy_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/codecopy.json",
    );
}

#[test]
fn vm_gas_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/gas.json",
    );
}

#[test]
fn vm_jump_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/jump.json",
    );
}

#[test]
fn vm_jumpi_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/jumpi.json",
    );
}

// TODO fix this test
#[test]
#[ignore]
fn vm_jump_to_push_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/jumpToPush.json",
    );
}

// TODO fix this test
#[test]
#[ignore]
fn vm_loop_stack_limit_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/loop_stacklimit.json",
    );
}

#[test]
fn vm_loops_condionals_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/loopsConditionals.json",
    );
}

#[test]
fn vm_mload_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/mload.json",
    );
}

#[test]
fn vm_msize_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/msize.json",
    );
}

#[test]
fn vm_mstore_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/mstore.json",
    );
}

#[test]
fn vm_mstore8_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/mstore8.json",
    );
}

#[test]
fn vm_pc_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/pc.json",
    );
}

#[test]
fn vm_pop_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/pop.json",
    );
}

#[test]
fn vm_return_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/return.json",
    );
}

#[test]
fn vm_sstore_sload_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmIOandFlowOperations/sstore_sload.json",
    );
}
