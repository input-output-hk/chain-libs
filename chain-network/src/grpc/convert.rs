use super::proto;
use crate::data::{
    block::{self, Block, BlockEvent, BlockId, ChainPullRequest, Header},
    fragment::Fragment,
    gossip::{Gossip, Node},
};
use crate::error::{self, Error};
use tonic::{Code, Status};

pub(super) fn error_into_grpc(err: Error) -> Status {
    use error::Code::*;

    let code = match err.code() {
        Canceled => Code::Cancelled,
        Unknown => Code::Unknown,
        InvalidArgument => Code::InvalidArgument,
        NotFound => Code::NotFound,
        FailedPrecondition => Code::FailedPrecondition,
        Aborted => Code::Aborted,
        Unimplemented => Code::Unimplemented,
        Internal => Code::Internal,
        Unavailable => Code::Unavailable,
        // When a new case has to be added here, remember to
        // add the corresponding case in error_from_grpc below.
    };

    Status::new(code, err.to_string())
}

pub(super) fn error_from_grpc(e: Status) -> Error {
    use error::Code::*;

    let code = match e.code() {
        Code::Cancelled => Canceled,
        Code::Unknown => Unknown,
        Code::InvalidArgument => InvalidArgument,
        Code::NotFound => NotFound,
        Code::FailedPrecondition => FailedPrecondition,
        Code::Aborted => Aborted,
        Code::Unimplemented => Unimplemented,
        Code::Internal => Internal,
        Code::Unavailable => Unavailable,
        _ => Unknown,
    };

    Error::new(code, e)
}

impl From<Error> for Status {
    #[inline]
    fn from(e: Error) -> Self {
        error_into_grpc(e)
    }
}

impl From<Status> for Error {
    #[inline]
    fn from(status: Status) -> Self {
        error_from_grpc(status)
    }
}

pub trait FromProtobuf<R>: Sized {
    fn from_message(message: R) -> Result<Self, Error>;
}

pub trait IntoProtobuf {
    type Message;
    fn into_message(self) -> Self::Message;
}

pub(super) fn ids_into_repeated_bytes<I>(ids: I) -> Vec<Vec<u8>>
where
    I: IntoIterator,
    I::Item: AsRef<[u8]>,
{
    ids.into_iter().map(|id| id.as_ref().to_vec()).collect()
}

impl FromProtobuf<proto::types::Block> for Block {
    fn from_message(message: proto::types::Block) -> Result<Self, Error> {
        Ok(Block::from_bytes(message.content))
    }
}

impl IntoProtobuf for Block {
    type Message = proto::types::Block;

    fn into_message(self) -> proto::types::Block {
        proto::types::Block {
            content: self.into(),
        }
    }
}

impl FromProtobuf<proto::types::Header> for Header {
    fn from_message(message: proto::types::Header) -> Result<Self, Error> {
        Ok(Header::from_bytes(message.content))
    }
}

impl IntoProtobuf for Header {
    type Message = proto::types::Header;

    fn into_message(self) -> proto::types::Header {
        proto::types::Header {
            content: self.into(),
        }
    }
}

impl FromProtobuf<proto::types::Fragment> for Fragment {
    fn from_message(message: proto::types::Fragment) -> Result<Self, Error> {
        Ok(Fragment::from_bytes(message.content))
    }
}

impl IntoProtobuf for Fragment {
    type Message = proto::types::Fragment;

    fn into_message(self) -> proto::types::Fragment {
        proto::types::Fragment {
            content: self.into(),
        }
    }
}

impl FromProtobuf<proto::node::Gossip> for Gossip {
    fn from_message(message: proto::node::Gossip) -> Result<Self, Error> {
        let gossip = Gossip {
            nodes: message
                .nodes
                .into_iter()
                .map(Node::from_bytes)
                .collect::<Vec<_>>()
                .into(),
        };
        Ok(gossip)
    }
}

impl IntoProtobuf for Gossip {
    type Message = proto::node::Gossip;

    fn into_message(self) -> proto::node::Gossip {
        proto::node::Gossip {
            nodes: self
                .nodes
                .into_vec()
                .into_iter()
                .map(|node| node.into_bytes())
                .collect(),
        }
    }
}

impl FromProtobuf<proto::node::PeersResponse> for Gossip {
    fn from_message(message: proto::node::PeersResponse) -> Result<Self, Error> {
        let gossip = Gossip {
            nodes: message
                .peers
                .into_iter()
                .map(Node::from_bytes)
                .collect::<Vec<_>>()
                .into(),
        };
        Ok(gossip)
    }
}

impl FromProtobuf<proto::node::BlockEvent> for BlockEvent {
    fn from_message(msg: proto::node::BlockEvent) -> Result<Self, Error> {
        use proto::node::block_event::Item::*;

        match msg.item {
            Some(Announce(header)) => {
                let header = Header::from_message(header)?;
                Ok(BlockEvent::Announce(header))
            }
            Some(Solicit(block_ids)) => {
                let block_ids = block::try_ids_from_iter(block_ids.ids)?;
                Ok(BlockEvent::Solicit(block_ids))
            }
            Some(Missing(pull_req)) => {
                let from = block::try_ids_from_iter(pull_req.from)?;
                let to = BlockId::try_from(&pull_req.to[..])?;
                Ok(BlockEvent::Missing(ChainPullRequest { from, to }))
            }
            None => Err(Error::new(
                error::Code::InvalidArgument,
                "one of the BlockEvent variants must be present",
            )),
        }
    }
}

impl IntoProtobuf for BlockEvent {
    type Message = proto::node::BlockEvent;

    fn into_message(self) -> proto::node::BlockEvent {
        use proto::node::block_event::Item;
        let item = match self {
            BlockEvent::Announce(header) => Item::Announce(header.into_message()),
            BlockEvent::Solicit(block_ids) => {
                let block_ids = proto::types::BlockIds {
                    ids: ids_into_repeated_bytes(block_ids.iter()),
                };
                Item::Solicit(block_ids)
            }
            BlockEvent::Missing(ChainPullRequest { from, to }) => {
                let request = proto::node::PullHeadersRequest {
                    from: ids_into_repeated_bytes(from.iter()),
                    to: to.as_bytes().into(),
                };
                Item::Missing(request)
            }
        };
        proto::node::BlockEvent { item: Some(item) }
    }
}
