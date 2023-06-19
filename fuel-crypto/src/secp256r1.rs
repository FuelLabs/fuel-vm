//! secp256r1 (P-256) functions

use crate::{
    Error,
    Message,
};
use coins_bip32::prelude::RecoveryId;
use fuel_types::Bytes64;
use p256::ecdsa::{
    Signature,
    SigningKey,
    VerifyingKey,
};

/// Combines recovery id with the signature bytes. See the following link for explanation.
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic_primitives.md#public-key-cryptography
fn encode_signature(signature: Signature, recovery_id: RecoveryId) -> [u8; 64] {
    let mut signature: [u8; 64] = signature.to_bytes().into();
    debug_assert!(signature[32] >> 7 == 0);
    assert!(!recovery_id.is_x_reduced());

    let v = recovery_id.is_y_odd() as u8;

    signature[32] = (v << 7) | (signature[32] & 0x7f);
    signature
}

/// Separates recovery id from the signature bytes. See the following link for
/// explanation. https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic_primitives.md#public-key-cryptography
fn decode_signature(mut signature: [u8; 64]) -> Option<(Signature, RecoveryId)> {
    let v = (signature[32] & 0x80) != 0;
    signature[32] &= 0x7f;

    let signature = Signature::from_slice(&signature).ok()?;

    Some((signature, RecoveryId::new(v, false)))
}

/// Sign a prehashed message. With the given key.
pub fn sign_prehashed(
    signing_key: &SigningKey,
    message: &Message,
) -> Result<Bytes64, Error> {
    let (signature, _) = signing_key
        .sign_prehash_recoverable(&**message)
        .map_err(|_| Error::FailedToSign)?;

    let signature = signature.normalize_s().unwrap_or(signature);

    // TODO: this is a hack to get the recovery id. The signature should be normalized
    // before computing the recovery id, but p256 library doesn't support this, and
    // instead always computes the recovery id from non-normalized signature.
    // So instead the recovery id is determined by checking which variant matches
    // the original public key.

    let recid1 = RecoveryId::new(false, false);
    let recid2 = RecoveryId::new(true, false);

    let rec1 = VerifyingKey::recover_from_prehash(&**message, &signature, recid1);
    let rec2 = VerifyingKey::recover_from_prehash(&**message, &signature, recid2);

    let actual = signing_key.verifying_key();

    let recovery_id = if rec1.map(|r| r == *actual).unwrap_or(false) {
        recid1
    } else if rec2.map(|r| r == *actual).unwrap_or(false) {
        recid2
    } else {
        unreachable!("Invalid signature generated");
    };

    Ok(Bytes64::from(encode_signature(signature, recovery_id)))
}

/// Convert the publi key point to it's uncompressed non-prefixed representation,
/// i.e. 32 bytes of x coordinate and 32 bytes of y coordinate.
pub fn encode_pubkey(key: VerifyingKey) -> [u8; 64] {
    let point = key.to_encoded_point(false);
    let mut result = [0u8; 64];
    result[..32].copy_from_slice(point.x().unwrap());
    result[32..].copy_from_slice(point.y().unwrap());
    result
}

/// Recover a public key from a signature and a message digest. It assumes
/// a compacted signature
pub fn recover(signature: &Bytes64, message: &Message) -> Result<Bytes64, Error> {
    let (signature, recovery_id) =
        decode_signature(**signature).ok_or(Error::InvalidSignature)?;

    if let Ok(pub_key) =
        VerifyingKey::recover_from_prehash(&**message, &signature, recovery_id)
    {
        Ok(Bytes64::from(encode_pubkey(pub_key)))
    } else {
        Err(Error::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use p256::ecdsa::SigningKey;
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    #[test]
    fn test_raw_recover() {
        let mut rng = &mut StdRng::seed_from_u64(1234);

        let signing_key = SigningKey::random(&mut rng);
        let verifying_key = signing_key.verifying_key();

        let message = Message::new([rng.gen(); 100]);

        let (signature, recovery_id) =
            signing_key.sign_prehash_recoverable(&*message).unwrap();

        let recovered =
            VerifyingKey::recover_from_prehash(&*message, &signature, recovery_id)
                .expect("Unable to recover the public key");

        assert_eq!(recovered, *verifying_key);
    }

    #[test]
    fn test_secp256r1_recover_from_msg() {
        let mut rng = &mut StdRng::seed_from_u64(1234);

        for _ in 0..100 {
            let signing_key = SigningKey::random(&mut rng);
            let verifying_key = signing_key.verifying_key();

            let message = Message::new([rng.gen(); 100]);
            let signature = crate::secp256r1::sign_prehashed(&signing_key, &message)
                .expect("Couldn't sign");

            let Ok(recovered) = recover(&signature, &message) else {
                panic!("Failed to recover public key from the message");
            };

            assert_eq!(*recovered, encode_pubkey(*verifying_key));
        }
    }

    #[test]
    fn test_signature_and_recovery_id_encoding_roundtrip() {
        let mut rng = &mut StdRng::seed_from_u64(1234);

        for _ in 0..100 {
            let message = Message::new([rng.gen(); 100]);
            let signing_key = SigningKey::random(&mut rng);
            let (signature, _) = signing_key.sign_prehash_recoverable(&*message).unwrap();
            let signature = signature.normalize_s().unwrap_or(signature);

            let recovery_id = RecoveryId::from_byte(0).unwrap();
            let encoded = encode_signature(signature, recovery_id);

            let (de_sig, de_recid) = decode_signature(encoded).unwrap();
            assert_eq!(signature, de_sig);
            assert_eq!(recovery_id, de_recid);

            let recovery_id = RecoveryId::from_byte(1).unwrap();
            let encoded = encode_signature(signature, recovery_id);

            let (de_sig, de_recid) = decode_signature(encoded).unwrap();
            assert_eq!(signature, de_sig);
            assert_eq!(recovery_id, de_recid);
        }
    }
}
