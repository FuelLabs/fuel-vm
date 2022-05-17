use crate::TxId;

use fuel_types::Bytes32;

use core::str::FromStr;

#[cfg(feature = "random")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

/// Identification of unspend transaction output.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UtxoId {
    /// transaction id
    tx_id: TxId,
    /// output index
    output_index: u8,
}

impl UtxoId {
    pub fn new(tx_id: TxId, output_index: u8) -> Self {
        Self {
            tx_id,
            output_index,
        }
    }

    pub fn tx_id(&self) -> &TxId {
        &self.tx_id
    }

    pub fn output_index(&self) -> u8 {
        self.output_index
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

impl core::fmt::LowerHex for UtxoId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "{:#x}{:02x}", self.tx_id, self.output_index)
        } else {
            write!(f, "{:x}{:02x}", self.tx_id, self.output_index)
        }
    }
}

impl core::fmt::UpperHex for UtxoId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "{:#X}{:02X}", self.tx_id, self.output_index)
        } else {
            write!(f, "{:X}{:02X}", self.tx_id, self.output_index)
        }
    }
}

impl FromStr for UtxoId {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "Invalid encoded byte";
        let s = s.trim_start_matches("0x");
        let utxo_id = if s.is_empty() {
            UtxoId::new(Bytes32::default(), 0)
        } else if s.len() > 2 {
            UtxoId::new(
                Bytes32::from_str(&s[..s.len() - 2])?,
                u8::from_str_radix(&s[s.len() - 2..], 16).map_err(|_| ERR)?,
            )
        } else {
            UtxoId::new(TxId::default(), u8::from_str_radix(s, 16).map_err(|_| ERR)?)
        };
        Ok(utxo_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuel_types::Bytes32;

    #[test]
    fn fmt_utxo_id() {
        let mut tx_id = Bytes32::zeroed();
        *tx_id.get_mut(0).unwrap() = 12;
        *tx_id.get_mut(31).unwrap() = 11;

        let utxo_id = UtxoId {
            tx_id,
            output_index: 26,
        };
        assert_eq!(
            format!("{:#x}", utxo_id),
            "0x0c0000000000000000000000000000000000000000000000000000000000000b1a"
        );
        assert_eq!(
            format!("{:x}", utxo_id),
            "0c0000000000000000000000000000000000000000000000000000000000000b1a"
        );
    }

    #[test]
    fn from_str_utxo_id() -> Result<(), &'static str> {
        let utxo_id = UtxoId::from_str(
            "0x0c0000000000000000000000000000000000000000000000000000000000000b1a",
        )?;

        assert_eq!(utxo_id.output_index, 26);
        assert_eq!(utxo_id.tx_id[31], 11);
        assert_eq!(utxo_id.tx_id[0], 12);
        Ok(())
    }
}
