use crate::common::PrefixError;

#[derive(Debug, Clone, derive_more::Display)]
pub enum DeserializeError {
    #[display(fmt = "{}", _0)]
    PrefixError(PrefixError),
}

impl From<PrefixError> for DeserializeError {
    fn from(err: PrefixError) -> Self {
        DeserializeError::PrefixError(err)
    }
}
