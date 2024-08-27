use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    Key,
    RawKey,
};

mod _private {
    pub trait Seal {}
}

/// Static name of a table
pub type TableName = &'static str;

/// Table in the registry
pub trait Table: _private::Seal {
    /// Unique name of the table
    const NAME: TableName;

    /// A `CountPerTable` for this table
    fn count(n: usize) -> CountPerTable;

    /// The type stored in the table
    type Type: PartialEq + Default + Serialize + for<'de> Deserialize<'de>;
}

/// Traits for accessing `*PerTable` using the table type
pub mod access {
    /// Copy value for the give table
    pub trait AccessCopy<T, V: Copy> {
        /// Copy value for the give table
        fn value(&self) -> V;
    }

    /// Get reference to the value for the given table
    pub trait AccessRef<T, V> {
        /// Get reference to the value for the given table
        fn get(&self) -> &V;
    }

    /// Get mutable reference to the value for the given table
    pub trait AccessMut<T, V> {
        /// Get mutable reference to the value for the given table
        fn get_mut(&mut self) -> &mut V;
    }
}

macro_rules! tables {
    ($($name:ident: $ty:ty),*$(,)?) => { paste::paste! {
        /// Marker struct for each table type
        pub mod tables {
            $(
                /// Specifies the table to use for a given key.
                /// The data is separated to tables based on the data type being stored.
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
                pub struct $name;

                impl super::_private::Seal for $name {}
                impl super::Table for $name {
                    const NAME: &'static str = stringify!($name);
                    fn count(n: usize) -> super::CountPerTable {
                        super::CountPerTable::$name(n)
                    }
                    type Type = $ty;
                }

                impl $name {
                    /// Calls the `to_key_*` method for this table on the context
                    pub fn to_key(value: $ty, ctx: &mut dyn super::CompactionContext) -> anyhow::Result<super::Key<$name>> {
                        ctx.[<to_key_ $name>](value)
                    }

                    /// Calls the `read_*` method for this table on the context
                    pub fn read(key: super::Key<$name>, ctx: &dyn super::DecompactionContext) -> anyhow::Result<$ty> {
                        ctx.[<read_ $name>](key)
                    }
                }
            )*
        }

        /// Context for compaction, i.e. converting data to reference-based format.
        /// The context is used to aggreage changes to the registry.
        /// A new context should be created for each compaction "session",
        /// typically a blockchain block.
        #[allow(non_snake_case)] // The `to_key_*` names match table type names eactly
        pub trait CompactionContext {
            $(
                /// Store a value to the changeset and return a short reference key to it.
                /// If the value already exists in the registry and will not be overwritten,
                /// the existing key can be returned instead.
                fn [<to_key_  $name>](&mut self, value: $ty) -> anyhow::Result<Key<tables::$name>>;
            )*

            /// Convert transaction id to a transaction pointer.
            fn to_tx_pointer(&mut self, tx_id: [u8; 32]) -> anyhow::Result<[u8; 6]>;
        }

        /// Context for compaction, i.e. converting data to reference-based format
        #[allow(non_snake_case)] // The `to_key_*` names match table type names eactly
        pub trait DecompactionContext {
            $(
                /// Read a value from the registry based on the key.
                fn [<read_  $name>](&self, key: Key<tables::$name>) -> anyhow::Result<<tables::$name as Table>::Type>;
            )*

            /// Lookup transaction id to a transaction pointer.
            fn lookup_tx_pointer(&self, tx_pointer: [u8; 6]) -> anyhow::Result<[u8; 32]>;
        }

        /// One counter per table
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
        #[allow(non_snake_case)] // The field names match table type names eactly
        #[allow(missing_docs)] // Makes no sense to document the fields
        #[non_exhaustive]
        pub struct CountPerTable {
            $(pub $name: usize),*
        }

        impl CountPerTable {$(
            /// Custom constructor per table
            #[allow(non_snake_case)] // The field names match table type names eactly
            pub fn $name(value: usize) -> Self {
                Self {
                    $name: value,
                    ..Self::default()
                }
            }
        )*}

        $(
            impl access::AccessCopy<tables::$name, usize> for CountPerTable {
                fn value(&self) -> usize {
                    self.$name
                }
            }
        )*

        impl core::ops::Add<CountPerTable> for CountPerTable {
            type Output = Self;

            fn add(self, rhs: CountPerTable) -> Self::Output {
                Self {
                    $($name: self.$name + rhs.$name),*
                }
            }
        }

        impl core::ops::AddAssign<CountPerTable> for CountPerTable {
            fn add_assign(&mut self, rhs: CountPerTable) {
                $(self.$name += rhs.$name);*
            }
        }

        /// One key value per table
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[allow(non_snake_case)] // The field names match table type names eactly
        #[allow(missing_docs)] // Makes no sense to document the fields
        #[non_exhaustive]
        pub struct KeyPerTable {
            $(pub $name: Key<tables::$name>),*
        }

        impl Default for KeyPerTable {
            fn default() -> Self {
                Self {
                    $($name: Key::ZERO,)*
                }
            }
        }

        impl KeyPerTable {
            /// Generate keys for each table using a function that's called for each table.
            /// Since type erasure is required, the function takes `TableName` adn returns `RawKey`.
            pub fn from_fn<F: Fn(TableName) -> RawKey>(f: F) -> Self {
                Self {
                    $($name: Key::from_raw(f(<tables::$name as Table>::NAME)),)*
                }
            }
        }

        $(
            impl access::AccessCopy<tables::$name, Key<tables::$name>> for KeyPerTable {
                fn value(&self) -> Key<tables::$name> {
                    self.$name
                }
            }
            impl access::AccessRef<tables::$name, Key<tables::$name>> for KeyPerTable {
                fn get(&self) -> &Key<tables::$name> {
                    &self.$name
                }
            }
            impl access::AccessMut<tables::$name, Key<tables::$name>> for KeyPerTable {
                fn get_mut(&mut self) -> &mut Key<tables::$name> {
                    &mut self.$name
                }
            }
        )*

        impl KeyPerTable {
            /// Used to add together keys and counts to deterimine possible overwrite range.
            /// Panics if the keys count cannot fit into `u32`.
            pub fn offset_by(&self, counts: CountPerTable) -> KeyPerTable {
                KeyPerTable {
                    $(
                        $name: self.$name.add_u32(counts.$name.try_into()
                            .expect("Count too large. Shouldn't happen as we control inputs here.")
                        ),
                    )*
                }
            }
        }
    }};
}

tables!(
    AssetId: [u8; 32],
    Address: [u8; 32],
    ContractId: [u8; 32],
    ScriptCode: Vec<u8>,
    Witness: Vec<u8>,
);
