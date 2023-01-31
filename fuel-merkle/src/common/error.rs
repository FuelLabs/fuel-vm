use crate::common::PrefixError;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum DeserializeError {
    #[cfg_attr(feature = "std", error(transparent))]
    PrefixError(PrefixError),
}

impl From<PrefixError> for DeserializeError {
    fn from(err: PrefixError) -> Self {
        DeserializeError::PrefixError(err)
    }
}
