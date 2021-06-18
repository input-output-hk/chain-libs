/// Represents an error in Distributed Key Generation protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum DkgError {
    /// This error occurs when a scalar parsing failed, due to the
    /// byte-array representing a scalar out of bounds.
    #[cfg_attr(feature = "std", error("Scalar out of bounds."))]
    ScalarOutOfBounds,
    /// This error occurs when the check of validity of the shares
    /// fails.
    #[cfg_attr(feature = "std", error("Share validity check failed."))]
    ShareValidityFailed,
    /// This error occurs when too many members misbehaved.
    #[cfg_attr(feature = "std", error("Misbehaviours higher than threshold."))]
    MisbehaviourHigherThreshold,
}
