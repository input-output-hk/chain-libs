use crate::{
    certificate::{
        DecryptedPrivateTally, DecryptedPrivateTallyError, DecryptedPrivateTallyProposal,
    },
    testing::data::CommitteeMembersManager,
    vote::VotePlanStatus,
};

use chain_vote::tally::batch_decrypt;
use rand::thread_rng;

pub fn decrypt_tally(
    vote_plan_status: &VotePlanStatus,
    members: &CommitteeMembersManager,
) -> Result<DecryptedPrivateTally, DecryptedPrivateTallyError> {
    let members_pks: Vec<chain_vote::MemberPublicKey> = members
        .members()
        .iter()
        .map(|member| member.public_key())
        .collect();

    let (shares, tallies): (Vec<_>, Vec<_>) = vote_plan_status
        .proposals
        .iter()
        .map(|proposal| {
            let tally_state = proposal.tally;
            let encrypted_tally = tally_state.private_encrypted().unwrap().clone();
            let decrypt_shares = members
                .members()
                .iter()
                .map(|member| member.secret_key())
                .map(|secret_key| {
                    encrypted_tally
                        .0
                        .partial_decrypt(&mut thread_rng(), secret_key)
                })
                .collect::<Vec<_>>();
            let validated_tally = encrypted_tally
                .0
                .validate_partial_decryptions(&members_pks, &decrypt_shares)
                .expect("Invalid shares");

            (decrypt_shares, validated_tally)
        })
        .unzip();

    let tallies = batch_decrypt(&tallies).unwrap();

    let proposals = shares
        .into_iter()
        .zip(tallies.into_iter())
        .map(|(shares, tally)| DecryptedPrivateTallyProposal {
            decrypt_shares: shares.into_boxed_slice(),
            tally_result: tally.votes.into_boxed_slice(),
        })
        .collect();

    DecryptedPrivateTally::new(proposals)
}
