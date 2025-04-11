use crate::{
    message::Message,
    secp256::{
        signature_format::{
            decode_signature,
            encode_signature,
            RecoveryId as SecpRecoveryId,
        },
        PublicKey,
    },
    Error,
};

use crate::SecretKey;

use k256::{
    ecdsa::{
        RecoveryId,
        VerifyingKey,
    },
    EncodedPoint,
};

#[cfg(feature = "random")]
use rand::{
    CryptoRng,
    RngCore,
};

/// Generates a random secret key
#[cfg(feature = "random")]
pub fn random_secret(rng: &mut (impl CryptoRng + RngCore)) -> SecretKey {
    k256::SecretKey::random(rng).into()
}

/// Derives the public key from a given secret key
pub fn public_key(secret: &SecretKey) -> PublicKey {
    let sk: k256::SecretKey = secret.into();
    let sk: ecdsa::SigningKey<k256::Secp256k1> = sk.into();
    let vk = sk.verifying_key();
    vk.into()
}

/// Sign a given message and compress the `v` to the signature
///
/// The compression scheme is described in
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md>
pub fn sign(secret: &SecretKey, message: &Message) -> [u8; 64] {
    let sk: k256::SecretKey = secret.into();
    let sk: ecdsa::SigningKey<k256::Secp256k1> = sk.into();
    let (signature, _recid) = sk
        .sign_prehash_recoverable(&**message)
        .expect("Infallible signature operation");

    // TODO: this is a hack to get the recovery id. The signature should be normalized
    // before computing the recovery id, but k256 library doesn't support this, and
    // instead always computes the recovery id from non-normalized signature.
    // So instead the recovery id is determined by checking which variant matches
    // the original public key.

    let recid1 = RecoveryId::new(false, false);
    let recid2 = RecoveryId::new(true, false);

    let rec1 = VerifyingKey::recover_from_prehash(&**message, &signature, recid1);
    let rec2 = VerifyingKey::recover_from_prehash(&**message, &signature, recid2);

    let actual = sk.verifying_key();

    let recovery_id = if rec1.map(|r| r == *actual).unwrap_or(false) {
        recid1
    } else if rec2.map(|r| r == *actual).unwrap_or(false) {
        recid2
    } else {
        unreachable!("Invalid signature generated");
    };

    let recovery_id = SecpRecoveryId::try_from(recovery_id)
        .expect("reduced-x recovery ids are never generated");
    encode_signature(signature.to_bytes().into(), recovery_id)
}

/// Recover the public key from a signature.
///
/// It takes the signature as owned because this operation is not idempotent. The
/// taken signature will not be recoverable. Signatures are meant to be
/// single use, so this avoids unnecessary copy.
pub fn recover(signature: [u8; 64], message: &Message) -> Result<PublicKey, Error> {
    let (sig, recid) = decode_signature(signature);
    let sig =
        k256::ecdsa::Signature::from_slice(&sig).map_err(|_| Error::InvalidSignature)?;
    let vk = VerifyingKey::recover_from_prehash(&**message, &sig, recid.into())
        .map_err(|_| Error::InvalidSignature)?;
    Ok(PublicKey::from(&vk))
}

/// Verify that a signature matches given public key
///
/// It takes the signature as owned because this operation is not idempotent. The
/// taken signature will not be recoverable. Signatures are meant to be
/// single use, so this avoids unnecessary copy.
pub fn verify(
    signature: [u8; 64],
    public_key: [u8; 64],
    message: &Message,
) -> Result<(), Error> {
    use ecdsa::signature::hazmat::PrehashVerifier;

    let vk = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
        &public_key.into(),
    ))
    .map_err(|_| Error::InvalidPublicKey)?;

    let (sig, _) = decode_signature(signature);
    let sig =
        k256::ecdsa::Signature::from_slice(&sig).map_err(|_| Error::InvalidSignature)?;

    vk.verify_prehash(&**message, &sig)
        .map_err(|_| Error::InvalidSignature)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use fuel_types::Bytes32;
    #[cfg(feature = "std")]
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn full() {
        let rng = &mut StdRng::seed_from_u64(1234);

        let secret = random_secret(rng);
        let public = public_key(&secret);

        let message = Message::new(rng.r#gen::<[u8; 10]>());

        let signature = sign(&secret, &message);
        verify(signature, *public, &message).expect("Verification failed");
        let recovered = recover(signature, &message).expect("Recovery failed");

        assert_eq!(public, recovered);
    }

    #[test]
    fn no_std() {
        let raw_secret: [u8; 32] = [
            0x99, 0xe8, 0x7b, 0xe, 0x91, 0x58, 0x53, 0x1e, 0xee, 0xb5, 0x3, 0xff, 0x15,
            0x26, 0x6e, 0x2b, 0x23, 0xc2, 0xa2, 0x50, 0x7b, 0x13, 0x8c, 0x9d, 0x1b, 0x1f,
            0x2a, 0xb4, 0x58, 0xdf, 0x2d, 0x6,
        ];
        let secret = SecretKey::try_from(Bytes32::from(raw_secret)).unwrap();
        let public = public_key(&secret);

        let message = Message::new(b"Every secret creates a potential failure point.");

        let signature = sign(&secret, &message);
        verify(signature, *public, &message).expect("Verification failed");
        let recovered = recover(signature, &message).expect("Recovery failed");

        assert_eq!(public, recovered);
    }
}
