mod blob;
mod chargeable_transaction;
mod create;
pub mod input;
mod mint;
pub mod output;
mod script;
mod storage;
mod upgrade;
mod upload;
mod utxo_id;
mod witness;

pub use blob::{
    Blob,
    BlobBody,
    BlobIdExt,
    BlobMetadata,
};
pub use chargeable_transaction::{
    ChargeableMetadata,
    ChargeableTransaction,
};
pub use create::{
    Create,
    CreateBody,
    CreateMetadata,
};
pub use mint::Mint;
pub use script::{
    Script,
    ScriptBody,
    ScriptCode,
    ScriptV2,
};
pub use storage::StorageSlot;
pub use upgrade::{
    Upgrade,
    UpgradeBody,
    UpgradeMetadata,
    UpgradePurpose,
};
pub use upload::{
    Upload,
    UploadBody,
    UploadMetadata,
    UploadSubsection,
};
pub use utxo_id::UtxoId;
pub use witness::Witness;

#[cfg(feature = "da-compression")]
pub use self::{
    mint::CompressedMint,
    utxo_id::CompressedUtxoId,
};

pub fn compute_transaction_id<T: fuel_types::canonical::Serialize>(
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
