mod declaration;
mod index;
mod ledger;
mod witness;

pub use declaration::{
    DeclElement, Declaration, DeclarationError, Identifier, WitnessMultisigData,
};
pub use ledger::{Ledger, LedgerError};
pub use witness::{Witness, WitnessBuilder};

pub use index::{Index, TreeIndex};

#[cfg(any(test, feature = "property-test-api"))]
mod test {
    use super::*;

    #[cfg(test)]
    use crate::transaction::{TransactionSignData, TransactionSignDataHash};
    #[cfg(test)]
    use crate::{account, key};
    #[cfg(test)]
    use chain_crypto::{PublicKey, SecretKey};
    use quickcheck::{Arbitrary, Gen};
    #[cfg(test)]
    use rand_core::{CryptoRng, RngCore};

    #[cfg(test)]
    fn make_keypair<R: RngCore + CryptoRng>(
        rng: &mut R,
    ) -> (
        SecretKey<account::AccountAlg>,
        PublicKey<account::AccountAlg>,
    ) {
        let sk = SecretKey::generate(rng);
        let pk = sk.to_public();
        (sk, pk)
    }

    #[cfg(test)]
    fn make_participant<R: RngCore + CryptoRng>(
        rng: &mut R,
        idx: usize,
    ) -> (
        SecretKey<account::AccountAlg>,
        PublicKey<account::AccountAlg>,
        key::Hash,
        Index,
    ) {
        let (sk, pk) = make_keypair(rng);
        let o = key::Hash::hash_bytes(pk.as_ref());
        let idx = Index::from_u8(idx as u8).unwrap();
        (sk, pk, o, idx)
    }

    #[test]
    fn multisig_works_depth1() {
        let mut rng = rand_core::OsRng;
        let (sk1, pk1, o1, i1) = make_participant(&mut rng, 0);
        let (sk2, pk2, o2, i2) = make_participant(&mut rng, 1);
        let (sk3, pk3, o3, i3) = make_participant(&mut rng, 2);

        let decl = Declaration {
            threshold: 2,
            owners: vec![
                DeclElement::Owner(o1),
                DeclElement::Owner(o2),
                DeclElement::Owner(o3),
            ],
        };

        let fake_sign_data: TransactionSignData = vec![1, 2, 3].into();
        let fake_sign_data_hash = TransactionSignDataHash::digest(&fake_sign_data);
        let fake_block0_hash = key::Hash::hash_bytes(&[1, 2, 3, 4, 5, 6, 7]);
        let msg = WitnessMultisigData::new(&fake_block0_hash, &fake_sign_data_hash);

        // test participant 1 and 3
        {
            let mut witness_builder = WitnessBuilder::new();
            witness_builder.append(TreeIndex::D1(i1), pk1.clone(), sk1.sign(&msg).coerce());
            witness_builder.append(TreeIndex::D1(i2), pk2.clone(), sk2.sign(&msg).coerce());
            witness_builder.append(TreeIndex::D1(i3), pk3.clone(), sk3.sign(&msg).coerce());
            let witness = witness_builder.finalize();

            assert!(
                witness.verify(&decl, &msg),
                "multisignature [1+3] 2/3 failed"
            );
        }

        // test participant 3 and 2
        {
            let mut witness_builder = WitnessBuilder::new();
            witness_builder.append(TreeIndex::D1(i3), pk3.clone(), sk3.sign(&msg).coerce());
            witness_builder.append(TreeIndex::D1(i2), pk2, sk2.sign(&msg).coerce());
            let witness = witness_builder.finalize();

            assert!(
                witness.verify(&decl, &msg),
                "multisignature [3+2] 2/3 failed"
            );
        }

        // test mislabelled participant 1 and participant 3
        {
            let mut witness_builder = WitnessBuilder::new();
            witness_builder.append(
                TreeIndex::D1(i2), // should be i1 to work
                pk1.clone(),
                sk1.sign(&msg).coerce(),
            );
            witness_builder.append(TreeIndex::D1(i3), pk3, sk3.sign(&msg).coerce());
            let witness = witness_builder.finalize();

            assert!(
                !witness.verify(&decl, &msg),
                "multisignature mislabelled 2/3 succeeded"
            );
        }

        // test threshold not met
        {
            let mut witness_builder = WitnessBuilder::new();
            witness_builder.append(TreeIndex::D1(i1), pk1, sk1.sign(&msg).coerce());
            let witness = witness_builder.finalize();

            assert!(
                !witness.verify(&decl, &msg),
                "multisignature not enough threshold 2/3 succeeded"
            );
        }
    }

    impl Arbitrary for Identifier {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut b = [0u8; 32];
            for v in b.iter_mut() {
                *v = Arbitrary::arbitrary(g)
            }
            Identifier::from(b)
        }
    }
}
