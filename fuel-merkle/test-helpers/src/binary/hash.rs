use digest::Digest;
use sha2::Sha256 as Hash;

pub type Data = [u8; 32];

const NODE: u8 = 0x01;
const LEAF: u8 = 0x00;

// Merkle Tree hash of an empty list
// MTH({}) = Hash()
pub fn empty_sum() -> &'static Data {
    const EMPTY_SUM: Data = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99,
        0x6f, 0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95,
        0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
    ];

    &EMPTY_SUM
}

// Merkle tree hash of an n-element list D[n]
// MTH(D[n]) = Hash(0x01 || MTH(D[0:k]) || MTH(D[k:n])
pub fn node_sum(lhs_data: &[u8], rhs_data: &[u8]) -> Data {
    let mut hash = Hash::new();
    hash.update([NODE]);
    hash.update(lhs_data);
    hash.update(rhs_data);
    hash.finalize().into()
}

// Merkle tree hash of a list with one entry
// MTH({d(0)}) = Hash(0x00 || d(0))
pub fn leaf_sum(data: &[u8]) -> Data {
    let mut hash = Hash::new();
    hash.update([LEAF]);
    hash.update(data);
    hash.finalize().into()
}
