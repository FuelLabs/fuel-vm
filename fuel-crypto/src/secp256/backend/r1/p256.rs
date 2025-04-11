//! secp256r1 (P-256) functions

#[cfg(feature = "test-helpers")]
use crate::secp256::signature_format::encode_signature;
use crate::{
    message::Message,
    secp256::signature_format::decode_signature,
    Error,
};
#[cfg(feature = "test-helpers")]
use ecdsa::RecoveryId;
use fuel_types::Bytes64;
use p256::ecdsa::VerifyingKey;

/// Sign a prehashed message. With the given key.
#[cfg(feature = "test-helpers")]
pub fn sign_prehashed(
    signing_key: &p256::ecdsa::SigningKey,
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

    let recovery_id = recovery_id
        .try_into()
        .expect("reduced-x recovery ids are never generated");
    Ok(Bytes64::from(encode_signature(
        signature.to_bytes().into(),
        recovery_id,
    )))
}

/// Convert the public key point to its uncompressed non-prefixed representation,
/// i.e. 32 bytes of x coordinate and 32 bytes of y coordinate.
#[cfg(feature = "test-helpers")]
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
    let (sig, recid) = decode_signature(**signature);
    let sig =
        p256::ecdsa::Signature::from_slice(&sig).map_err(|_| Error::InvalidSignature)?;
    let vk = VerifyingKey::recover_from_prehash(&**message, &sig, recid.into())
        .map_err(|_| Error::InvalidSignature)?;
    let point = vk.to_encoded_point(false);
    let mut raw = Bytes64::zeroed();
    raw[..32].copy_from_slice(point.x().unwrap());
    raw[32..].copy_from_slice(point.y().unwrap());
    Ok(raw)
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

        let message = Message::new([rng.r#gen(); 100]);

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

            let message = Message::new([rng.r#gen(); 100]);
            let signature =
                sign_prehashed(&signing_key, &message).expect("Couldn't sign");

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
            let message = Message::new([rng.r#gen(); 100]);
            let signing_key = SigningKey::random(&mut rng);
            let (signature, _) = signing_key.sign_prehash_recoverable(&*message).unwrap();
            let signature = signature.normalize_s().unwrap_or(signature);
            let signature: [u8; 64] = signature.to_bytes().into();

            let recovery_id = RecoveryId::from_byte(0).unwrap().try_into().unwrap();
            let encoded = encode_signature(signature, recovery_id);

            let (de_sig, de_recid) = decode_signature(encoded);
            assert_eq!(signature, de_sig);
            assert_eq!(recovery_id, de_recid);

            let recovery_id = RecoveryId::from_byte(1).unwrap().try_into().unwrap();
            let encoded = encode_signature(signature, recovery_id);

            let (de_sig, de_recid) = decode_signature(encoded);
            assert_eq!(signature, de_sig);
            assert_eq!(recovery_id, de_recid);
        }
    }
}
