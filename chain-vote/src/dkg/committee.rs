//! Implementation of the distributed key generation (DKG)
//! procedure presented by Gennaro, Jarecki, Krawczyk and Rabin in
//! ["Secure distributed key generation for discrete-log based cryptosystems."](https://link.springer.com/article/10.1007/s00145-006-0347-3).
//! The distinction with the original protocol lies in the use of hybrid
//! encryption. We use the description and notation presented in the technical
//! [spec](https://github.com/input-output-hk/treasury-crypto/blob/master/docs/voting_protocol_spec/Treasury_voting_protocol_spec.pdf),
//! written by Dmytro Kaidalov.

use super::procedure_keys::{
    MemberCommunicationKey, MemberCommunicationPublicKey, MemberPublicKey, MemberSecretKey,
};
use crate::encryption::{HybridCiphertext, PublicKey, SecretKey};
use crate::errors::DkgError;
use crate::gang::{GroupElement, Scalar};
use crate::math::Polynomial;
use crate::Crs;
use rand_core::{CryptoRng, RngCore};

pub type DistributedKeyGeneration = MemberState1;

/// Initial state generated by a Member, corresponding to round 1.
#[derive(Clone)]
pub struct MemberState1 {
    sk_share: MemberSecretKey,
    threshold: usize,
    nr_members: usize,
    owner_index: usize,
    crs: Crs,
    apubs: Vec<GroupElement>,
    coeff_comms: Vec<GroupElement>,
    encrypted_shares: Vec<IndexedEncryptedShares>,
}

/// State of the member corresponding to round 2.
#[derive(Clone)]
pub struct MemberState2 {
    threshold: usize,
    misbehaving_parties: Vec<MisbehavingPartiesState1>,
}

/// Type that contains the index of the receiver, and its two encrypted
/// shares.
pub(crate) type IndexedEncryptedShares = (usize, HybridCiphertext, HybridCiphertext);

// todo: third element should be a proof of misbehaviour. waiting for PR542 to resolve
/// Type that contains misbehaving parties detected in round 1. These
/// consist of the misbehaving member's index, the error which failed,
/// and a proof of correctness of the misbehaviour claim.
type MisbehavingPartiesState1 = (usize, DkgError, usize);

/// State of the members after round 1. This structure contains the indexed encrypted
/// shares of every other participant, `indexed_shares`, and the committed coefficients
/// of the generated polynomials, `committed_coeffs`.
#[derive(Clone)]
pub struct MembersFetchedState1 {
    indexed_shares: IndexedEncryptedShares,
    committed_coeffs: Vec<GroupElement>,
}

impl MembersFetchedState1 {
    fn get_index(&self) -> usize {
        self.indexed_shares.0
    }
}

impl MemberState1 {
    /// Generate a new member state from random. This is round 1 of the protocol. Receives as
    /// input the threshold `t`, the expected number of participants, `n`, common reference string
    /// `crs`, `committee_pks`, and the party's index `my`. Initiates a Pedersen-VSS as a dealer,
    /// and returns the committed coefficients of its polynomials, together with encryption of the
    /// shares of the other different members.
    pub fn init<R: RngCore + CryptoRng>(
        rng: &mut R,
        t: usize,
        n: usize,
        crs: &Crs, // TODO: document
        committee_pks: &[MemberCommunicationPublicKey],
        my: usize,
    ) -> MemberState1 {
        assert_eq!(committee_pks.len(), n);
        assert!(t > 0);
        assert!(t <= n);
        assert!(t > n / 2);
        assert!(my < n);

        let pcomm = Polynomial::random(rng, t);
        let pshek = Polynomial::random(rng, t);

        let mut apubs = Vec::with_capacity(t);
        let mut coeff_comms = Vec::with_capacity(t);

        for (ai, bi) in pshek.get_coefficients().zip(pcomm.get_coefficients()) {
            let apub = GroupElement::generator() * ai;
            let coeff_comm = &apub + crs * bi;
            apubs.push(apub);
            coeff_comms.push(coeff_comm);
        }

        let mut encrypted_shares: Vec<IndexedEncryptedShares> = Vec::with_capacity(n - 1);
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            // don't generate share for self
            if i == my {
                continue;
            } else {
                let idx = Scalar::from_u64((i + 1) as u64);
                let share_comm = pcomm.evaluate(&idx);
                let share_shek = pshek.evaluate(&idx);

                let pk = &committee_pks[i];

                let ecomm = pk.hybrid_encrypt(&share_comm.to_bytes(), rng);
                let eshek = pk.hybrid_encrypt(&share_shek.to_bytes(), rng);

                encrypted_shares.push((i, ecomm, eshek));
            }
        }

