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

use secp256k1::{
    ecdsa::{
        RecoverableSignature,
        Signature,
    },
    Secp256k1,
};
use std::sync::OnceLock;

use crate::SecretKey;

#[cfg(feature = "random")]
use rand::{
    CryptoRng,
    RngCore,
};

fn get_context() -> &'static Secp256k1<secp256k1::All> {
    static CONTEXT: OnceLock<Secp256k1<secp256k1::All>> = OnceLock::new();
    CONTEXT.get_or_init(Secp256k1::new)
}

/// Generates a random secret key
#[cfg(feature = "random")]
pub fn random_secret(rng: &mut (impl CryptoRng + RngCore)) -> SecretKey {
    secp256k1::SecretKey::new(rng).into()
}

/// Derives the public key from a given secret key
pub fn public_key(secret: &SecretKey) -> PublicKey {
    let sk: secp256k1::SecretKey = secret.into();
    let vk = secp256k1::PublicKey::from_secret_key(get_context(), &sk);
    vk.into()
}

/// Sign a given message and compress the `v` to the signature
///
/// The compression scheme is described in
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md>
pub fn sign(secret: &SecretKey, message: &Message) -> [u8; 64] {
    let signature = get_context().sign_ecdsa_recoverable(&message.into(), &secret.into());
    let (recovery_id, signature) = signature.serialize_compact();

    // encode_signature cannot panic as we don't generate reduced-x recovery ids.
    let recovery_id = SecpRecoveryId::try_from(recovery_id)
        .expect("reduced-x recovery ids are never generated");
    encode_signature(signature, recovery_id)
}

/// Recover the public key from a signature.
///
/// It takes the signature as owned because this operation is not idempotent. The
/// taken signature will not be recoverable. Signatures are meant to be
/// single use, so this avoids unnecessary copy.
pub fn recover(signature: [u8; 64], message: &Message) -> Result<PublicKey, Error> {
    let (signature, recovery_id) = decode_signature(signature);
    let recoverable = RecoverableSignature::from_compact(&signature, recovery_id.into())
        .map_err(|_| Error::InvalidSignature)?;
    let vk = get_context()
        .recover_ecdsa(&message.into(), &recoverable)
        .map_err(|_| Error::InvalidSignature)?;
    Ok(PublicKey::from(vk))
}

/// Verify that a signature matches given public key
pub fn verify(
    signature: [u8; 64],
    public_key: [u8; 64],
    message: &Message,
) -> Result<(), Error> {
    let (signature, _) = decode_signature(signature); // Trunactes recovery id
    let signature =
        Signature::from_compact(&signature).map_err(|_| Error::InvalidSignature)?;

    let mut prefixed_public_key = [0u8; 65];
    prefixed_public_key[0] = 0x04; // Uncompressed
    prefixed_public_key[1..].copy_from_slice(&public_key);
    let vk = secp256k1::PublicKey::from_slice(&prefixed_public_key)
        .map_err(|_| Error::InvalidPublicKey)?;

    get_context()
        .verify_ecdsa(&message.into(), &signature, &vk)
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

        let message = Message::new(rng.r#gen::<[u8; 10]>());

        let signature = sign(&secret, &message);
        verify(signature, *public, &message).expect("Verification failed");
        let recovered = recover(signature, &message).expect("Recovery failed");

        assert_eq!(public, recovered);
    }
}
