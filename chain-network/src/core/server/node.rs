use super::{BlockService, BottleInSeaService, FragmentService, GossipService};

/// Interface to application logic of the blockchain node server.
///
/// An implementation of a blockchain node implements this trait to
/// serve the network protocols using node's subsystems such as
/// block storage and fragment processor.
pub trait Node {
    /// The implementation of the block service.
    type BlockService: BlockService + Send + Sync;

    /// The implementation of the fragment service.
    type FragmentService: FragmentService + Send + Sync;

    /// The implementation of the gossip service.
    type GossipService: GossipService + Send + Sync;

    /// The implementation of the "bottle in the sea" gossip service.
    type BottleInSeaService: BottleInSeaService + Send + Sync;

    /// Instantiates the block service,
    /// if supported by this node.
    fn block_service(&self) -> Option<&Self::BlockService>;

    /// Instantiates the fragment service,
    /// if supported by this node.
    fn fragment_service(&self) -> Option<&Self::FragmentService>;

    /// Instantiates the gossip service,
    /// if supported by this node.
    fn gossip_service(&self) -> Option<&Self::GossipService>;

    /// Instantiates the "bottle in the sea" service,
    /// if supported by this node.
    fn bottle_in_sea_service(&self) -> Option<&Self::BottleInSeaService>;
}
