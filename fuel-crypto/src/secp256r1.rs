//! secp256r1 (P-256) functions

use crate::Error;
use coins_bip32::prelude::RecoveryId;
use fuel_types::{
    Bytes32,
    Bytes64,
};
use p256::{
    ecdsa::{
        Signature,
        VerifyingKey,
    },
    elliptic_curve::group::GroupEncoding,
};

/// Recover a public key from a signature and a message digest.
pub fn recover(signature: &Bytes64, message: &Bytes32) -> Result<Bytes32, Error> {
    let Ok(signature) = Signature::from_slice(&**signature) else {
        return Err(Error::InvalidSignature);
    };

    // Attempt the four possible recovery ids
    for recid in 0..RecoveryId::MAX {
        let recid = RecoveryId::from_byte(recid).unwrap();
        if let Ok(pub_key) = VerifyingKey::recover_from_msg(&**message, &signature, recid)
        {
            let mut result = [0u8; 32];
            result.copy_from_slice(&pub_key.as_affine().to_bytes()[1..]);
            return Ok(Bytes32::from(result))
        }
    }

    Err(Error::InvalidSignature)
}