        MemberState1 {
            sk_share: MemberSecretKey(SecretKey {
                sk: pshek.at_zero(),
            }),
            crs: crs.clone(),
            threshold: t,
            nr_members: n,
            owner_index: my + 1, // committee member are 1-indexed
            apubs,
            coeff_comms,
            encrypted_shares,
        }
    }

    /// Function to proceed to phase 2. It checks and keeps track of misbehaving parties. If this
    /// step does not validate, the member is not allowed to proceed to phase 3.
    pub fn to_phase_2(
        &self,
        secret_key: &MemberCommunicationKey,
        members_state: &Vec<MembersFetchedState1>,
    ) -> MemberState2 {
        let mut misbehaving_parties: Vec<MisbehavingPartiesState1> = Vec::new();
        for fetched_data in members_state {
            if let (Some(comm), Some(shek)) =
                secret_key.decrypt_shares(fetched_data.indexed_shares.clone())
            {
                let index_pow = Scalar::from_u64(self.owner_index as u64)
                    .exp_iter()
                    .take(self.threshold + 1);

                let check_element = GroupElement::generator() * shek + &self.crs * comm;
                #[cfg(feature = "ristretto255")]
                let multi_scalar = GroupElement::vartime_multiscalar_multiplication(
                    index_pow,
                    fetched_data.committed_coeffs.clone(),
                );
                #[cfg(not(feature = "ristretto255"))]
                let multi_scalar = GroupElement::multiscalar_multiplication(
                    index_pow,
                    fetched_data.committed_coeffs.clone(),
                );

                if check_element != multi_scalar {
                    // todo: should we instead store the sender's index?
                    misbehaving_parties.push((
                        fetched_data.get_index().clone(),
                        DkgError::ShareValidityFailed,
                        0,
                    ));
                }
            } else {
                // todo: handle the proofs. Might not be the most optimal way of handling these two
                misbehaving_parties.push((
                    fetched_data.get_index().clone(),
                    DkgError::ScalarOutOfBounds,
                    0,
                ));
            }
        }

        MemberState2 {
            misbehaving_parties,
            threshold: self.threshold,
        }
    }

    pub fn secret_key(&self) -> &MemberSecretKey {
        &self.sk_share
    }

    pub fn public_key(&self) -> MemberPublicKey {
        MemberPublicKey(PublicKey {
            pk: self.apubs[0].clone(),
        })
    }
}

impl MemberState2 {
    pub fn validate(&self) -> Result<Self, DkgError> {
        if self.misbehaving_parties.len() == self.threshold {
            return Err(DkgError::MisbehaviourHigherThreshold);
        }

        Ok(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    #[test]
    fn valid_phase_2() {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);

        let mut shared_string =
            b"Example of a shared string. This should be VotePlan.to_id()".to_owned();
        let h = Crs::from_hash(&mut shared_string);

        let mc1 = MemberCommunicationKey::new(&mut rng);
        let mc2 = MemberCommunicationKey::new(&mut rng);
        let mc = [mc1.to_public(), mc2.to_public()];

        let threshold = 2;
        let nr_members = 2;

        let m1 = DistributedKeyGeneration::init(&mut rng, threshold, nr_members, &h, &mc, 0);
        let m2 = DistributedKeyGeneration::init(&mut rng, threshold, nr_members, &h, &mc, 1);

        // Now, party one fetches the state of the other parties, mainly party two and three
        let fetched_state = vec![MembersFetchedState1 {
            indexed_shares: m2.encrypted_shares[0].clone(),
            committed_coeffs: m2.coeff_comms.clone(),
        }];

        let phase_2 = m1.to_phase_2(&mc1, &fetched_state);

        assert!(phase_2.validate().is_ok());
    }
    #[test]
    fn invalid_phase_2() {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);

        let mut shared_string =
            b"Example of a shared string. This should be VotePlan.to_id()".to_owned();
        let h = Crs::from_hash(&mut shared_string);

        let mc1 = MemberCommunicationKey::new(&mut rng);
        let mc2 = MemberCommunicationKey::new(&mut rng);
        let mc3 = MemberCommunicationKey::new(&mut rng);
        let mc = [mc1.to_public(), mc2.to_public(), mc3.to_public()];

        let threshold = 2;
        let nr_members = 3;

        let m1 = DistributedKeyGeneration::init(&mut rng, threshold, nr_members, &h, &mc, 0);
        let m2 = DistributedKeyGeneration::init(&mut rng, threshold, nr_members, &h, &mc, 1);
        let m3 = DistributedKeyGeneration::init(&mut rng, threshold, nr_members, &h, &mc, 2);

        // Now, party one fetches invalid state of the other parties, mainly party two and three
        let fetched_state = vec![
            MembersFetchedState1 {
                indexed_shares: m2.encrypted_shares[0].clone(),
                committed_coeffs: vec![GroupElement::zero(); 3],
            },
            MembersFetchedState1 {
                indexed_shares: m3.encrypted_shares[0].clone(),
                committed_coeffs: vec![GroupElement::zero(); 3],
            },
        ];

        let phase_2_faked = m1.to_phase_2(&mc1, &fetched_state);
        // todo: we probably want to check for a particular error here
        assert!(phase_2_faked.validate().is_err());
    }
}
