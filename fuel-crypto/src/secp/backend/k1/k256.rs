use crate::{
    message::Message,
    secp::{
        signature_format::{
            decode_signature,
            encode_signature,
            RecoveryId as SecpRecoveryId,
        },
        PublicKey,
    },
    Error,
    SecretKey,
};

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
pub fn public_key<SK: Into<k256::SecretKey>>(secret: SK) -> PublicKey {
    let sk: k256::SecretKey = secret.into();
    let sk: ecdsa::SigningKey<k256::Secp256k1> = sk.into();
    let vk = sk.verifying_key();
    vk.into()
}

/// Sign a given message and compress the `v` to the signature
///
/// The compression scheme is described in
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md>
pub fn sign<SK: Into<k256::SecretKey>>(secret: SK, message: &Message) -> [u8; 64] {
    let sk: k256::SecretKey = secret.into();
    let sk: ecdsa::SigningKey<k256::Secp256k1> = sk.into();
    let (signature, _recid) = sk
        .sign_prehash_recoverable(&**message)
        .expect("Infallible signature operation");

    // Hack: see secp256k1 more more info
    // TODO: clean up
    // TODO: merge impl with secp256k1

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
    // TODO: explain why hazmat is needed and why Message is prehash
    use ecdsa::signature::hazmat::PrehashVerifier;

    let vk = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
        &public_key.into(),
    ))
    .expect("Invalid public key");

    let (sig, _) = decode_signature(signature);
    let sig =
        k256::ecdsa::Signature::from_slice(&sig).map_err(|_| Error::InvalidSignature)?;

    vk.verify_prehash(&**message, &sig)
        .map_err(|_| Error::InvalidSignature)?;
    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    use super::*;

    #[test]
    fn full() {
        let rng = &mut StdRng::seed_from_u64(1234);

        let secret = random_secret(rng);
        let public = public_key(&secret);

        let message = Message::new(rng.gen::<[u8; 10]>());

        let signature = sign(&secret, &message);
        verify(signature, *public, &message).expect("Verification failed");
        let recovered = recover(signature, &message).expect("Recovery failed");

        assert_eq!(public, recovered);
    }
}
