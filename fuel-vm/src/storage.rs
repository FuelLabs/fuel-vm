//! Storage backend implementations.

use fuel_storage::Mappable;
use fuel_tx::Contract;
use fuel_types::{
    Bytes32,
    ContractId,
};

mod blob_data;
mod contracts_assets;
mod contracts_state;
mod interpreter;
#[cfg(feature = "test-helpers")]
mod memory;
pub(crate) mod predicate;

pub use blob_data::{
    BlobBytes,
    BlobData,
};
pub use contracts_assets::{
    ContractsAssetKey,
    ContractsAssets,
};
pub use contracts_state::{
    ContractsState,
    ContractsStateData,
    ContractsStateKey,
};
pub use interpreter::{
    ContractsAssetsStorage,
    InterpreterStorage,
};
#[cfg(feature = "test-helpers")]
pub use memory::MemoryStorage;
pub use predicate::PredicateStorage;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// The uploaded bytecode can be in two states: fully uploaded or partially uploaded.
pub enum UploadedBytecode {
    /// The bytecode is partially uploaded.
    Uncompleted {
        /// The cumulative bytecode of `uploaded_subsections_number` parts.
        bytecode: Vec<u8>,
        /// The number of already included subsections of the bytecode.
        uploaded_subsections_number: u16,
    },
    /// The bytecode is fully uploaded and ready to be used.
    Completed(Vec<u8>),
}

/// The storage table for uploaded bytecode.
pub struct UploadedBytecodes;

impl Mappable for UploadedBytecodes {
    /// The key is a Merkle root of the bytecode.
    type Key = Self::OwnedKey;
    type OwnedKey = Bytes32;
    type OwnedValue = UploadedBytecode;
    type Value = Self::OwnedValue;
}

/// The storage table for contract's raw byte code.
pub struct ContractsRawCode;

impl Mappable for ContractsRawCode {
    type Key = Self::OwnedKey;
    type OwnedKey = ContractId;
    type OwnedValue = Contract;
    type Value = [u8];
}

/// The macro defines a new type of double storage key. It is a merge of the two
/// types into one general type that represents the storage key of some entity.
///
/// Both types are represented by one big array. It is done from the performance
/// perspective to minimize the number of copies. The current code doesn't use
/// consumed values and uses it in most cases as on big key(except tests, which
/// require access to sub-keys). But in the future, we may change the layout of the
/// fields based on the performance measurements/new business logic.
#[macro_export]
macro_rules! double_key {
    (
        $i:ident, $first:ident, $first_getter:ident, $second:ident, $second_getter:ident
    ) => {
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
                default.0[Self::first_end()..Self::second_end()]
                    .copy_from_slice(second.as_ref());
                default
            }

            /// Creates a new instance of double storage key from the array.
            pub fn from_array(array: [u8; { $first::LEN + $second::LEN }]) -> Self {
                Self(array)
            }

            /// Creates a new instance of double storage key from the slice.
            pub fn from_slice(
                slice: &[u8],
            ) -> Result<Self, core::array::TryFromSliceError> {
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

        impl TryFrom<&[u8]> for $i {
            type Error = core::array::TryFromSliceError;

            fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
                $i::from_slice(slice)
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $i {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde_with::SerializeAs;
                serde_with::Bytes::serialize_as(&self.0, serializer)
            }
        }

        #[cfg(feature = "serde")]
        impl<'a> serde::Deserialize<'a> for $i {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'a>,
            {
                use serde_with::DeserializeAs;
                let bytes: [u8; $i::LEN] =
                    serde_with::Bytes::deserialize_as(deserializer)?;
                Ok(Self(bytes))
            }
        }
    };
}
