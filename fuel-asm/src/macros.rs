/// Defines the enum with `TryFrom` trait implementation.
#[macro_export]
macro_rules! enum_try_from {
    (
        $(#[$meta:meta])* $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
        },
        $from:ident
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<$from> for $name {
            type Error = $crate::PanicReason;

            fn try_from(v: $from) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as $from => Ok($name::$vname),)*
                    _ => Err($crate::PanicReason::InvalidMetadataIdentifier),
                }
            }
        }
    }
}
