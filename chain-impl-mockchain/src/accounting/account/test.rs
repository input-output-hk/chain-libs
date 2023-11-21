#[warn(unused_imports)]
use super::{AccountState, DelegationType, LastRewards};
use imhamt::Hamt;
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for AccountState<()> {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        AccountState {
            delegation: DelegationType::Full(Arbitrary::arbitrary(gen)),
            value: Arbitrary::arbitrary(gen),
            tokens: Hamt::new(),
            last_rewards: LastRewards::default(),
            #[cfg(feature = "evm")]
            evm_state: chain_evm::state::AccountState::default(),
            extra: (),
        }
    }
}
