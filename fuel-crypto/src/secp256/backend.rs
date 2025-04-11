//! Backends for different secp-style elliptic curves

/// secp256k1 implementations
pub mod k1 {
    // The k256 module is always available in-crate, since it's tested against secp256k1
    #[cfg_attr(feature = "std", allow(dead_code))]
    pub(crate) mod k256;
    #[cfg(feature = "std")]
    pub(crate) mod secp256k1;

    // Pick the default backend
    #[cfg(not(feature = "std"))]
    pub use self::k256::*;
    #[cfg(feature = "std")]
    pub use self::secp256k1::*;
}

/// secp256r1 implementations
pub mod r1 {
    pub mod p256;
    pub use self::p256::*;
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    use crate::{
        message::Message,
        secp256::SecretKey,
    };

    use super::k1::{
        k256,
        secp256k1,
    };

    /// Make sure that the k256 and secp256k1 backends produce the same results
    #[test]
    fn equivalent_k256_secp256k1() {
        let rng = &mut StdRng::seed_from_u64(1234);

        for case in 0..100 {
            let secret = SecretKey::random(rng);
            let message = Message::new(vec![rng.r#gen(); case]);

            let public_k = k256::public_key(&secret);
            let public_s = secp256k1::public_key(&secret);
            assert_eq!(public_k, public_s);

            let signed_k = k256::sign(&secret, &message);
            let signed_s = secp256k1::sign(&secret, &message);
            assert_eq!(signed_k, signed_s);

            k256::verify(signed_k, *public_k, &message).expect("Failed to verify (k256)");
            secp256k1::verify(signed_s, *public_s, &message)
                .expect("Failed to verify (secp256k1)");

            let recovered_k =
                k256::recover(signed_k, &message).expect("Failed to recover (k256)");
            let recovered_s = secp256k1::recover(signed_k, &message)
                .expect("Failed to recover (secp256k1)");
            assert_eq!(recovered_k, recovered_s);
        }
    }
}
