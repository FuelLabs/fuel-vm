mod input;
mod output;
mod witness;

pub type Address = [u8; 32];
pub type Color = [u8; 32];
pub type ContractAddress = [u8; 32];
pub type Hash = [u8; 32];
pub type Salt = [u8; 32];

pub use input::Input;
pub use output::Output;
pub use witness::Witness;
