//! Atomic types of the FuelVM.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unsafe_code)]
#![warn(missing_docs)]
#![deny(unused_crate_dependencies)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]
// `fuel-derive` requires `fuel_types` import
// TODO: Move canonical serialization to `fuel-canonical` crate
#![allow(unused_crate_dependencies)]
extern crate self as fuel_types;

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

pub mod canonical;

mod array_types;
#[cfg(feature = "alloc")]
mod fmt;
mod numeric_types;

pub use array_types::*;
#[cfg(feature = "alloc")]
pub use fmt::*;
pub use numeric_types::*;

/// Word-aligned bytes serialization functions.
pub mod bytes;

#[cfg(test)]
mod tests;

/// Register ID type
pub type RegisterId = usize;

/// Register value type
pub type Word = u64;

/// 6-bits immediate value type
pub type Immediate06 = u8;

/// 12-bits immediate value type
pub type Immediate12 = u16;

/// 18-bits immediate value type
pub type Immediate18 = u32;

/// 24-bits immediate value type
pub type Immediate24 = u32;

#[cfg(test)]
mod playground {
    use crate::{
        AssetId,
        ContractId,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RawKey(u32);

    pub trait Compressible {
        type Compressed;
    }

    pub trait CompressibleBy<C>: Compressible
    where
        C: ?Sized,
    {
        fn compress(&self, compressor: &mut C) -> Self::Compressed;
    }

    pub trait Compressor<Type>
    where
        Type: Compressible,
    {
        fn compress(&mut self, value: &Type) -> Type::Compressed;
    }

    impl Compressible for AssetId {
        type Compressed = RawKey;
    }

    impl<C> CompressibleBy<C> for AssetId
    where
        C: Compressor<Self>,
    {
        fn compress(&self, compressor: &mut C) -> Self::Compressed {
            compressor.compress(self)
        }
    }

    impl Compressible for ContractId {
        type Compressed = RawKey;
    }

    impl<C> CompressibleBy<C> for ContractId
    where
        C: Compressor<Self>,
    {
        fn compress(&self, compressor: &mut C) -> Self::Compressed {
            compressor.compress(self)
        }
    }

    // #[derive(CompressibleBy)]
    pub struct ComplexStruct {
        asset_id: AssetId,
        contract_id: ContractId,
        // #[compressible_by(skip)]
        some_field: u64,
    }

    // Generated code from `#[derive(CompressibleBy)]`
    const _: () = {
        use super::*;

        pub struct CompressedComplexStruct {
            asset_id: <AssetId as Compressible>::Compressed,
            contract_id: <ContractId as Compressible>::Compressed,
        }

        impl Compressible for ComplexStruct {
            type Compressed = CompressedComplexStruct;
        }

        impl<C> CompressibleBy<C> for ComplexStruct
        where
            AssetId: CompressibleBy<C>,
            ContractId: CompressibleBy<C>,
        {
            fn compress(&self, register: &mut C) -> Self::Compressed {
                let asset_id = self.asset_id.compress(register);
                let contract_id = self.contract_id.compress(register);

                CompressedComplexStruct {
                    asset_id,
                    contract_id,
                }
            }
        }
    };

    // #[derive(CompressibleBy)]
    pub struct ComplexComplexStruct<G> {
        complex: ComplexStruct,
        generic: G,
        // #[compact(skip)]
        some_field: u64,
    }

    // Generated code from `#[derive(CompressibleBy)]`
    const _: () = {
        use super::*;

        pub struct CompressedComplexComplexStruct<G>
        where
            G: Compressible,
        {
            complex: <ComplexStruct as Compressible>::Compressed,
            generic: <G as Compressible>::Compressed,
        }

        impl<G> Compressible for ComplexComplexStruct<G>
        where
            G: Compressible,
        {
            type Compressed = CompressedComplexComplexStruct<G>;
        }

        impl<G, C> CompressibleBy<C> for ComplexComplexStruct<G>
        where
            ComplexStruct: CompressibleBy<C>,
            G: CompressibleBy<C>,
        {
            fn compress(&self, register: &mut C) -> Self::Compressed {
                let complex = self.complex.compress(register);
                let generic = self.generic.compress(register);

                CompressedComplexComplexStruct { complex, generic }
            }
        }
    };

    mod tests {
        use super::*;
        use std::collections::HashMap;

        #[derive(Default)]
        struct MapRegister {
            assets: HashMap<AssetId, RawKey>,
            contracts: HashMap<ContractId, RawKey>,
        }

        impl Compressor<AssetId> for MapRegister {
            fn compress(&mut self, type_to_compact: &AssetId) -> RawKey {
                let size = self.assets.len();
                let entry = self
                    .assets
                    .entry(*type_to_compact)
                    .or_insert_with(|| RawKey(size as u32));

                *entry
            }
        }

        impl Compressor<ContractId> for MapRegister {
            fn compress(&mut self, type_to_compact: &ContractId) -> RawKey {
                let size = self.contracts.len();
                let entry = self
                    .contracts
                    .entry(*type_to_compact)
                    .or_insert_with(|| RawKey(size as u32));
                *entry
            }
        }

        #[test]
        fn can_register() {
            let mut register = MapRegister::default();
            let complex_struct = ComplexStruct {
                asset_id: [1; 32].into(),
                contract_id: [2; 32].into(),
                some_field: 3,
            };
            let compressed_complex = complex_struct.compress(&mut register);
            assert_eq!(compressed_complex.asset_id.0, 0);
            assert_eq!(compressed_complex.contract_id.0, 0);

            let compressed_again_complex = complex_struct.compress(&mut register);
            assert_eq!(compressed_again_complex.asset_id.0, 0);
            assert_eq!(compressed_again_complex.contract_id.0, 0);

            let new_complex_struct = ComplexStruct {
                asset_id: [2; 32].into(),
                contract_id: [2; 32].into(),
                some_field: 3,
            };
            let compressed_new_complex = new_complex_struct.compress(&mut register);
            assert_eq!(compressed_new_complex.asset_id.0, 1);
            assert_eq!(compressed_new_complex.contract_id.0, 0);

            let complex_complex_struct_with_new = ComplexComplexStruct {
                complex: new_complex_struct,
                generic: ContractId::new([3; 32]),
                some_field: 3,
            };
            let compressed_complex_complex =
                complex_complex_struct_with_new.compress(&mut register);
            assert_eq!(compressed_complex_complex.complex.asset_id.0, 1);
            assert_eq!(compressed_complex_complex.complex.contract_id.0, 0);
            assert_eq!(compressed_complex_complex.generic.0, 1);
        }
    }
}
