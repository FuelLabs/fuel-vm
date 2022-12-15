#[derive(Debug, Eq, PartialEq)]
pub enum Bit {
    _0 = 0,
    _1 = 1,
}

trait GetBit {
    fn get_bit(&self, bit_index: usize) -> Option<Bit>;
}

impl GetBit for u8 {
    fn get_bit(&self, bit_index: usize) -> Option<Bit> {
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
    fn get_bit_at_index_from_msb(&self, index: usize) -> Option<Bit>;
    fn common_prefix_count(&self, other: &Self) -> usize;
}

impl<const N: usize> Msb for [u8; N] {
    fn get_bit_at_index_from_msb(&self, index: usize) -> Option<Bit> {
        // The byte that contains the bit
        let byte_index = index / 8;
        // The bit within the containing byte
        let byte_bit_index = index % 8;
        self.get(byte_index)
            .and_then(|byte| byte.get_bit(byte_bit_index))
    }

    fn common_prefix_count(&self, other: &Self) -> usize {
        let mut count = 0;
        for i in 0..(N * 8) {
            let lhs_bit = self.get_bit_at_index_from_msb(i).unwrap();
            let rhs_bit = other.get_bit_at_index_from_msb(i).unwrap();
            if lhs_bit == rhs_bit {
                count += 1;
            } else {
                break;
            }
        }
        count
    }
}

#[cfg(test)]
mod test {
    use crate::common::{Bytes1, Bytes2, Bytes4, Bytes8, Msb};
    use core::mem::size_of;

    #[test]
    fn test_msb_for_bytes_1() {
        const NUM_BITS: usize = size_of::<Bytes1>() * 8;

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
        const NUM_BITS: usize = size_of::<Bytes2>() * 8;

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
        const NUM_BITS: usize = size_of::<Bytes4>() * 8;

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
        const NUM_BITS: usize = size_of::<Bytes8>() * 8;

        let bytes: Bytes8 = [
            0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010,
            0b10101010,
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
        let lhs_bytes: Bytes4 = [0b10101010, 0b10101010, 0b10101010, 0b10101010];
        let rhs_bytes: Bytes4 = [0b10101010, 0b10101010, !0b10101010, 0b10101010];
        let common_prefix_count = lhs_bytes.common_prefix_count(&rhs_bytes);

        assert_eq!(common_prefix_count, 2 * 8);
    }

    #[test]
    fn test_common_prefix_count_returns_0_when_the_first_bits_are_different() {
        let lhs_bytes: Bytes4 = [0b10101010, 0b10101010, 0b10101010, 0b10101010];
        let rhs_bytes: Bytes4 = [0b00101010, 0b10101010, 0b10101010, 0b10101010];
        let common_prefix_count = lhs_bytes.common_prefix_count(&rhs_bytes);

        assert_eq!(common_prefix_count, 0);
    }
}
