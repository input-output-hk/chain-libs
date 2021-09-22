use crate::chain_storable::{Choice, VotePlanId};
use sanakirja::{direct_repr, Storable, UnsizedStorable};
use std::mem;
use zerocopy::AsBytes;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct TransactionCertificate {
    tag: CertificateTag,
    cert: SerializedCertificate,
}

impl TransactionCertificate {
    pub fn from_vote_plan_id(id: VotePlanId) -> Self {
        TransactionCertificate {
            tag: CertificateTag::VotePlan,
            cert: SerializedCertificate { vote_plan: id },
        }
    }

    pub fn from_public_vote_cast(vote: PublicVoteCast) -> Self {
        TransactionCertificate {
            tag: CertificateTag::PublicVoteCast,
            cert: SerializedCertificate {
                public_vote_cast: vote,
            },
        }
    }

    pub fn from_private_vote_cast(vote: PrivateVoteCast) -> Self {
        TransactionCertificate {
            tag: CertificateTag::PrivateVoteCast,
            cert: SerializedCertificate {
                private_vote_cast: vote,
            },
        }
    }

    pub fn as_vote_plan(&self) -> Option<&VotePlanId> {
        unsafe {
            match self {
                Self {
                    tag: CertificateTag::VotePlan,
                    cert: SerializedCertificate { vote_plan },
                } => Some(vote_plan),
                _ => None,
            }
        }
    }

    pub fn as_public_vote_cast(&self) -> Option<&PublicVoteCast> {
        unsafe {
            match self {
                Self {
                    tag: CertificateTag::PublicVoteCast,
                    cert: SerializedCertificate { public_vote_cast },
                } => Some(public_vote_cast),
                _ => None,
            }
        }
    }

    pub fn as_private_vote_cast(&self) -> Option<&PrivateVoteCast> {
        unsafe {
            match self {
                Self {
                    tag: CertificateTag::PrivateVoteCast,
                    cert: SerializedCertificate { private_vote_cast },
                } => Some(private_vote_cast),
                _ => None,
            }
        }
    }
}

direct_repr!(TransactionCertificate);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(u8)]
pub(crate) enum CertificateTag {
    VotePlan = 0,
    PublicVoteCast = 1,
    PrivateVoteCast = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
union SerializedCertificate {
    vote_plan: VotePlanId,
    public_vote_cast: PublicVoteCast,
    private_vote_cast: PrivateVoteCast,
}

impl SerializedCertificate {
    fn as_bytes(&self) -> &[u8; mem::size_of::<Self>()] {
        unsafe { std::mem::transmute(self) }
    }
}

impl PartialEq for SerializedCertificate {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes().eq(other.as_bytes())
    }
}

impl Eq for SerializedCertificate {}

impl PartialOrd for SerializedCertificate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl Ord for SerializedCertificate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl std::fmt::Debug for SerializedCertificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.as_bytes()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct PublicVoteCast {
    pub vote_plan_id: VotePlanId,
    pub proposal_index: u8,
    pub choice: Choice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct PrivateVoteCast {
    pub vote_plan_id: VotePlanId,
    pub proposal_index: u8,
}
