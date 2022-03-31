use super::{
    element::SingleAccountBindingSignature, AccountBindingSignature, AccountIdentifier, Input,
    NoExtra, Payload, Transaction, TxBuilder, UnspecifiedAccountIdentifier, UtxoPointer, Witness,
};
use crate::account::SpendingCounter;
#[cfg(test)]
use crate::certificate::OwnerStakeDelegation;
use crate::date::BlockDate;
use crate::key::{EitherEd25519SecretKey, SpendingSignature};
#[cfg(test)]
use crate::testing::serialization::serialization_bijection;
use chain_crypto::{testing::arbitrary_secret_key, Ed25519, SecretKey, Signature};
#[cfg(test)]
use quickcheck::TestResult;
use quickcheck::{Arbitrary, Gen};
use quickcheck_macros::quickcheck;

quickcheck! {
    fn transaction_encode_decode(transaction: Transaction<NoExtra>) -> TestResult {
        serialization_bijection(transaction)
    }
    fn stake_owner_delegation_tx_encode_decode(transaction: Transaction<OwnerStakeDelegation>) -> TestResult {
        serialization_bijection(transaction)
    }
    /*
    fn certificate_tx_encode_decode(transaction: Transaction<Address, Certificate>) -> TestResult {
        chain_core::property::testing::serialization_bijection(transaction)
    }
    */
    fn signed_transaction_encode_decode(transaction: Transaction<NoExtra>) -> TestResult {
        serialization_bijection(transaction)
    }
}

#[cfg(test)]
fn check_eq<X>(s1: &str, x1: X, s2: &str, x2: X, s: &str) -> Result<(), String>
where
    X: Eq + std::fmt::Display,
{
    if x1 == x2 {
        Ok(())
    } else {
        Err(format!(
            "{} and {} have different number of {} : {} != {}",
            s1, s2, x1, x2, s
        ))
    }
}

#[quickcheck]
pub fn check_transaction_accessor_consistent(tx: Transaction<NoExtra>) -> TestResult {
    let slice = tx.as_slice();
    let res = check_eq(
        "tx",
        tx.nb_inputs(),
        "tx-slice",
        slice.nb_inputs(),
        "inputs",
    )
    .and_then(|()| {
        check_eq(
            "tx",
            tx.nb_inputs(),
            "tx-inputs-slice",
            slice.inputs().nb_inputs(),
            "inputs",
        )
    })
    .and_then(|()| {
        check_eq(
            "tx",
            tx.nb_inputs() as usize,
            "tx-inputs-slice-iter",
            slice.inputs().iter().count(),
            "inputs",
        )
    })
    .and_then(|()| {
        check_eq(
            "tx",
            tx.nb_outputs(),
            "tx-outputs-slice",
            slice.outputs().nb_outputs(),
            "outputs",
        )
    })
    .and_then(|()| {
        check_eq(
            "tx",
            tx.nb_outputs() as usize,
            "tx-outputs-slice-iter",
            slice.outputs().iter().count(),
            "outputs",
        )
    })
    .and_then(|()| {
        check_eq(
            "tx",
            tx.nb_witnesses(),
            "tx-witness-slice",
            slice.witnesses().nb_witnesses(),
            "witnesses",
        )
    })
    .and_then(|()| {
        check_eq(
            "tx",
            tx.nb_witnesses() as usize,
            "tx-witness-slice-iter",
            slice.witnesses().iter().count(),
            "witnesses",
        )
    });
    match res {
        Ok(()) => TestResult::passed(),
        Err(e) => TestResult::error(e),
    }
}

impl Arbitrary for UtxoPointer {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        UtxoPointer {
            transaction_id: Arbitrary::arbitrary(g),
            output_index: Arbitrary::arbitrary(g),
            value: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for Input {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Input::from_utxo(Arbitrary::arbitrary(g))
    }
}

impl Arbitrary for NoExtra {
    fn arbitrary<G: Gen>(_: &mut G) -> Self {
        Self
    }
}

impl<Extra: Arbitrary + Payload> Arbitrary for Transaction<Extra>
where
    Extra::Auth: Arbitrary,
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let payload: Extra = Arbitrary::arbitrary(g);
        let payload_auth: Extra::Auth = Arbitrary::arbitrary(g);

        let num_inputs = u8::arbitrary(g) as usize;
        let num_outputs = u8::arbitrary(g) as usize;

        let inputs: Vec<_> = std::iter::repeat_with(|| Arbitrary::arbitrary(g))
            .take(num_inputs % 16)
            .collect();
        let outputs: Vec<_> = std::iter::repeat_with(|| Arbitrary::arbitrary(g))
            .take(num_outputs % 16)
            .collect();
        let witnesses: Vec<_> = std::iter::repeat_with(|| Arbitrary::arbitrary(g))
            .take(num_inputs % 16)
            .collect();

