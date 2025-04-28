use crate::{
    Error,
    Message,
    PublicKey,
    SecretKey,
    Signature,
};

#[cfg(feature = "std")]
use rand::{
    SeedableRng,
    rngs::StdRng,
};

#[cfg(feature = "std")]
#[test]
fn recover() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message = b"A beast can never be as cruel as a human being, so artistically, so picturesquely cruel.";

    for _ in 0..100 {
        let message = Message::new(message);

        let secret = SecretKey::random(rng);
        let public = secret.public_key();

        let signature = Signature::sign(&secret, &message);
        let recover = signature.recover(&message).expect("Failed to recover PK");

        assert_eq!(public, recover);
    }
}

#[cfg(feature = "std")]
#[test]
fn verify() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message = b"Music expresses that which cannot be put into words and that which cannot remain silent.";

    for _ in 0..100 {
        let message = Message::new(message);

        let secret = SecretKey::random(rng);
        let public = secret.public_key();

        let pub1 = crate::secp256::backend::k1::public_key(&secret);
        let pub2 = crate::secp256::backend::k1::public_key(&secret);
        assert_eq!(pub1, pub2);

        let signature = Signature::sign(&secret, &message);

        signature
            .verify(&public, &message)
            .expect("Failed to verify signature");
    }
}

#[test]
fn corrupted_signature() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message = b"When life itself seems lunatic, who knows where madness lies?";
    let message = Message::new(message);

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    let signature = Signature::sign(&secret, &message);

    // Tamper, bit by bit, the signature and public key.
    //
    // The recover and verify operations should fail in all cases.
    (0..Signature::LEN).for_each(|i| {
        (0..7).fold(1u8, |m, _| {
            let mut s = signature;

            s.as_mut()[i] ^= m;

            match s.recover(&message) {
                Ok(pk) => assert_ne!(public, pk),
                Err(Error::InvalidSignature) => (),
                Err(e) => panic!("Unexpected error: {e}"),
            }

            m << 1
        });
    });

    (0..Signature::LEN).for_each(|i| {
        (0..7).fold(1u8, |m, _| {
            let mut s = signature;

            s.as_mut()[i] ^= m;

            assert!(s.verify(&public, &message).is_err());

            m << 1
        });
    });

    (0..PublicKey::LEN).for_each(|i| {
        (0..7).fold(1u8, |m, _| {
            let mut p = public;

            p.as_mut()[i] ^= m;

            assert!(signature.verify(&p, &message).is_err());

            m << 1
        });
    });
}
