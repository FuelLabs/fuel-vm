use crate::Error;

use fuel_types::Bytes64;

use core::{
    fmt,
    ops::Deref,
    str,
};

use crate::{
    Message,
    PublicKey,
    SecretKey,
};

use k256::ecdsa::{
    RecoveryId,
    VerifyingKey,
};

/// Compact-form Secp256k1 signature.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
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

impl Signature {
    /// Separates recovery id from the signature bytes. See the following link for
    /// explanation. https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md#ecdsa-public-key-cryptography
    fn decode(&self) -> (k256::ecdsa::Signature, RecoveryId) {
        let mut sig = *self.0;
        let v = (sig[32] & 0x80) != 0;
        sig[32] &= 0x7f;

        let sig =
            k256::ecdsa::Signature::from_slice(&sig).expect("Signature must be valid");
        (sig, RecoveryId::new(v, false))
    }
}

impl Signature {
    /// Sign a given message and compress the `v` to the signature
    ///
    /// The compression scheme is described in
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md>
    pub fn sign(secret: &SecretKey, message: &Message) -> Self {
        let sk: k256::SecretKey = (*secret).into();
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

        // encode_signature cannot panic as we don't generate reduced-x recovery ids.

        // Combine recovery id with the signature bytes. See the following link for
        // explanation. https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md#ecdsa-public-key-cryptography
        // Panics if the highest bit of byte at index 32 is set, as this indicates
        // non-normalized signature. Panics if the recovery id is in reduced-x
        // form.
        let mut signature: [u8; 64] = signature.to_bytes().into();
        assert!(signature[32] >> 7 == 0, "Non-normalized signature");
        assert!(!recovery_id.is_x_reduced(), "Invalid recovery id");

        let v = recovery_id.is_y_odd() as u8;

        signature[32] = (v << 7) | (signature[32] & 0x7f);

        Self(Bytes64::from(signature))
    }

    /// Recover the public key from a signature performed with
    /// [`Signature::sign`]
    ///
    /// It takes the signature as owned because this operation is not idempotent. The
    /// taken signature will not be recoverable. Signatures are meant to be
    /// single use, so this avoids unnecessary copy.
    pub fn recover(self, message: &Message) -> Result<PublicKey, Error> {
        let (sig, recid) = self.decode();

        match VerifyingKey::recover_from_prehash(&**message, &sig, recid) {
            Ok(vk) => Ok(PublicKey::from(vk)),
            Err(_) => Err(Error::InvalidSignature),
        }
    }

    /// Verify a signature produced by [`Signature::sign`]
    ///
    /// It takes the signature as owned because this operation is not idempotent. The
    /// taken signature will not be recoverable. Signatures are meant to be
    /// single use, so this avoids unnecessary copy.
    pub fn verify(self, vk: &PublicKey, message: &Message) -> Result<(), Error> {
        // TODO: explain why hazmat is needed and why Message is prehash
        use ecdsa::signature::hazmat::PrehashVerifier;

        let vk: k256::PublicKey = (*vk).into(); // TODO: remove clone
        let vk: ecdsa::VerifyingKey<k256::Secp256k1> = vk.into();

        let (sig, _) = self.decode();

        vk.verify_prehash(&**message, &sig)
            .map_err(|_| Error::InvalidSignature)?;
        Ok(())
    }
}
