use core::convert::Infallible;

/// Crypto error variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// Invalid secp256k1 secret key
    InvalidSecretKey,

    /// Invalid secp256k1 public key
    InvalidPublicKey,

    /// Invalid secp256k1 signature message
    InvalidMessage,

    /// Invalid secp256k1 signature
    InvalidSignature,

    /// The provided key wasn't found
    KeyNotFound,

    /// The keystore isn't available or is corrupted
    KeystoreNotAvailable,

    /// Out of preallocated memory
    NotEnoughMemory,
}

impl From<Error> for Infallible {
    fn from(_: Error) -> Infallible {
        unreachable!()
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Error {
        unreachable!()
    }
}

#[cfg(feature = "std")]
mod use_std {
    use super::*;
    use secp256k1::Error as Secp256k1Error;
    use std::{error, fmt, io};

    impl From<Secp256k1Error> for Error {
        fn from(secp: Secp256k1Error) -> Self {
            match secp {
                Secp256k1Error::IncorrectSignature
                | Secp256k1Error::InvalidSignature
                | Secp256k1Error::InvalidTweak
                | Secp256k1Error::TweakCheckFailed
                | Secp256k1Error::InvalidRecoveryId => Self::InvalidSignature,
                Secp256k1Error::InvalidMessage => Self::InvalidMessage,
                Secp256k1Error::InvalidPublicKey => Self::InvalidPublicKey,
                Secp256k1Error::InvalidSecretKey => Self::InvalidSecretKey,
                Secp256k1Error::NotEnoughMemory => Self::NotEnoughMemory,
            }
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl error::Error for Error {
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            None
        }
    }

    impl From<Error> for io::Error {
        fn from(e: Error) -> io::Error {
            io::Error::new(io::ErrorKind::Other, e)
        }
    }
}
