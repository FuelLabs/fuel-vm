/// Converts value to usize is a way that's consistet on 32-bit and 64-bit platforms.
pub(crate) fn to_usize(value: u64) -> Option<usize> {
    usize::try_from(u32::try_from(value).ok()?).ok()
}
