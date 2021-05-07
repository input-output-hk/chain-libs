use crate::encryption::PublicKey;
use crate::private_voting::Announcement;
use crate::{Ciphertext, Scalar};
use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;

pub(crate) struct ChallengeContext(Blake2b);

fn hash_to_scalar(b: &Blake2b) -> Scalar {
    let mut h = [0u8; 32];
    b.clone().result(&mut h);
    Scalar::from_bytes(&h).unwrap()
}

impl ChallengeContext {
    pub(crate) fn new(
        public_key: &PublicKey,
        ciphers: &[Ciphertext],
        ibas: &[Announcement],
    ) -> Self {
        let mut ctx = Blake2b::new(32);
        ctx.input(&public_key.to_bytes());
        for c in ciphers {
            ctx.input(&c.to_bytes());
        }
        for iba in ibas {
            ctx.input(&iba.i.to_bytes());
            ctx.input(&iba.b.to_bytes());
            ctx.input(&iba.a.to_bytes());
        }
        ChallengeContext(ctx)
    }

    pub(crate) fn first_challenge(&self) -> Scalar {
        hash_to_scalar(&self.0)
    }

    pub(crate) fn second_challenge(&self, ds: &[Ciphertext]) -> Scalar {
        let mut x = self.0.clone();
        for d in ds {
            x.input(&d.to_bytes())
        }
        hash_to_scalar(&x)
    }
}
