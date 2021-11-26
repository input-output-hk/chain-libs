mod builder;
mod element;
mod input;
mod io;
mod payload;
#[allow(clippy::module_inception)]
mod transaction;
mod transfer;
mod utxo;
mod witness;

#[cfg(any(test, feature = "property-test-api"))]
pub mod test;

use chain_core::{
    mempack::ReadBuf,
    property::{Deserialize, ReadError, Serialize, WriteError},
};

// to remove..
pub use builder::{
    SetAuthData, SetIOs, SetPayload, SetTtl, SetWitnesses, TxBuilder, TxBuilderState,
};
pub use element::*;
pub use input::*;
pub use io::{Error, InputOutput, InputOutputBuilder, OutputPolicy};
pub use payload::{NoExtra, Payload, PayloadAuthData, PayloadAuthSlice, PayloadData, PayloadSlice};
pub use transaction::*;
pub use transfer::*;
pub use utxo::*;
pub use witness::*;

impl<Extra: Payload> Serialize for Transaction<Extra> {
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), WriteError> {
        writer.write_all(self.as_ref()).map_err(|e| e.into())
    }
}

impl<Extra: Payload> Deserialize for Transaction<Extra> {
    fn deserialize(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let utx = UnverifiedTransactionSlice::from(buf.get_slice_end());
        match utx.check() {
            Ok(tx) => Ok(tx.to_owned()),
            Err(e) => Err(ReadError::StructureInvalid(e.to_string())),
        }
    }
}

// TEMPORARY
pub type AuthenticatedTransaction<P> = Transaction<P>;
