//! Defines a `TranscriptProtocol` trait for using a Merlin transcript.

use crate::gang::{ GroupElement, Scalar };
use merlin::Transcript;

use crate::commitment::CommitmentKey;
use crate::gargamel::PublicKey;
use crate::encrypted::PTP;
use crate::Ciphertext;
use crate::shvzk::IBA;

pub trait TranscriptProtocol {
    /// Append a `scalar` with the given `label`.
    fn append_scalar(&mut self, label: &'static [u8], scalar: &Scalar);

    /// Append a `point` with the given `label`.
    fn append_point(&mut self, label: &'static [u8], point: &GroupElement);

    /// Append a `commitment_key` with the given `label`.
    fn append_ck(&mut self, label: &'static [u8], commitment_key: &CommitmentKey);

    /// Append a `public_key` with the given `label`.
    fn append_pk(&mut self, label: &'static [u8], public_key: &PublicKey);

    /// Append `ciphers`, a list of ciphertexts, with the given `label`.
    fn append_ciphers(&mut self, label:&'static [u8], cipherts: &PTP<Ciphertext>);

    /// Append `ibas`, the announcements of the Unit Vector proofs, with the given `label`.
    fn append_ibas(&mut self, label:&'static [u8], ibas: &Vec<IBA>);

    /// Compute a `label`ed challenge variable.
    fn challenge_scalar(&mut self, label: &'static [u8]) -> Scalar;
}

impl TranscriptProtocol for Transcript {
    fn append_scalar(&mut self, label: &'static [u8], scalar: &Scalar) {
        self.append_message(label, &scalar.to_bytes());
    }

    fn append_point(&mut self, label: &'static [u8], point: &GroupElement) {
        self.append_message(label, &point.to_bytes());
    }

    fn append_ck(&mut self, label: &'static [u8], commitment_key: &CommitmentKey) {
        self.append_message(label, &commitment_key.h.to_bytes());
    }

    fn append_pk(&mut self, label: &'static [u8], public_key: &PublicKey) {
        self.append_message(label, &public_key.to_bytes())
    }

    fn append_ciphers(&mut self, label: &'static [u8], ciphers: &PTP<Ciphertext>) {
        for ctxt in ciphers.as_ref() {
            self.append_message(label, &ctxt.to_bytes())
        }
    }

    fn append_ibas(&mut self, label:&'static [u8], ibas: &Vec<IBA>) {
        for iba in ibas {
            self.append_message(label, &iba.to_bytes());
        }
    }

    fn challenge_scalar(&mut self, label: &'static [u8]) -> Scalar {
        let mut buf = [0u8; 32];
        self.challenge_bytes(label, &mut buf);

        loop {
            if let Some(e) = Scalar::from_bytes(&buf) {
                break e;
            }
            self.append_message(b"filling scalar", &[0u8]);
            self.challenge_bytes(label, &mut buf);
        }
    }
}
