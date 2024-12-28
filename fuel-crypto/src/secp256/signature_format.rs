//! Utility functions common for secp256k1 and secp256r1

/// Recovery id, used to encode the y parity of the public key
/// in the signature.
///
/// Guaranteed to be valid by construction. Only encodes the y parity,
/// and rejects reduced-x recovery ids.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RecoveryId {
    is_y_odd: bool,
}

impl From<RecoveryId> for k256::ecdsa::RecoveryId {
    fn from(recid: RecoveryId) -> Self {
        k256::ecdsa::RecoveryId::new(recid.is_y_odd, false)
    }
}
impl TryFrom<ecdsa::RecoveryId> for RecoveryId {
    type Error = ();

    fn try_from(recid: ecdsa::RecoveryId) -> Result<Self, Self::Error> {
        if recid.is_x_reduced() {
            return Err(())
        }

        Ok(Self {
            is_y_odd: recid.is_y_odd(),
        })
    }
}

#[cfg(feature = "std")]
impl From<RecoveryId> for secp256k1::ecdsa::RecoveryId {
    fn from(recid: RecoveryId) -> Self {
        secp256k1::ecdsa::RecoveryId::try_from(recid.is_y_odd as i32)
            .expect("0 and 1 are always valid recovery ids")
    }
}

#[cfg(feature = "std")]
impl TryFrom<secp256k1::ecdsa::RecoveryId> for RecoveryId {
    type Error = ();

    fn try_from(recid: secp256k1::ecdsa::RecoveryId) -> Result<Self, Self::Error> {
        let id: i32 = recid.into();
        match id {
            0 => Ok(Self { is_y_odd: false }),
            1 => Ok(Self { is_y_odd: true }),
            _ => Err(()),
        }
    }
}

/// Combines recovery id with the signature bytes. See the following link for explanation.
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md#ecdsa-public-key-cryptography
/// Panics if the highest bit of byte at index 32 is set, as this indicates non-normalized
/// signature. Panics if the recovery id is in reduced-x form.
pub fn encode_signature(mut signature: [u8; 64], recovery_id: RecoveryId) -> [u8; 64] {
    assert!(signature[32] >> 7 == 0, "Non-normalized signature");
    let v = recovery_id.is_y_odd as u8;

    signature[32] = (v << 7) | (signature[32] & 0x7f);
    signature
}

/// Separates recovery id from the signature bytes. See the following link for
/// explanation. https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md#ecdsa-public-key-cryptography
pub fn decode_signature(mut signature: [u8; 64]) -> ([u8; 64], RecoveryId) {
    let is_y_odd = (signature[32] & 0x80) != 0;
    signature[32] &= 0x7f;
    (signature, RecoveryId { is_y_odd })
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use rand::{
        rngs::StdRng,
        SeedableRng,
    };

    use crate::{
        Message,
        SecretKey,
        Signature,
    };

    use super::*;

    #[test]
    fn signature_roundtrip() {
        let rng = &mut StdRng::seed_from_u64(1234);

        let message = Message::new("Hello, world!");
        let secret = SecretKey::random(rng);
        let signature = Signature::sign(&secret, &message);

        let (decoded, recovery_id) = decode_signature(*signature);
        let encoded = encode_signature(decoded, recovery_id);

        assert_eq!(*signature, encoded);
    }
}
