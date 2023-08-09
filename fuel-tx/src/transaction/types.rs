mod create;
pub mod input;
mod mint;
mod output;
mod script;
mod storage;
mod utxo_id;
mod witness;

pub use create::Create;
pub use mint::Mint;
pub use output::{
    Output,
    OutputRepr,
};
pub use script::Script;
pub use storage::StorageSlot;
pub use utxo_id::UtxoId;
pub use witness::Witness;

#[cfg(feature = "std")]
pub fn compute_transaction_id<T: fuel_types::bytes::SerializableVec + Clone>(
    chain_id: &fuel_types::ChainId,
    tx: &mut T,
) -> crate::TxId {
    let mut hasher = fuel_crypto::Hasher::default();
    // chain ID
    hasher.input(chain_id.to_be_bytes());
    // transaction bytes
    hasher.input(tx.to_bytes().as_slice());
    hasher.finalize()
}
