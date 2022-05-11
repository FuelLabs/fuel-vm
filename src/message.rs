use crate::Hasher;

pub use fuel_types::Bytes32;

use core::fmt;
use core::ops::Deref;

/// Normalized signature message
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Message(Bytes32);

impl Message {
    /// Memory length of the type
    pub const LEN: usize = Bytes32::LEN;

    /// Normalize a message for signature
    pub fn new<M>(message: M) -> Self
    where
        M: AsRef<[u8]>,
    {
        Self(Hasher::hash(message))
    }

    /// Add a conversion from arbitrary slices into owned
    ///
    /// # Safety
    ///
    /// There is no guarantee the provided bytes will be the product of a cryptographically secure
    /// hash. Using insecure messages might compromise the security of the signature.
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
    /// This function extends the unsafety of [`Self::from_bytes_unchecked`].
    pub unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
        Self(Bytes32::from_slice_unchecked(bytes))
    }

    /// Copy-free reference cast
    ///
    /// # Safety
    ///
    /// Inputs smaller than `Self::LEN` will cause undefined behavior.
    ///
    /// This function extends the unsafety of [`Self::from_bytes_unchecked`].
    pub unsafe fn as_ref_unchecked(bytes: &[u8]) -> &Self {
        // The interpreter will frequently make references to keys and values using
        // logically checked slices.
        //
        // This function will avoid unnecessary copy to owned slices for the interpreter
        // access
        &*(bytes.as_ptr() as *const Self)
    }
}

impl Deref for Message {
    type Target = [u8; Message::LEN];

    fn deref(&self) -> &[u8; Message::LEN] {
        self.0.deref()
    }
}

impl AsRef<[u8]> for Message {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<Message> for [u8; Message::LEN] {
    fn from(message: Message) -> [u8; Message::LEN] {
        message.0.into()
    }
}

impl From<&Hasher> for Message {
    fn from(hasher: &Hasher) -> Self {
        // Safety: `Hasher` is a cryptographic hash
        unsafe { Self::from_bytes_unchecked(*hasher.digest()) }
    }
}

impl From<Hasher> for Message {
    fn from(hasher: Hasher) -> Self {
        // Safety: `Hasher` is a cryptographic hash
        unsafe { Self::from_bytes_unchecked(*hasher.finalize()) }
    }
}

impl fmt::LowerHex for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "std")]
mod use_std {
    use crate::Message;

    use secp256k1::Message as Secp256k1Message;

    impl Message {
        pub(crate) fn to_secp(&self) -> Secp256k1Message {
            // The only validation performed by `Message::from_slice` is to check if it is
            // 32 bytes. This validation exists to prevent users from signing
            // non-hashed messages, which is a severe violation of the protocol
            // security.
            debug_assert_eq!(Self::LEN, secp256k1::constants::MESSAGE_SIZE);
            Secp256k1Message::from_slice(self.as_ref()).expect("Unreachable error")
        }
    }
}
