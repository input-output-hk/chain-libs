use crate::vote::{EncryptedVote, ProofOfCorrectVote};
use chain_vote::{EncryptingVoteKey, Vote};
use merlin::Transcript;
use rand_core::{CryptoRng, RngCore};

#[allow(dead_code)]
pub fn encrypt_vote<R: RngCore + CryptoRng>(
    rng: &mut R,
    transcript: &mut Transcript,
    public_key: &EncryptingVoteKey,
    vote: Vote,
) -> (EncryptedVote, ProofOfCorrectVote) {
    let (ev, proof) = chain_vote::encrypt_vote(rng, transcript, public_key, vote);
    (
        EncryptedVote::from_inner(ev),
        ProofOfCorrectVote::from_inner(proof),
    )
}
