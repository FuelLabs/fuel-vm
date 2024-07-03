use alloc::vec::Vec;
use core::{
    fmt,
    fmt::Formatter,
};

/// Formatting utility to truncate a vector of bytes to a hex string of max length `N`
pub fn fmt_truncated_hex<const N: usize>(data: &[u8], f: &mut Formatter) -> fmt::Result {
    let formatted = if data.len() > N {
        let mut s = hex::encode(&data[0..N.saturating_sub(3)]);
        s.push_str("...");
        s
    } else {
        hex::encode(data)
    };
    f.write_str(formatted.as_str())
}

/// Formatting utility to truncate a optional vector of bytes to a hex string of max
/// length `N`
pub fn fmt_option_truncated_hex<const N: usize>(
    data: &Option<Vec<u8>>,
    f: &mut Formatter,
) -> fmt::Result {
    if let Some(data) = data {
        let formatted = if data.len() > N {
            let mut s = hex::encode(&data[0..N.saturating_sub(3)]);
            s.push_str("...");
            s
        } else {
            hex::encode(data)
        };
        f.write_str("Some(")?;
        f.write_str(formatted.as_str())?;
        f.write_str(")")
    } else {
        f.write_str("None")
    }
}
