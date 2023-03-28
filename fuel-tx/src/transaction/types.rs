mod create;
pub mod input;
mod mint;
mod output;
mod script;
mod storage;
mod utxo_id;
mod witness;

use crate::{ConsensusParameters, TxId};
pub use create::Create;
use fuel_crypto::Hasher;
use fuel_types::bytes::SerializableVec;
pub use mint::Mint;
pub use output::{Output, OutputRepr};
pub use script::Script;
pub use storage::StorageSlot;
pub use utxo_id::UtxoId;
pub use witness::Witness;

pub fn compute_transaction_id<T: SerializableVec + Clone>(params: &ConsensusParameters, tx: &mut T) -> TxId {
    let mut hasher = Hasher::default();
    // chain ID
    hasher.input(params.chain_id.to_be_bytes());
    // transaction bytes
    hasher.input(tx.to_bytes().as_slice());
    hasher.finalize()
}
