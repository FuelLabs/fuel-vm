use fuel_crypto::{Error, Hasher, PublicKey, SecretKey, Signature};
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn ecrecover() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message =
        b"A beast can never be as cruel as a human being, so artistically, so picturesquely cruel.";

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    let signature = secret.sign(message);
    let recover = PublicKey::recover(signature, message).expect("Failed to recover PK");

    assert_eq!(public, recover);
}

#[test]
fn ecrecover_corrupted_signature() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message = b"When life itself seems lunatic, who knows where madness lies?";

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    let signature = secret.sign(message);

    (0..Signature::LEN).for_each(|i| {
        (0..7).fold(1u8, |m, _| {
            let mut s = signature;

            s[i] ^= m;

            match PublicKey::recover(s, message) {
                Ok(pk) => assert_ne!(public, pk),
                Err(Error::InvalidSignature) => (),
                Err(e) => panic!("Unexpected error: {}", e),
            }

            m << 1
        });
    });
}

#[test]
fn ecrecover_unchecked() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message =
        b"Music expresses that which cannot be put into words and that which cannot remain silent.";
    let message = Hasher::hash(message);

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    let signature = unsafe { secret.sign_unchecked(message) };
    let recover =
        unsafe { PublicKey::recover_unchecked(signature, message).expect("Failed to recover PK") };

    assert_eq!(public, recover);
}

#[test]
fn ecrecover_unchecked_corrupted_signature() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message = b"All things excellent are as difficult as they are rare.";
    let message = Hasher::hash(message);

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    let signature = unsafe { secret.sign_unchecked(message) };

    (0..Signature::LEN).for_each(|i| {
        (0..7).fold(1u8, |m, _| {
            let mut s = signature;

            s[i] ^= m;

            let recover = unsafe { PublicKey::recover_unchecked(s, message) };

            match recover {
                Ok(pk) => assert_ne!(public, pk),
                Err(Error::InvalidSignature) => (),
                Err(e) => panic!("Unexpected error: {}", e),
            }

            m << 1
        });
    });
}
