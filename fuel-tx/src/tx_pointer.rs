use fuel_types::{
    bytes::WORD_SIZE,
    BlockHeight,
};

use fuel_types::canonical::{
    Deserialize,
    Serialize,
};

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
#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[derive(Deserialize, Serialize)]
pub struct TxPointer {
    /// Block height
    block_height: BlockHeight,
    /// Transaction index
    #[cfg(feature = "u32-tx-pointer")]
    tx_index: u32,
    #[cfg(not(feature = "u32-tx-pointer"))]
    tx_index: u16,
}

impl TxPointer {
    pub const LEN: usize = 2 * WORD_SIZE;

    pub const fn new(
        block_height: BlockHeight,
        #[cfg(feature = "u32-tx-pointer")] tx_index: u32,
        #[cfg(not(feature = "u32-tx-pointer"))] tx_index: u16,
    ) -> Self {
        Self {
            block_height,
            tx_index,
        }
    }

    pub const fn block_height(&self) -> BlockHeight {
        self.block_height
    }

    #[cfg(feature = "u32-tx-pointer")]
    pub const fn tx_index(&self) -> u32 {
        self.tx_index
    }

    #[cfg(not(feature = "u32-tx-pointer"))]
    pub const fn tx_index(&self) -> u16 {
        self.tx_index
    }
}

#[cfg(feature = "random")]
impl Distribution<TxPointer> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TxPointer {
        TxPointer::new(rng.gen(), rng.gen())
    }
}

impl fmt::Display for TxPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

#[cfg(feature = "u32-tx-pointer")]
impl fmt::LowerHex for TxPointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:08x}{:08x}", self.block_height, self.tx_index)
    }
}

#[cfg(not(feature = "u32-tx-pointer"))]
impl fmt::LowerHex for TxPointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:08x}{:04x}", self.block_height, self.tx_index)
    }
}

#[cfg(feature = "u32-tx-pointer")]
impl fmt::UpperHex for TxPointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:08X}{:08X}", self.block_height, self.tx_index)
    }
}

#[cfg(not(feature = "u32-tx-pointer"))]
impl fmt::UpperHex for TxPointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:08X}{:04X}", self.block_height, self.tx_index)
    }
}

impl str::FromStr for TxPointer {
    type Err = &'static str;

    #[cfg(feature = "u32-tx-pointer")]
    /// TxPointer is encoded as 16 hex characters:
    /// - 8 characters for block height
    /// - 8 characters for tx index
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "Invalid encoded byte in TxPointer";

        if s.len() != 16 || !s.is_char_boundary(8) {
            return Err(ERR)
        }

        let (block_height, tx_index) = s.split_at(8);

        let block_height = u32::from_str_radix(block_height, 16).map_err(|_| ERR)?;
        let tx_index = u32::from_str_radix(tx_index, 16).map_err(|_| ERR)?;

        Ok(Self::new(block_height.into(), tx_index))
    }

    #[cfg(not(feature = "u32-tx-pointer"))]
    /// TxPointer is encoded as 12 hex characters:
    /// - 8 characters for block height
    /// - 4 characters for tx index
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "Invalid encoded byte in TxPointer";

        if s.len() != 12 || !s.is_char_boundary(8) {
            return Err(ERR)
        }

        let (block_height, tx_index) = s.split_at(8);

        let block_height = u32::from_str_radix(block_height, 16).map_err(|_| ERR)?;
        let tx_index = u16::from_str_radix(tx_index, 16).map_err(|_| ERR)?;

        Ok(Self::new(block_height.into(), tx_index))
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
    impl TxPointer {
        #[wasm_bindgen(constructor)]
        pub fn typescript_new(value: &str) -> Result<TxPointer, js_sys::Error> {
            use core::str::FromStr;
            TxPointer::from_str(value).map_err(js_sys::Error::new)
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
        pub fn typescript_from_bytes(value: &[u8]) -> Result<TxPointer, js_sys::Error> {
            use fuel_types::canonical::Deserialize;
            <Self as Deserialize>::from_bytes(value)
                .map_err(|e| js_sys::Error::new(&format!("{:?}", e)))
        }
    }
}

#[cfg(not(feature = "u32-tx-pointer"))]
#[test]
fn fmt_encode_decode() {
    use core::str::FromStr;

    let cases = vec![(83473, 3829)];

    for (block_height, tx_index) in cases {
        let tx_pointer = TxPointer::new(block_height.into(), tx_index);

        let lower = format!("{tx_pointer:x}");
        let upper = format!("{tx_pointer:X}");

        assert_eq!(lower, format!("{block_height:08x}{tx_index:04x}"));
        assert_eq!(upper, format!("{block_height:08X}{tx_index:04X}"));

        let x = TxPointer::from_str(&lower).expect("failed to decode from str");
        assert_eq!(tx_pointer, x);

        let x = TxPointer::from_str(&upper).expect("failed to decode from str");
        assert_eq!(tx_pointer, x);

        let bytes = tx_pointer.clone().to_bytes();
        let tx_pointer_p = TxPointer::from_bytes(&bytes).expect("failed to deserialize");

        assert_eq!(tx_pointer, tx_pointer_p);
    }
}

#[cfg(feature = "u32-tx-pointer")]
#[test]
fn fmt_encode_decode_u32() {
    use core::str::FromStr;

    let cases = vec![(83473, 3829)];

    for (block_height, tx_index) in cases {
        let tx_pointer = TxPointer::new(block_height.into(), tx_index);

        let lower = format!("{tx_pointer:x}");
        let upper = format!("{tx_pointer:X}");

        assert_eq!(lower, format!("{block_height:08x}{tx_index:08x}"));
        assert_eq!(upper, format!("{block_height:08X}{tx_index:08X}"));

        let x = TxPointer::from_str(&lower).expect("failed to decode from str");
        assert_eq!(tx_pointer, x);

        let x = TxPointer::from_str(&upper).expect("failed to decode from str");
        assert_eq!(tx_pointer, x);

        let bytes = tx_pointer.clone().to_bytes();
        let tx_pointer_p = TxPointer::from_bytes(&bytes).expect("failed to deserialize");

        assert_eq!(tx_pointer, tx_pointer_p);
    }
}

/// See https://github.com/FuelLabs/fuel-vm/issues/521
#[test]
fn decode_bug() {
    use core::str::FromStr;
    TxPointer::from_str("00000😎000").expect_err("Should fail on incorrect input");
}
