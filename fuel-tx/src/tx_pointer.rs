use fuel_types::{
    bytes::{SizedBytes, WORD_SIZE},
    mem_layout, BlockHeight, MemLayout, MemLocType,
};

use core::{fmt, str};

#[cfg(feature = "std")]
use fuel_types::bytes;

#[cfg(feature = "std")]
use std::io;

#[cfg(feature = "random")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

/// Identification of unspend transaction output.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxPointer {
    /// Block height
    block_height: BlockHeight,
    /// Transaction index
    tx_index: u16,
}

mem_layout!(
    TxPointerLayout for TxPointer
    block_height: u32 = WORD_SIZE,
    tx_index: u16 = WORD_SIZE
);

impl TxPointer {
    pub const LEN: usize = 2 * WORD_SIZE;

    pub const fn new(block_height: BlockHeight, tx_index: u16) -> Self {
        Self { block_height, tx_index }
    }

    pub const fn block_height(&self) -> BlockHeight {
        self.block_height
    }

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

impl fmt::LowerHex for TxPointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:08x}{:04x}", self.block_height, self.tx_index)
    }
}

impl fmt::UpperHex for TxPointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:08X}{:04X}", self.block_height, self.tx_index)
    }
}

impl str::FromStr for TxPointer {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "Invalid encoded byte";

        if s.len() != 12 {
            return Err(ERR);
        }

        let block_height = u32::from_str_radix(&s[..8], 16).map_err(|_| ERR)?;
        let tx_index = u16::from_str_radix(&s[8..12], 16).map_err(|_| ERR)?;

        Ok(Self::new(block_height.into(), tx_index))
    }
}

impl SizedBytes for TxPointer {
    fn serialized_size(&self) -> usize {
        Self::LEN
    }
}

#[cfg(feature = "std")]
impl io::Write for TxPointer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf: &[_; Self::LEN] = buf
            .get(..Self::LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        let block_height = bytes::restore_u32_at(buf, Self::layout(Self::LAYOUT.block_height)).into();
        let tx_index = bytes::restore_u16_at(buf, Self::layout(Self::LAYOUT.tx_index));

        self.block_height = block_height;
        self.tx_index = tx_index;

        Ok(Self::LEN)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "std")]
impl io::Read for TxPointer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let buf: &mut [_; Self::LEN] = buf
            .get_mut(..Self::LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.block_height), *self.block_height);
        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.tx_index), self.tx_index);

        Ok(Self::LEN)
    }
}

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

        #[cfg(feature = "std")]
        {
            use fuel_types::bytes::{Deserializable, SerializableVec};

            let bytes = tx_pointer.clone().to_bytes();
            let tx_pointer_p = TxPointer::from_bytes(&bytes).expect("failed to deserialize");

            assert_eq!(tx_pointer, tx_pointer_p);
        }
    }
}
