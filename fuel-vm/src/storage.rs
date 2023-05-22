//! Storage backend implementations.

use fuel_storage::Mappable;
use fuel_tx::Contract;
use fuel_types::{AssetId, Bytes32, ContractId, Salt, Word};

mod interpreter;
mod memory;
mod predicate;

pub use interpreter::ContractsAssetsStorage;
pub use interpreter::InterpreterStorage;
pub use memory::MemoryStorage;
pub use predicate::PredicateStorage;

/// The storage table for contract's raw byte code.
pub struct ContractsRawCode;

impl Mappable for ContractsRawCode {
    type Key = Self::OwnedKey;
    type OwnedKey = ContractId;
    type Value = Self::OwnedValue;
    type OwnedValue = Contract;
}

/// The storage table for contract's additional information as salt, root hash, etc.
pub struct ContractsInfo;

impl Mappable for ContractsInfo {
    type Key = Self::OwnedKey;
    type OwnedKey = ContractId;
    /// The salt used during creation of the contract for uniqueness,
    /// and the root hash of the contract's code.
    type Value = (Salt, Bytes32);
    type OwnedValue = Self::Value;
}

/// The storage table for contract's assets balances.
///
/// Lifetime is for optimization to avoid `clone`.
pub struct ContractsAssets;

impl Mappable for ContractsAssets {
    type Key = Self::OwnedKey;
    type OwnedKey = ContractsAssetKey;
    type Value = Word;
    type OwnedValue = Self::Value;
}

/// The storage table for contract's hashed key-value state.
pub struct ContractsState;

impl Mappable for ContractsState {
    type Key = Self::OwnedKey;
    /// The table key is combination of the `ContractId` and `Bytes32` hash of the value's key.
    type OwnedKey = ContractsStateKey;
    /// The table value is hash of the value.
    type Value = Bytes32;
    type OwnedValue = Self::Value;
}

/// The macro defines a new type of double storage key. It is a merge of the two types into one
/// general type that represents the storage key of some entity.
///
/// Both types are represented by one big array. It is done from the performance perspective
/// to minimize the number of copies. The current code doesn't use consumed values and uses
/// it in most cases as on big key(except tests, which require access to sub-keys).
/// But in the future, we may change the layout of the fields based on
/// the performance measurements/new business logic.
#[macro_export]
macro_rules! double_key {
    ($i:ident, $first:ident, $first_getter:ident, $second:ident, $second_getter:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        /// The FuelVM storage double key.
        pub struct $i([u8; { $first::LEN + $second::LEN }]);

        impl Default for $i {
            fn default() -> Self {
                Self([0; { Self::second_end() }])
            }
        }

        impl $i {
            /// The length of the underlying array.
            pub const LEN: usize = $first::LEN + $second::LEN;

            /// Create a new instance of the double storage key from references.
            pub fn new(first: &$first, second: &$second) -> Self {
                let mut default = Self::default();
                default.0[0..Self::first_end()].copy_from_slice(first.as_ref());
                default.0[Self::first_end()..Self::second_end()].copy_from_slice(second.as_ref());
                default
            }

            /// Creates a new instance of double storage key from the array.
            pub fn from_array(array: [u8; { $first::LEN + $second::LEN }]) -> Self {
                Self(array)
            }

            /// Creates a new instance of double storage key from the slice.
            pub fn from_slice(slice: &[u8]) -> Result<Self, core::array::TryFromSliceError> {
                Ok(Self(slice.try_into()?))
            }

            /// Returns the reference to the first sub-key.
            pub fn $first_getter(&self) -> &$first {
                $first::from_bytes_ref(
                    (&self.0[0..Self::first_end()])
                        .try_into()
                        .expect("0..first_end() < first_end() + second_end()"),
                )
            }

            /// Returns the reference to the second sub-key.
            pub fn $second_getter(&self) -> &$second {
                $second::from_bytes_ref(
                    (&self.0[Self::first_end()..Self::second_end()])
                        .try_into()
                        .expect("first_end()..second_end() < first_end() + second_end()"),
                )
            }

            const fn first_end() -> usize {
                $first::LEN
            }

            const fn second_end() -> usize {
                $first::LEN + $second::LEN
            }
        }

        impl From<(&$first, &$second)> for $i {
            fn from(pair: (&$first, &$second)) -> Self {
                Self::new(pair.0, pair.1)
            }
        }

        impl AsRef<[u8]> for $i {
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref()
            }
        }

        impl From<$i> for ($first, $second) {
            fn from(key: $i) -> ($first, $second) {
                let first = &key.0[0..$i::first_end()];
                let second = &key.0[$i::first_end()..$i::second_end()];
                let first = first.try_into().unwrap();
                let second = second.try_into().unwrap();
                (first, second)
            }
        }

        impl From<$i> for [u8; { $first::LEN + $second::LEN }] {
            fn from(key: $i) -> [u8; { $first::LEN + $second::LEN }] {
                key.0
            }
        }
    };
}

double_key!(ContractsAssetKey, ContractId, contract_id, AssetId, asset_id);
double_key!(ContractsStateKey, ContractId, contract_id, Bytes32, state_key);
