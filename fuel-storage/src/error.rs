/// Wraps around possible errors that can occur during storage operations.
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum StorageError {
    /// Storage is unavailable in predicate context.
    #[cfg_attr(feature = "std", error("storage is unavailable in predicate context"))]
    Unavailable,
    /// Error occurred during serialization or deserialization of the entity.
    #[cfg_attr(
        feature = "std",
        error("error performing serialization or deserialization")
    )]
    Codec,
    /// Error occurred during interaction with database.
    #[cfg(feature = "std")]
    #[error("error occurred in the underlying datastore `{0}`")]
    Database(Box<dyn std::error::Error + Send + Sync>),
    /// The requested object of given type was not found.
    /// This is primarily used in fuel-core, which creates these with `not_found` macro.
    #[cfg_attr(
        feature = "std",
        error("resource of type `{0}` was not found at the: {1}")
    )]
    NotFound(&'static str, &'static str),
    /// Unknown or unexpected error.
    #[cfg_attr(feature = "std", error(transparent))]
    Other(#[cfg_attr(feature = "std", from)] anyhow::Error),
}

#[cfg(not(feature = "std"))]
impl core::fmt::Display for StorageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Storage error: {:?}", self)
    }
}

impl PartialEq for StorageError {
    /// We do our best here, but not all errors are comparable easily,
    /// so we fall back to true if they have same variant and non-compareable inner error.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NotFound(a, b), Self::NotFound(c, d)) => a == c && b == d,
            #[cfg(feature = "std")]
            (Self::Database(a), Self::Database(b)) => a.to_string() == b.to_string(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[cfg(feature = "std")]
impl From<StorageError> for std::io::Error {
    fn from(e: StorageError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    }
}
