pub mod binary;
pub mod sparse;

mod encoded_value;
mod test_error;

pub use encoded_value::{
    EncodedValue,
    Encoding,
};
pub use test_error::TestError;
