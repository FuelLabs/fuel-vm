//! secp256r1 (P-256) functions

use crate::{
    Error,
    Message,
};
use coins_bip32::prelude::RecoveryId;
use fuel_types::Bytes64;
use p256::ecdsa::{
    Signature,
    VerifyingKey,
};

/// Recover a public key from a signature and a message digest.
pub fn recover(signature: &Bytes64, message: &Message) -> Result<Bytes64, Error> {
    let Ok(signature) = Signature::from_slice(&**signature) else {
        return Err(Error::InvalidSignature);
    };

    for recid in 0..RecoveryId::MAX {
        let recid = RecoveryId::from_byte(recid).unwrap();
        if let Ok(pub_key) =
            VerifyingKey::recover_from_prehash(&**message, &signature, recid)
        {
            // Extract the compressed public key. The first byte is ignored, as it's a
            // flag for compression: https://en.bitcoin.it/wiki/Elliptic_Curve_Digital_Signature_Algorithm
            let mut result = [0u8; 64];
            result.copy_from_slice(&pub_key.to_encoded_point(false).to_bytes()[1..]);
            return Ok(Bytes64::from(result))
        }
    }

    Err(Error::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;

    use p256::ecdsa::SigningKey;
    use rand::{
        rngs::StdRng,
        SeedableRng,
    };

    use crate::Message;

    #[test]
    fn secp256r1_recover_from_msg() {
        let mut rng = &mut StdRng::seed_from_u64(1234);

        let signing_key = SigningKey::random(&mut rng);
        let verifying_key = signing_key.verifying_key();

        let message = Message::new([3u8; 100]);
        let (signature, _recid) =
            signing_key.sign_prehash_recoverable(&*message).unwrap();

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&signature.to_bytes());
        let Ok(recovered) = recover(&Bytes64::from(sig_bytes), &message) else {
            panic!("Failed to recover public key from the message");
        };

        assert!(*recovered == verifying_key.to_encoded_point(false).to_bytes()[1..]);
    }
}
