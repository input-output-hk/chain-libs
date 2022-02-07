use crate::ledger::tests::evm_test_suite::run_evm_test;

#[test]
fn vm_add_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/add.json",
    );
}

#[test]
fn vm_add_mod_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/addmod.json",
    );
}

#[test]
fn vm_arith_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/arith.json",
    );
}

#[test]
fn vm_div_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/div.json",
    );
}

#[test]
fn vm_div_by_zero_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/divByZero.json",
    );
}

#[test]
fn vm_exp_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/exp.json",
    );
}

#[test]
fn vm_exp_power2_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/expPower2.json",
    );
}

#[test]
fn vm_exp_power256_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/expPower256.json",
    );
}

#[test]
fn vm_exp_power256_of_256_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/expPower256Of256.json",
    );
}

#[test]
fn vm_fib_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/fib.json",
    );
}

#[test]
fn vm_mod_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/mod.json",
    );
}

#[test]
fn vm_mul_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/mul.json",
    );
}

#[test]
fn vm_mulmod_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/mulmod.json",
    );
}

#[test]
fn vm_sdiv_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/sdiv.json",
    );
}

#[test]
fn vm_signextend_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/signextend.json",
    );
}

#[test]
fn vm_smod_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/smod.json",
    );
}

#[test]
fn vm_sub_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/sub.json",
    );
}

#[test]
fn vm_two_ops_test() {
    run_evm_test(
        "../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/twoOps.json",
    );
}
