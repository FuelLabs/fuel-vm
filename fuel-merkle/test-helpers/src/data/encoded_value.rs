use serde::{
    Deserialize,
    Serialize,
};

use crate::data::TestError;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Base64,
    Hex,
    #[serde(rename = "utf-8")]
    Utf8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EncodedValue {
    value: String,
    encoding: Encoding,
}

impl EncodedValue {
    pub fn new(value: String, encoding: Encoding) -> Self {
        Self { value, encoding }
    }

    pub fn from_raw<T: AsRef<[u8]>>(value: T, encoding: Encoding) -> Self {
        let encoded_value = match encoding {
            Encoding::Base64 => base64::encode(value),
            Encoding::Hex => hex::encode(value),
            Encoding::Utf8 => String::from_utf8_lossy(value.as_ref()).to_string(),
        };
        Self {
            value: encoded_value,
            encoding,
        }
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, TestError> {
        match self.encoding {
            Encoding::Base64 => {
                base64::decode(self.value).map_err(|_| TestError::DecodingError)
            }
            Encoding::Hex => {
                hex::decode(self.value).map_err(|_| TestError::DecodingError)
            }
            Encoding::Utf8 => Ok(self.value.into_bytes()),
        }
    }
}
