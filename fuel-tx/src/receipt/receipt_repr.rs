use crate::receipt::Receipt;

macro_rules! enum_from {
    (
        $(#[$meta:meta])* $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl From<&Receipt> for $name {
            fn from(receipt: &Receipt) -> Self {
                match receipt {
                    $(Receipt::$vname { .. } => Self::$vname,)*
                }
            }
        }

        #[cfg(feature = "std")]
        impl TryFrom<fuel_types::Word> for $name {
            type Error = std::io::Error;

            fn try_from(x: fuel_types::Word) -> Result<Self, Self::Error> {
                match x {
                    $(x if x == $name::$vname as fuel_types::Word => Ok($name::$vname),)*
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "The provided identifier is invalid!",
                    )),
                }
            }
        }
    }
}

enum_from! {
    #[allow(dead_code)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ReceiptRepr {
        Call = 0x00,
        Return = 0x01,
        ReturnData = 0x02,
        Panic = 0x03,
        Revert = 0x04,
        Log = 0x05,
        LogData = 0x06,
        Transfer = 0x07,
        TransferOut = 0x08,
        ScriptResult = 0x09,
        MessageOut = 0x0A,
        Mint = 0x0B,
        Burn = 0x0C,
    }
}
