#[derive(Debug, Eq, PartialEq)]
pub enum Bit {
    _0 = 0,
    _1 = 1,
}

trait GetBit {
    fn get_bit(&self, bit_index: u32) -> Option<Bit>;
}

impl GetBit for u8 {
    fn get_bit(&self, bit_index: u32) -> Option<Bit> {
        if bit_index < 8 {
            let mask = 1 << (7 - bit_index);
            let bit = self & mask;
            match bit {
                0 => Some(Bit::_0),
                _ => Some(Bit::_1),
            }
        } else {
            None
        }
    }
}

pub trait Msb {
    fn get_bit_at_index_from_msb(&self, index: u32) -> Option<Bit>;
    fn common_prefix_count(&self, other: &[u8]) -> u32;
}

impl<const N: usize> Msb for [u8; N] {
    fn get_bit_at_index_from_msb(&self, index: u32) -> Option<Bit> {
        // The byte that contains the bit
        let byte_index = index / 8;
        // The bit within the containing byte
        let byte_bit_index = index % 8;
        self.get(byte_index as usize)
            .and_then(|byte| byte.get_bit(byte_bit_index))
    }

    fn common_prefix_count(&self, other: &[u8]) -> u32 {
        let mut count = 0;
        for (byte1, byte2) in self.iter().zip(other.iter()) {
            // For each pair of bytes, compute the similarity of each byte using
            // exclusive or (XOR). The leading zeros measures the number of
            // similar bits from left to right. For equal bytes, this will be 8.
            count += (byte1 ^ byte2).leading_zeros();
            if byte1 != byte2 {
                break
            }
        }
        count
    }
}

#[allow(clippy::cast_possible_truncation)]
#[cfg(test)]
mod test {
    use crate::common::{
        Bytes1,
        Bytes2,
        Bytes4,
        Bytes8,
        Msb,
    };
    use core::mem::size_of;

    #[test]
    fn test_msb_for_bytes_1() {
        const NUM_BITS: u32 = size_of::<Bytes1>() as u32 * 8;

        let bytes: Bytes1 = [0b10101010];
        let expected_n = u8::from_be_bytes(bytes);

        let mut n = 0;
        for i in 0..NUM_BITS {
            let bit = bytes.get_bit_at_index_from_msb(i).unwrap() as u8;
            let shift = bit << (NUM_BITS - 1 - i);
            n |= shift;
        }

        assert_eq!(n, expected_n);
    }

    #[test]
    fn test_msb_for_bytes_2() {
        const NUM_BITS: u32 = size_of::<Bytes2>() as u32 * 8;

        let bytes: Bytes2 = [0b10101010, 0b10101010];
        let expected_n = u16::from_be_bytes(bytes);

        let mut n = 0;
        for i in 0..NUM_BITS {
            let bit = bytes.get_bit_at_index_from_msb(i).unwrap() as u16;
            let shift = bit << (NUM_BITS - 1 - i);
            n |= shift;
        }

        assert_eq!(n, expected_n);
    }

    #[test]
    fn test_msb_for_bytes_4() {
        const NUM_BITS: u32 = size_of::<Bytes4>() as u32 * 8;

        let bytes: Bytes4 = [0b10101010, 0b10101010, 0b10101010, 0b10101010];
        let expected_n = u32::from_be_bytes(bytes);

        let mut n = 0;
        for i in 0..NUM_BITS {
            let bit = bytes.get_bit_at_index_from_msb(i).unwrap() as u32;
            let shift = bit << (NUM_BITS - 1 - i);
            n |= shift;
        }

        assert_eq!(n, expected_n);
    }

    #[test]
    fn test_msb_for_bytes_8() {
        const NUM_BITS: u32 = size_of::<Bytes8>() as u32 * 8;

        let bytes: Bytes8 = [
            0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010,
            0b10101010, 0b10101010,
        ];
        let expected_n = u64::from_be_bytes(bytes);

        let mut n = 0;
        for i in 0..NUM_BITS {
            let bit = bytes.get_bit_at_index_from_msb(i).unwrap() as u64;
            let shift = bit << (NUM_BITS - 1 - i);
            n |= shift;
        }

        assert_eq!(n, expected_n);
    }

    #[test]
    fn test_get_bit_at_index_from_msb_returns_none_for_index_out_of_bounds() {
        let bytes: Bytes4 = [0b10101010, 0b10101010, 0b10101010, 0b10101010];

        // Returns None; acceptable inputs for Bytes4 are in [0, 31]
        let bit = bytes.get_bit_at_index_from_msb(32);
        assert_eq!(bit, None);
    }

    #[test]
    fn test_common_prefix_count_returns_count_of_common_bits_when_all_bits_match() {
        let lhs_bytes: Bytes4 = [0b10101010, 0b10101010, 0b10101010, 0b10101010];
        let rhs_bytes: Bytes4 = [0b10101010, 0b10101010, 0b10101010, 0b10101010];
        let common_prefix_count = lhs_bytes.common_prefix_count(&rhs_bytes);

        assert_eq!(common_prefix_count, 4 * 8);
    }

    #[test]
    fn test_common_prefix_count_returns_count_of_common_bits_when_some_bits_match() {
        let lhs_bytes: Bytes4 = [0b11111111, 0b11111111, 0b11111111, 0b11111111];
        let rhs_bytes: Bytes4 = [0b11111111, 0b11111111, 0b11100000, 0b00000000];
        let common_prefix_count = lhs_bytes.common_prefix_count(&rhs_bytes);

        assert_eq!(common_prefix_count, 19);
    }

    #[test]
    fn test_common_prefix_count_returns_0_when_the_first_bits_are_different() {
        let lhs_bytes: Bytes4 = [0b11111111, 0b11111111, 0b11111111, 0b11111111];
        let rhs_bytes: Bytes4 = [0b01111111, 0b11111111, 0b11111111, 0b11111111];
        let common_prefix_count = lhs_bytes.common_prefix_count(&rhs_bytes);

        assert_eq!(common_prefix_count, 0);
    }

    #[test]
    fn test_common_prefix_count_returns_0_when_all_bits_are_different() {
        let lhs_bytes: Bytes4 = [0b11111111, 0b11111111, 0b11111111, 0b11111111];
        let rhs_bytes: Bytes4 = [0b00000000, 0b00000000, 0b00000000, 0b00000000];
        let common_prefix_count = lhs_bytes.common_prefix_count(&rhs_bytes);

        assert_eq!(common_prefix_count, 0);
    }
}
