use fuel_types::Bytes64;

use core::fmt;
use core::ops::Deref;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
/// Secp256k1 signature implementation
pub struct Signature(Bytes64);

impl Signature {
    /// Memory length of the type
    pub const LEN: usize = Bytes64::LEN;

    /// Add a conversion from arbitrary slices into owned
    ///
    /// # Safety
    ///
    /// There is no guarantee the provided bytes will be a valid signature. Internally, some FFI
    /// calls to `secp256k1` are performed and we might have undefined behavior in case the bytes
    /// are not canonically encoded to a valid `secp256k1` signature.
    pub unsafe fn from_bytes_unchecked(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes.into())
    }

    /// Add a conversion from arbitrary slices into owned
    ///
    /// # Safety
    ///
    /// This function will not panic if the length of the slice is smaller than
    /// `Self::LEN`. Instead, it will cause undefined behavior and read random
    /// disowned bytes.
    ///
    /// There is no guarantee the provided bytes will be a valid signature. Internally, some FFI
    /// calls to `secp256k1` are performed and we might have undefined behavior in case the bytes
    /// are not canonically encoded to a valid `secp256k1` signature.
    pub unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
        Self(Bytes64::from_slice_unchecked(bytes))
    }

    /// Copy-free reference cast
    ///
    /// There is no guarantee the provided bytes will fit the field.
    ///
    /// # Safety
    ///
    /// Inputs smaller than `Self::LEN` will cause undefined behavior.
    pub unsafe fn as_ref_unchecked(bytes: &[u8]) -> &Self {
        // The interpreter will frequently make references to keys and values using
        // logically checked slices.
        //
        // This function will avoid unnecessary copy to owned slices for the interpreter
        // access
        &*(bytes.as_ptr() as *const Self)
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

impl From<Signature> for [u8; Signature::LEN] {
    fn from(salt: Signature) -> [u8; Signature::LEN] {
        salt.0.into()
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

#[cfg(feature = "std")]
mod use_std {
    use crate::{Error, Message, PublicKey, SecretKey, Signature};

    use secp256k1::recovery::{RecoverableSignature as SecpRecoverableSignature, RecoveryId};
    use secp256k1::Secp256k1;

    use std::borrow::Borrow;

    impl Signature {
        // Internal API - this isn't meant to be made public because some assumptions and pre-checks
        // are performed prior to this call
        pub(crate) fn to_secp(&mut self) -> SecpRecoverableSignature {
            let v = (self.as_mut()[32] >> 7) as i32;

            self.truncate_recovery_id();

            let v = RecoveryId::from_i32(v)
                .unwrap_or_else(|_| RecoveryId::from_i32(0).expect("0 is infallible recovery ID"));

            let signature = SecpRecoverableSignature::from_compact(self.as_ref(), v)
                .unwrap_or_else(|_| {
                    SecpRecoverableSignature::from_compact(&[0u8; 64], v)
                        .expect("Zeroed signature is infallible")
                });

            signature
        }

        pub(crate) fn from_secp(signature: SecpRecoverableSignature) -> Self {
            let (v, mut signature) = signature.serialize_compact();

            let v = v.to_i32();

            signature[32] |= (v << 7) as u8;

            // Safety: the security of this call reflects the security of secp256k1 FFI
            unsafe { Signature::from_bytes_unchecked(signature) }
        }

        /// Truncate the recovery id from the signature, producing a valid `secp256k1`
        /// representation.
        pub(crate) fn truncate_recovery_id(&mut self) {
            self.as_mut()[32] &= 0x7f;
        }

        /// Sign a given message and compress the `v` to the signature
        ///
        /// The compression scheme is described in
        /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/cryptographic_primitives.md#public-key-cryptography>
        pub fn sign(secret: &SecretKey, message: &Message) -> Self {
            let secret = secret.borrow();
            let message = message.to_secp();

            let signature = Secp256k1::signing_only().sign_recoverable(&message, secret);

            Signature::from_secp(signature)
        }

        /// Recover the public key from a signature performed with
        /// [`Signature::sign`]
        ///
        /// It takes the signature as owned because this operation is not idempotent. The taken
        /// signature will not be recoverable. Signatures are meant to be single use, so this
        /// avoids unnecessary copy.
        pub fn recover(mut self, message: &Message) -> Result<PublicKey, Error> {
            let signature = self.to_secp();
            let message = message.to_secp();

            let pk = Secp256k1::new()
                .recover(&message, &signature)
                .map(|pk| PublicKey::from_secp(&pk))?;

            Ok(pk)
        }

        /// Verify a signature produced by [`Signature::sign`]
        ///
        /// It takes the signature as owned because this operation is not idempotent. The taken
        /// signature will not be recoverable. Signatures are meant to be single use, so this
        /// avoids unnecessary copy.
        pub fn verify(self, pk: &PublicKey, message: &Message) -> Result<(), Error> {
            // TODO evaluate if its worthy to use native verify
            //
            // https://github.com/FuelLabs/fuel-crypto/issues/4

            self.recover(message)
                .and_then(|pk_p| (pk == &pk_p).then(|| ()).ok_or(Error::InvalidSignature))
        }
    }
}
