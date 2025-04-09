use crate::Hasher;
use core::{
    fmt,
    ops::Deref,
};
pub use fuel_types::Bytes32;

/// Normalized (hashed) message authenticated by a signature
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Message(Bytes32);

impl Message {
    /// Memory length of the type in bytes.
    pub const LEN: usize = Bytes32::LEN;

    /// Normalize the given message by cryptographically hashing its content in
    /// preparation for signing.
    pub fn new<M>(message: M) -> Self
    where
        M: AsRef<[u8]>,
    {
        Self(Hasher::hash(message))
    }

    /// Construct a `Message` directly from its bytes.
    ///
    /// This constructor expects the given bytes to be a valid,
    /// cryptographically hashed message. No hashing is performed.
    pub fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes.into())
    }

    /// Construct a `Message` reference directly from a reference to its bytes.
    ///
    /// This constructor expects the given bytes to be a valid,
    /// cryptographically hashed message. No hashing is performed.
    pub fn from_bytes_ref(bytes: &[u8; Self::LEN]) -> &Self {
        // TODO: Wrap this unsafe conversion safely in `fuel_types::Bytes32`.
        #[allow(unsafe_code)]
        unsafe {
            &*(bytes.as_ptr() as *const Self)
        }
    }

    /// Kept temporarily for backwards compatibility.
    #[deprecated = "Use `Message::from_bytes` instead"]
    pub fn from_bytes_unchecked(bytes: [u8; Self::LEN]) -> Self {
        Self::from_bytes(bytes)
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

impl From<Message> for Bytes32 {
    fn from(s: Message) -> Self {
        s.0
    }
}

impl From<&Hasher> for Message {
    fn from(hasher: &Hasher) -> Self {
        // Safety: `Hasher` is a cryptographic hash
        Self::from_bytes(*hasher.digest())
    }
}

impl From<Hasher> for Message {
    fn from(hasher: Hasher) -> Self {
        // Safety: `Hasher` is a cryptographic hash
        Self::from_bytes(*hasher.finalize())
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
impl From<&Message> for secp256k1::Message {
    fn from(message: &Message) -> Self {
        secp256k1::Message::from_digest_slice(&*message.0).expect("length always matches")
    }
}
