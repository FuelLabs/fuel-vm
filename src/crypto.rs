use fuel_tx::crypto::Hasher;
use fuel_types::{Bytes32, Bytes64};
use secp256k1::recovery::{RecoverableSignature, RecoveryId};
use secp256k1::Error as Secp256k1Error;
use secp256k1::{Message, Secp256k1, SecretKey};

use std::convert::TryFrom;
use std::mem;

/// Sign a given message and compress the `v` to the signature
///
/// The compression scheme is described in
/// <https://github.com/lazyledger/lazyledger-specs/blob/master/specs/data_structures.md#public-key-cryptography>
pub fn secp256k1_sign_compact_recoverable(secret: &[u8], message: &[u8]) -> Result<Bytes64, Secp256k1Error> {
    let secret = SecretKey::from_slice(secret)?;
    let message = Message::from_slice(message)?;

    let signature = Secp256k1::new().sign_recoverable(&message, &secret);
    let (v, mut signature) = signature.serialize_compact();

    let v = v.to_i32();
    signature[32] |= (v << 7) as u8;

    Ok(signature.into())
}

/// Recover the public key from a signature performed with
/// [`secp256k1_sign_compact_recoverable`]
pub fn secp256k1_sign_compact_recover(signature: &[u8], message: &[u8]) -> Result<Bytes64, Secp256k1Error> {
    let message = Message::from_slice(message)?;
    let mut signature = Bytes64::try_from(signature).map_err(|_| Secp256k1Error::InvalidSignature)?;

    let v = ((signature.as_mut()[32] & 0x80) >> 7) as i32;
    signature.as_mut()[32] &= 0x7f;

    let v = RecoveryId::from_i32(v)?;
    let signature = RecoverableSignature::from_compact(signature.as_ref(), v)?;

    let pk = Secp256k1::new().recover(&message, &signature)?.serialize_uncompressed();

    // Ignore the first byte of the compressed flag
    let pk = &pk[1..];

    // Safety: secp256k1 protocol specifies 65 bytes output
    let pk = unsafe { Bytes64::from_slice_unchecked(pk) };

    Ok(pk)
}

/// Calculate a binary merkle root
///
/// The space complexity of this operation is O(n). This means it expects small
/// sets. For bigger sets (e.g. blockchain state), use a storage backed merkle
/// implementation
pub fn ephemeral_merkle_root<L, I>(mut leaves: I) -> Bytes32
where
    L: AsRef<[u8]>,
    I: Iterator<Item = L> + ExactSizeIterator,
{
    let mut hasher = Hasher::default();
    let mut width = leaves.len().next_power_of_two();
    let mut len = leaves.len() as f32;

    if width <= 2 {
        return leaves.collect::<Hasher>().digest();
    }

    width /= 2;
    len /= 2.0;

    let mut current = vec![Bytes32::default(); width];

    // Drain the leaves
    current.iter_mut().for_each(|l| {
        hasher.reset();

        // let a = leaves.next();
        // let b = leaves.next();

        for a in leaves.by_ref().take(2) {
            hasher.input(a);
        }

        *l = hasher.digest();
    });

    let mut next = current.clone();

    // Cheap loop with no realloc
    while width > 1 {
        mem::swap(&mut current, &mut next);

        let mut c = current.iter().take(len.ceil() as usize);

        width /= 2;
        len /= 2.0;
        next.iter_mut().take(width).for_each(|n| {
            hasher.reset();

            for a in c.by_ref().take(2) {
                hasher.input(a);
            }

            *n = hasher.digest();
        });
    }

    next[0]
}

#[cfg(all(test, feature = "random"))]
mod tests {
    use super::*;
    use crate::prelude::*;

    use fuel_tx::crypto::Hasher;
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore, SeedableRng};
    use secp256k1::PublicKey;

    use std::convert::TryFrom;

    #[test]
    fn ecrecover() {
        let secp = Secp256k1::new();
        let mut rng = StdRng::seed_from_u64(2322u64);
        let mut secret_seed = [0u8; 32];
        let mut message = [0u8; 95];

        for _ in 0..10 {
            rng.fill_bytes(&mut message);
            rng.fill_bytes(&mut secret_seed);

            let secret = SecretKey::from_slice(&secret_seed).expect("Failed to generate random secret!");
            let public = PublicKey::from_secret_key(&secp, &secret).serialize_uncompressed();
            let public = Bytes64::try_from(&public[1..]).expect("Failed to parse public key!");

            let e = Hasher::hash(&message);

            let sig =
                secp256k1_sign_compact_recoverable(secret.as_ref(), e.as_ref()).expect("Failed to generate signature");
            let pk_p =
                secp256k1_sign_compact_recover(sig.as_ref(), e.as_ref()).expect("Failed to recover PK from signature");

            assert_eq!(public, pk_p);
        }
    }

    #[test]
    fn ephemeral_merkle_root_works() {
        let mut rng = StdRng::seed_from_u64(2322u64);

        // Test for 0 leaves
        let empty: Vec<Address> = vec![];

        let root = ephemeral_merkle_root(empty.iter());
        let empty = Hasher::default().digest();

        assert_eq!(empty, root);

        // Test for 5 leaves
        let a: Address = rng.gen();
        let b: Address = rng.gen();
        let c: Address = rng.gen();
        let d: Address = rng.gen();
        let e: Address = rng.gen();

        let initial = [a, b, c, d, e];

        let a = [a, b].iter().collect::<Hasher>().digest();
        let b = [c, d].iter().collect::<Hasher>().digest();
        let c = [e].iter().collect::<Hasher>().digest();

        let a = [a, b].iter().collect::<Hasher>().digest();
        let b = [c].iter().collect::<Hasher>().digest();

        let root = [a, b].iter().collect::<Hasher>().digest();
        let root_p = ephemeral_merkle_root(initial.iter());

        assert_eq!(root, root_p);

        // Test for n leaves
        let mut inputs = vec![Address::default(); 64];

        inputs.iter_mut().for_each(|i| *i = rng.gen());

        (0..65).into_iter().for_each(|w| {
            let initial: Vec<&Address> = inputs.iter().take(w).collect();
            let mut level: Vec<Bytes32> = initial
                .chunks(2)
                .map(|c| c.iter().collect::<Hasher>().digest())
                .collect();

            while level.len() > 1 {
                level = level.chunks(2).map(|c| c.iter().collect::<Hasher>().digest()).collect();
            }

            let root = level.first().copied().unwrap_or(empty);
            let root_p = ephemeral_merkle_root(initial.iter());

            assert_eq!(root, root_p);
        });
    }
}
