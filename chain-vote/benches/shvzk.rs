use chain_vote::*;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;
use merlin::Transcript;

fn common(rng: &mut ChaCha20Rng) -> (EncryptingVoteKey, EncryptingVote) {
    let shared_string = b"Example of a shared string. This should be VotePlan.to_id()";
    let mut member_transcript = Transcript::new(b"Member transcript");
    member_transcript.append_message(b"Election identifier", shared_string);

    let mc1 = MemberCommunicationKey::new(rng);
    let mc = [mc1.to_public()];

    let threshold = 1;

    let m1 = MemberState::new(rng, threshold, &mut member_transcript, &mc, 0);

    let participants = vec![m1.public_key()];
    let ek = EncryptingVoteKey::from_participants(&participants);

    let vote_options = 2;
    let vote = Vote::new(vote_options, 0);

    let ev = EncryptingVote::prepare(rng, ek.as_raw(), &vote);
    (ek, ev)
}

fn encrypt_and_prove(c: &mut Criterion) {
    let mut rng = ChaCha20Rng::from_seed([0u8; 32]);
    let mut group = c.benchmark_group("Encrypt and prove");

    let shared_string = b"Example of a shared string. This should be VotePlan.to_id()";
    let mut member_transcript = Transcript::new(b"Member transcript");
    member_transcript.append_message(b"Election identifier", shared_string);

    let (ek, _) = common(&mut rng);

    for &number_candidates in [2usize, 4, 8, 16, 32, 64, 128, 256, 512, 1024].iter() {
        let parameter_string = format!("{} candidates", number_candidates);
        group.bench_with_input(
            BenchmarkId::new("Encrypt and Prove", parameter_string),
            &number_candidates,
            |b, &nr| b.iter(|| encrypt_vote(&mut rng, &mut member_transcript, &ek, Vote::new(nr, 0))),
        );
    }

    group.finish();
}

fn verify(c: &mut Criterion) {
    let mut rng = ChaCha20Rng::from_seed([0u8; 32]);
    let mut group = c.benchmark_group("Verify vote proof");

    let shared_string = b"Example of a shared string. This should be VotePlan.to_id()";
    let mut prover_transcript = Transcript::new(b"Election transcript");
    prover_transcript.append_message(b"Election identifier", shared_string);

    let mut verifier_transcript = Transcript::new(b"Election transcript");
    verifier_transcript.append_message(b"Election identifier", shared_string);

    let (ek, _) = common(&mut rng);

    for &number_candidates in [2usize, 4, 8, 16, 32, 64, 128, 256, 512, 1024].iter() {
        let (vote, proof) = encrypt_vote(&mut rng, &mut prover_transcript, &ek, Vote::new(number_candidates, 0));
        let parameter_string = format!("{} candidates", number_candidates);
        group.bench_with_input(
            BenchmarkId::new("Verify with", parameter_string),
            &number_candidates,
            |b, _| b.iter(|| verify_vote(&mut verifier_transcript, &ek, &vote, &proof)),
        );
    }

    group.finish();
}

criterion_group!(
    name = shvzk;
    config = Criterion::default();
    targets =
    encrypt_and_prove,
    verify,
);

criterion_main!(shvzk);
