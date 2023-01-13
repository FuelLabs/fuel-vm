//! Storage backend implementations.

use fuel_storage::Mappable;
use fuel_tx::Contract;
use fuel_types::{AssetId, Bytes32, ContractId, Salt, Word};

mod interpreter;
mod memory;
mod predicate;

pub use interpreter::InterpreterStorage;
pub use memory::MemoryStorage;
pub use predicate::PredicateStorage;

/// The storage table for contract's raw byte code.
pub struct ContractsRawCode;

impl Mappable for ContractsRawCode {
    type Key<'a> = ContractId;
    type SetValue = [u8];
    type GetValue = Contract;
}

/// The storage table for contract's additional information as salt, root hash, etc.
pub struct ContractsInfo;

impl Mappable for ContractsInfo {
    type Key<'a> = ContractId;
    /// `Salt` - is the salt used during creation of the contract for uniques.
    /// `Bytes32` - is the root hash of the contract's code.
    type SetValue = (Salt, Bytes32);
    type GetValue = Self::SetValue;
}

/// The storage table for contract's assets balances.
///
/// Lifetime is for optimization to avoid `clone`.
pub struct ContractsAssets;

impl Mappable for ContractsAssets {
    type Key<'a> = (&'a ContractId, &'a AssetId);
    type SetValue = Word;
    type GetValue = Self::SetValue;
}

/// The storage table for contract's hashed key-value state.
///
/// Lifetime is for optimization to avoid `clone`.
pub struct ContractsState;

impl Mappable for ContractsState {
    /// The table key is combination of the `ContractId` and `Bytes32` hash of the value's key.
    type Key<'a> = (&'a ContractId, &'a Bytes32);
    /// The table value is hash of the value.
    type SetValue = Bytes32;
    type GetValue = Self::SetValue;
}
