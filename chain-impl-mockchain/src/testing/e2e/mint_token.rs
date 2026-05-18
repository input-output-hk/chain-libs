use crate::{
    certificate::MintToken,
    ledger::Error::MintingPolicyViolation,
    testing::{
        ledger::ConfigBuilder,
        scenario::{prepare_scenario, wallet},
        TestGen,
    },
    tokens::minting_policy::{
        MintingPolicy,
        MintingPolicyViolation::{AdditionalMintingNotAllowed, UnauthorizedMinter},
    },
    value::Value,
};

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

#[test]
pub fn mint_token_not_allowed_outside_block_0() {
    let (mut ledger, controller) = prepare_scenario()
        .with_config(ConfigBuilder::new())
        .with_initials(vec![wallet(ALICE).with(1_000)])
        .build()
        .unwrap();

    let alice = controller.wallet(ALICE).unwrap();

    assert_eq!(
        controller
            .mint_token(
                &alice,
                TestGen::mint_token_for_wallet(alice.public_key().into()),
                &mut ledger,
            )
            .err()
            .unwrap(),
        MintingPolicyViolation(AdditionalMintingNotAllowed)
    );
}

#[test]
pub fn mint_token_with_authority_policy_succeeds_at_runtime() {
    let (mut ledger, controller) = prepare_scenario()
        .with_config(ConfigBuilder::new())
        .with_initials(vec![wallet(ALICE).with(1_000)])
        .build()
        .unwrap();

    let alice = controller.wallet(ALICE).unwrap();

    let mint = MintToken {
        name: TestGen::token_name(),
        policy: MintingPolicy::with_authority(alice.public_key()),
        to: alice.public_key().into(),
        value: Value(100),
    };

    controller
        .mint_token(&alice, mint, &mut ledger)
        .expect("authority-keyed mint should be accepted at runtime");
}

#[test]
pub fn mint_token_unauthorized_minter_is_rejected_at_runtime() {
    let (mut ledger, controller) = prepare_scenario()
        .with_config(ConfigBuilder::new())
        .with_initials(vec![
            wallet(ALICE).with(1_000),
            wallet(BOB).with(1_000),
        ])
        .build()
        .unwrap();

    let alice = controller.wallet(ALICE).unwrap();
    let bob = controller.wallet(BOB).unwrap();

    // Policy authorizes alice but the wrapping tx is funded by bob.
    let mint = MintToken {
        name: TestGen::token_name(),
        policy: MintingPolicy::with_authority(alice.public_key()),
        to: bob.public_key().into(),
        value: Value(100),
    };

    assert_eq!(
        controller
            .mint_token(&bob, mint, &mut ledger)
            .err()
            .unwrap(),
        MintingPolicyViolation(UnauthorizedMinter)
    );
}