        TxBuilder::new()
            .set_payload(&payload)
            .set_expiry_date(BlockDate::first().next_epoch())
            .set_ios(&inputs, &outputs)
            .set_witnesses(&witnesses)
            .set_payload_auth(&payload_auth)
    }
}

mod pt {
    use crate::account::{AccountAlg, SpendingCounter};
    use crate::block::BlockDate;
    use crate::fragment::FragmentId;
    use crate::key::SpendingSignature;
    use crate::transaction::{
        Input, Output, TxBuilder, UtxoPointer, Witness, WitnessAccountData, WitnessUtxoData,
    };
    use crate::value::Value;

    use super::{Payload, Transaction};
    use chain_addr::Address;
    use chain_crypto::testing::public_key_strategy;
    use chain_crypto::{Ed25519, Signature};
    use proptest::arbitrary::StrategyFor;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::strategy::Map;

    impl<Extra> Arbitrary for Transaction<Extra>
    where
        Extra: Arbitrary + Payload,
        Extra::Auth: Arbitrary,
    {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            (0usize..16, 0usize..16)
                .prop_flat_map(|(num_inputs, num_outputs)| {
                    any::<(Extra, Extra::Auth)>().prop_flat_map(move |(extra, auth)| {
                        let inputs = vec(any::<Input>(), num_inputs);
                        let outputs = vec(any::<Output<Address>>(), num_outputs);
                        let witnesses = vec(any::<Witness>(), num_inputs);

                        (inputs, outputs, witnesses).prop_map(
                            move |(inputs, outputs, witnesses)| {
                                TxBuilder::new()
                                    .set_payload(&extra)
                                    .set_expiry_date(BlockDate::first().next_epoch())
                                    .set_ios(&inputs, &outputs)
                                    .set_witnesses(&witnesses)
                                    .set_payload_auth(&auth)
                            },
                        )
                    })
                })
                .boxed()
        }
    }

    impl Arbitrary for Input {
        type Parameters = ();
        type Strategy = Map<StrategyFor<UtxoPointer>, fn(UtxoPointer) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<UtxoPointer>().prop_map(Input::from_utxo)
        }
    }

    impl Arbitrary for UtxoPointer {
        type Parameters = ();
        type Strategy =
            Map<StrategyFor<(FragmentId, u8, Value)>, fn((FragmentId, u8, Value)) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<(FragmentId, u8, Value)>().prop_map(|(transaction_id, output_index, value)| {
                UtxoPointer {
                    transaction_id,
                    output_index,
                    value,
                }
            })
        }
    }

    impl Arbitrary for Witness {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            prop_oneof![
                any::<SpendingSignature<WitnessUtxoData>>().prop_map(Witness::Utxo),
                any::<(SpendingCounter, Signature<WitnessAccountData, AccountAlg>)>()
                    .prop_map(|(counter, witness)| Witness::Account(counter, witness)),
                public_key_strategy::<Ed25519>().prop_flat_map(move |key| any::<
                    Signature<WitnessUtxoData, Ed25519>,
                >()
                .prop_map(move |signature| {
                    Witness::OldUtxo(key.clone(), [0u8; 32], signature)
                }))
            ]
            .boxed()
        }
    }
}

impl Arbitrary for SingleAccountBindingSignature {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        SingleAccountBindingSignature(Arbitrary::arbitrary(g))
    }
}

impl Arbitrary for AccountBindingSignature {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        AccountBindingSignature::Single(Arbitrary::arbitrary(g))
    }
}

#[derive(Clone)]
pub struct TransactionSigningKey(pub EitherEd25519SecretKey);

impl std::fmt::Debug for TransactionSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "TransactionSigningKey(<secret-key>)")
    }
}

impl Arbitrary for TransactionSigningKey {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        TransactionSigningKey(EitherEd25519SecretKey::Extended(arbitrary_secret_key(g)))
    }
}

impl Arbitrary for Witness {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let opt = u8::arbitrary(g) % 3;
        match opt {
            0 => Witness::Utxo(SpendingSignature::arbitrary(g)),
            1 => Witness::Account(
                SpendingCounter::arbitrary(g),
                SpendingSignature::arbitrary(g),
            ),
            2 => {
                let sk: SecretKey<Ed25519> = arbitrary_secret_key(g);
                Witness::OldUtxo(sk.to_public(), [0u8; 32], Signature::arbitrary(g))
            }
            _ => panic!("not implemented"),
        }
    }
}

impl Arbitrary for UnspecifiedAccountIdentifier {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut b = [0u8; 32];
        for v in b.iter_mut() {
            *v = Arbitrary::arbitrary(g)
        }
        b.into()
    }
}

impl Arbitrary for AccountIdentifier {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if Arbitrary::arbitrary(g) {
            AccountIdentifier::Single(Arbitrary::arbitrary(g))
        } else {
            AccountIdentifier::Multi(Arbitrary::arbitrary(g))
        }
    }
}
