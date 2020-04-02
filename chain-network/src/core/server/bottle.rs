use super::PushStream;
use crate::data::BottleInSea;
use crate::data::Peer;
use crate::error::Error;
use async_trait::async_trait;
use futures::stream::Stream;

/// Interface for the blockchain node to exchange "bottle in the sea" messages.
#[async_trait]
pub trait BottleInSeaService {
    /// The type of outbound asynchronous streams returned by the
    /// `subscription` method.
    type SubscriptionStream: Stream<Item = Result<BottleInSea, Error>> + Send + Sync;

    /// Called by the protocol implementation to establish a
    /// bidirectional subscription stream.
    /// The inbound stream is passed to the asynchronous method,
    /// which resolves to the outbound stream.
    async fn bottle_in_sea_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<BottleInSea>,
    ) -> Result<Self::SubscriptionStream, Error>;
}
