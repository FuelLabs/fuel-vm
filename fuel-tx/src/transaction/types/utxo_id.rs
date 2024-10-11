use crate::TxId;

use fuel_types::Bytes32;
use postcard_bindgen::PostcardBindings;
use core::{
    fmt,
    str,
};

#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

/// Identification of unspend transaction output.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(
    serde::Serialize,
    serde::Deserialize,
    PostcardBindings,
    fuel_types::canonical::Deserialize,
    fuel_types::canonical::Serialize,
)]
pub struct UtxoId {
    /// transaction id
    tx_id: TxId,
    /// output index
    output_index: u16,
}

#[cfg(feature = "da-compression")]
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CompressedUtxoId {
    pub tx_pointer: crate::TxPointer,
    pub output_index: u16,
}

#[cfg(feature = "da-compression")]
impl fuel_compression::Compressible for UtxoId {
    type Compressed = CompressedUtxoId;
}

impl UtxoId {
    pub const LEN: usize = TxId::LEN + 8;

    pub const fn new(tx_id: TxId, output_index: u16) -> Self {
        Self {
            tx_id,
            output_index,
        }
    }

    pub const fn tx_id(&self) -> &TxId {
        &self.tx_id
    }

    pub const fn output_index(&self) -> u16 {
        self.output_index
    }

    pub fn replace_tx_id(&mut self, tx_id: TxId) {
        self.tx_id = tx_id;
    }
}

#[cfg(feature = "random")]
impl Distribution<UtxoId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UtxoId {
        let mut tx_id = Bytes32::default();
        rng.fill_bytes(tx_id.as_mut());
        UtxoId::new(tx_id, rng.gen())
    }
}

impl fmt::LowerHex for UtxoId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "{:#x}{:04x}", self.tx_id, self.output_index)
        } else {
            write!(f, "{:x}{:04x}", self.tx_id, self.output_index)
        }
    }
}

impl fmt::UpperHex for UtxoId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "{:#X}{:04X}", self.tx_id, self.output_index)
        } else {
            write!(f, "{:X}{:04X}", self.tx_id, self.output_index)
        }
    }
}

impl core::fmt::Display for UtxoId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "{:#x}{:04x}", self.tx_id, self.output_index)
        } else {
            write!(f, "{:x}{:04x}", self.tx_id, self.output_index)
        }
    }
}

impl str::FromStr for UtxoId {
    type Err = &'static str;

    /// UtxoId is encoded as hex string with optional 0x prefix, where
    /// the last two characters are the output index and the part
    /// optionally preceeding it is the transaction id.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "Invalid encoded byte in UtxoId";
        let s = s.strip_prefix("0x").unwrap_or(s);

        Ok(if s.is_empty() {
            UtxoId::new(Bytes32::default(), 0)
        } else if s.len() <= 4 {
            UtxoId::new(
                TxId::default(),
                u16::from_str_radix(s, 16).map_err(|_| ERR)?,
            )
        } else {
            #[allow(clippy::arithmetic_side_effects)] // Checked above
            let i = s.len() - 4;
            if !s.is_char_boundary(i) {
                return Err(ERR)
            }
            let (tx_id, output_index) = s.split_at(i);
            let tx_id = tx_id.strip_suffix(':').unwrap_or(tx_id);

            UtxoId::new(
                Bytes32::from_str(tx_id)?,
                u16::from_str_radix(output_index, 16).map_err(|_| ERR)?,
            )
        })
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use super::*;

    use wasm_bindgen::prelude::*;

    use alloc::{
        format,
        string::String,
        vec::Vec,
    };

    #[wasm_bindgen]
    impl UtxoId {
        #[wasm_bindgen(constructor)]
        pub fn typescript_new(value: &str) -> Result<UtxoId, js_sys::Error> {
            use core::str::FromStr;
            UtxoId::from_str(value).map_err(js_sys::Error::new)
        }

        #[wasm_bindgen(js_name = toString)]
        pub fn typescript_to_string(&self) -> String {
            format!("{:#x}", self)
        }

        #[wasm_bindgen(js_name = to_bytes)]
        pub fn typescript_to_bytes(&self) -> Vec<u8> {
            use fuel_types::canonical::Serialize;
            <Self as Serialize>::to_bytes(self)
        }

        #[wasm_bindgen(js_name = from_bytes)]
        pub fn typescript_from_bytes(value: &[u8]) -> Result<UtxoId, js_sys::Error> {
            use fuel_types::canonical::Deserialize;

            <Self as Deserialize>::from_bytes(value)
                .map_err(|e| js_sys::Error::new(&format!("{:?}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;
    use fuel_types::Bytes32;

    #[test]
    fn fmt_utxo_id_with_one_bytes_output_index() {
        let mut tx_id = Bytes32::zeroed();
        *tx_id.get_mut(0).unwrap() = 12;
        *tx_id.get_mut(31).unwrap() = 11;

        let utxo_id = UtxoId {
            tx_id,
            output_index: 0xab,
        };
        assert_eq!(
            format!("{utxo_id:#x}"),
            "0x0c0000000000000000000000000000000000000000000000000000000000000b00ab"
        );
        assert_eq!(
            format!("{utxo_id:x}"),
            "0c0000000000000000000000000000000000000000000000000000000000000b00ab"
        );
    }

    #[test]
    fn fmt_utxo_id_with_two_bytes_output_index() {
        let mut tx_id = Bytes32::zeroed();
        *tx_id.get_mut(0).unwrap() = 12;
        *tx_id.get_mut(31).unwrap() = 11;

        let utxo_id = UtxoId {
            tx_id,
            output_index: 0xabcd,
        };
        assert_eq!(
            format!("{utxo_id:#x}"),
            "0x0c0000000000000000000000000000000000000000000000000000000000000babcd"
        );
        assert_eq!(
            format!("{utxo_id:x}"),
            "0c0000000000000000000000000000000000000000000000000000000000000babcd"
        );
    }

    #[test]
    fn from_str_utxo_id() -> Result<(), &'static str> {
        let utxo_id = UtxoId::from_str(
            "0x0c0000000000000000000000000000000000000000000000000000000000000babcd",
        )?;

        assert_eq!(utxo_id.output_index, 0xabcd);
        assert_eq!(utxo_id.tx_id[31], 11);
        assert_eq!(utxo_id.tx_id[0], 12);
        Ok(())
    }

    #[test]
    fn from_str_utxo_id_colon_separator() -> Result<(), &'static str> {
        let utxo_id = UtxoId::from_str(
            "0c0000000000000000000000000000000000000000000000000000000000000b:abcd",
        )?;

        assert_eq!(utxo_id.output_index, 0xabcd);
        assert_eq!(utxo_id.tx_id[31], 11);
        assert_eq!(utxo_id.tx_id[0], 12);
        Ok(())
    }

    /// See https://github.com/FuelLabs/fuel-vm/issues/521
    #[test]
    fn from_str_utxo_id_multibyte_bug() {
        UtxoId::from_str("0x00😎").expect_err("Should fail on incorrect input");
        UtxoId::from_str("0x000😎").expect_err("Should fail on incorrect input");
    }
}
