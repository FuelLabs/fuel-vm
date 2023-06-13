use crate::Error;

use fuel_types::Bytes64;

use core::{
    fmt,
    ops::Deref,
    str,
};

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
/// Secp256k1 signature implementation
pub struct Signature(Bytes64);

impl Signature {
    /// Memory length of the type in bytes.
    pub const LEN: usize = Bytes64::LEN;

    /// Construct a `Signature` directly from its bytes.
    ///
    /// This constructor expects the given bytes to be a valid signature. No signing is
    /// performed.
    pub fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes.into())
    }

    /// Construct a `Signature` reference directly from a reference to its bytes.
    ///
    /// This constructor expects the given bytes to be a valid signature. No signing is
    /// performed.
    pub fn from_bytes_ref(bytes: &[u8; Self::LEN]) -> &Self {
        // TODO: Wrap this unsafe conversion safely in `fuel_types::Bytes64`.
        #[allow(unsafe_code)]
        unsafe {
            &*(bytes.as_ptr() as *const Self)
        }
    }

    /// Kept temporarily for backwards compatibility.
    #[deprecated = "Use `Signature::from_bytes` instead"]
    pub fn from_bytes_unchecked(bytes: [u8; Self::LEN]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl Deref for Signature {
    type Target = [u8; Signature::LEN];

    fn deref(&self) -> &[u8; Signature::LEN] {
        self.0.deref()
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for Signature {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl fmt::LowerHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Signature> for [u8; Signature::LEN] {
    fn from(salt: Signature) -> [u8; Signature::LEN] {
        salt.0.into()
    }
}

impl From<Signature> for Bytes64 {
    fn from(s: Signature) -> Self {
        s.0
    }
}

impl str::FromStr for Signature {
    type Err = Error;

    /// Parse a `Signature` directly from its bytes encoded as hex in a string.
    ///
    /// This constructor does not perform any signing.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Bytes64::from_str(s)
            .map_err(|_| Error::InvalidSignature)
            .map(|s| Self::from_bytes(s.into()))
    }
}

#[cfg(feature = "std")]
mod use_std {
    use crate::{
        Error,
        Message,
        PublicKey,
        SecretKey,
        Signature,
    };

    use lazy_static::lazy_static;
    use secp256k1::{
        ecdsa::{
            RecoverableSignature as SecpRecoverableSignature,
            RecoveryId,
        },
        Secp256k1,
    };

    use std::borrow::Borrow;

    lazy_static! {
        static ref SIGNING_SECP: Secp256k1<secp256k1::SignOnly> =
            Secp256k1::signing_only();
        static ref RECOVER_SECP: Secp256k1<secp256k1::All> = Secp256k1::new();
    }

    impl Signature {
        // Internal API - this isn't meant to be made public because some assumptions and
        // pre-checks are performed prior to this call
        fn to_secp(&mut self) -> SecpRecoverableSignature {
            let v = (self.as_mut()[32] >> 7) as i32;

            self.truncate_recovery_id();

            let v = RecoveryId::from_i32(v).unwrap_or_else(|_| {
                RecoveryId::from_i32(0).expect("0 is infallible recovery ID")
            });

            let signature = SecpRecoverableSignature::from_compact(self.as_ref(), v)
                .unwrap_or_else(|_| {
                    SecpRecoverableSignature::from_compact(&[0u8; 64], v)
                        .expect("Zeroed signature is infallible")
                });

            signature
        }

        fn from_secp(signature: SecpRecoverableSignature) -> Self {
            let (v, mut signature) = signature.serialize_compact();

            let v = v.to_i32();

            signature[32] |= (v << 7) as u8;
            Signature::from_bytes(signature)
        }

        /// Truncate the recovery id from the signature, producing a valid `secp256k1`
        /// representation.
        fn truncate_recovery_id(&mut self) {
            self.as_mut()[32] &= 0x7f;
        }

        /// Sign a given message and compress the `v` to the signature
        ///
        /// The compression scheme is described in
        /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic_primitives.md>
        pub fn sign(secret: &SecretKey, message: &Message) -> Self {
            let secret = secret.borrow();
            let message = message.to_secp();

            let signature = SIGNING_SECP.sign_ecdsa_recoverable(&message, secret);

            Signature::from_secp(signature)
        }

        /// Recover the public key from a signature performed with
        /// [`Signature::sign`]
        ///
        /// It takes the signature as owned because this operation is not idempotent. The
        /// taken signature will not be recoverable. Signatures are meant to be
        /// single use, so this avoids unnecessary copy.
        pub fn recover(mut self, message: &Message) -> Result<PublicKey, Error> {
            let signature = self.to_secp();
            let message = message.to_secp();

            let pk = RECOVER_SECP
                .recover_ecdsa(&message, &signature)
                .map(|pk| PublicKey::from_secp(&pk))?;

            Ok(pk)
        }

        /// Verify a signature produced by [`Signature::sign`]
        ///
        /// It takes the signature as owned because this operation is not idempotent. The
        /// taken signature will not be recoverable. Signatures are meant to be
        /// single use, so this avoids unnecessary copy.
        pub fn verify(mut self, pk: &PublicKey, message: &Message) -> Result<(), Error> {
            let signature = self.to_secp().to_standard();
            let message = message.to_secp();
            let pk = pk.to_secp()?;
            RECOVER_SECP
                .verify_ecdsa(&message, &signature, &pk)
                .map_err(|_| Error::InvalidSignature)
        }
    }
}
