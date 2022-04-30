use crate::Word;

#[cfg(feature = "std")]
pub use use_std::*;

#[cfg(feature = "alloc")]
pub use use_alloc::*;

/// Memory size of a [`Word`]
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

/// Define the amount of bytes for a serialization implementation.
pub trait SizedBytes {
    /// Return the expected serialized size for an instance of the type.
    fn serialized_size(&self) -> usize;
}

/// Return the word-padded length of the buffer
pub const fn padded_len(bytes: &[u8]) -> usize {
    padded_len_usize(bytes.len())
}

/// Return the word-padded length of an arbitrary length
pub const fn padded_len_usize(len: usize) -> usize {
    let pad = len % WORD_SIZE;

    // `pad != 0` is checked because we shouldn't pad in case the length is already well-formed.
    //
    // Example being `w := WORD_SIZE` and `x := 2 · w`
    //
    // 1) With the check (correct result)
    // f(x) -> x + (x % w != 0) · (w - x % w)
    // f(x) -> x + 0 · w
    // f(x) -> x
    //
    // 2) Without the check (incorrect result)
    // f(x) -> x + w - x % w
    // f(x) -> x + w
    len + (pad != 0) as usize * (WORD_SIZE - pad)
}

/// Consume the initial bytes of a buffer to store a word.
///
/// Return the remainder of the buffer
///
/// # Panics
///
/// This function will panic if the length of the buffer is smaller than a word
pub fn store_number_unchecked<T>(buf: &mut [u8], number: T) -> &mut [u8]
where
    T: Into<Word>,
{
    buf[..WORD_SIZE].copy_from_slice(&number.into().to_be_bytes());

    &mut buf[WORD_SIZE..]
}

/// Read the initial bytes of a buffer to fetch a word.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_number_unchecked<T>(buf: &[u8]) -> (T, &[u8])
where
    T: From<Word>,
{
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number).into();

    (number, &buf[WORD_SIZE..])
}

/// Read the initial bytes of a buffer to fetch a word.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_word_unchecked(buf: &[u8]) -> (Word, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number);

    (number, &buf[WORD_SIZE..])
}

/// Read the a word-padded u8 from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_u8_unchecked(buf: &[u8]) -> (u8, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u8;

    (number, &buf[WORD_SIZE..])
}

/// Read the a word-padded u16 from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_u16_unchecked(buf: &[u8]) -> (u16, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u16;

    (number, &buf[WORD_SIZE..])
}

/// Read the a word-padded u32 from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_u32_unchecked(buf: &[u8]) -> (u32, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u32;

    (number, &buf[WORD_SIZE..])
}

/// Read the a word-padded usize from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_usize_unchecked(buf: &[u8]) -> (usize, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as usize;

    (number, &buf[WORD_SIZE..])
}

/// Copy `array` into `buf` and return the remainder bytes of `buf`
///
/// # Panics
///
/// This function will panic if the length of `buf` is smaller than `N`
pub fn store_array_unchecked<'a, const N: usize>(
    buf: &'a mut [u8],
    array: &[u8; N],
) -> &'a mut [u8] {
    buf[..N].copy_from_slice(array);

    &mut buf[N..]
}

/// Read an array of `N` bytes from `buf`.
///
/// Return the read array and the remainder bytes.
///
/// # Safety
///
/// This function will panic if the length of `buf` is smaller than `N`
pub unsafe fn restore_array_unchecked<const N: usize>(buf: &[u8]) -> ([u8; N], &[u8]) {
    (from_slice_unchecked(buf), &buf[N..])
}

/// Add a conversion from arbitrary slices into arrays
///
/// # Safety
///
/// This function will not panic if the length of the slice is smaller than `N`. Instead, it will
/// cause undefined behavior and read random disowned bytes.
pub unsafe fn from_slice_unchecked<const N: usize>(buf: &[u8]) -> [u8; N] {
    let ptr = buf.as_ptr() as *const [u8; N];

    // Static assertions are not applicable to runtime length check (e.g. slices).
    // This is safe if the size of `bytes` is consistent to `N`
    *ptr
}

#[cfg(feature = "alloc")]
mod use_alloc {
    use super::*;

    use alloc::vec::Vec;

    /// Auto-trait to create variable sized vectors out of [`SizedBytes`] implementations.
    pub trait SerializableVec: SizedBytes {
        /// Create a variable size vector of bytes from the instance.
        fn to_bytes(&mut self) -> Vec<u8>;
    }
}

#[cfg(feature = "std")]
mod use_std {
    use super::*;

    use std::io;

    /// Describe the ability to deserialize the type from sets of bytes.
    pub trait Deserializable: Sized {
        /// Deserialization from variable length slices of bytes.
        fn from_bytes(bytes: &[u8]) -> io::Result<Self>;
    }

    impl<T> SerializableVec for T
    where
        T: SizedBytes + io::Read,
    {
        #[allow(clippy::unused_io_amount)]
        fn to_bytes(&mut self) -> Vec<u8> {
            let n = self.serialized_size();

            let mut bytes = vec![0u8; n];

            // Read return is not checked because it is already calculated with
            // `serialized_size` and any additional check is unnecessary
            self.read(bytes.as_mut_slice())
                .expect("Incorrect `SizedBytes` implementation!");

            bytes
        }
    }

    impl<T> Deserializable for T
    where
        T: Default + io::Write,
    {
        #[allow(clippy::unused_io_amount)]
        fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
            let mut instance = Self::default();

            // Write return is not checked because it is already calculated with
            // `serialized_size` and any additional check is unnecessary
            instance.write(bytes)?;

            Ok(instance)
        }
    }

    /// End of file error representation.
    pub fn eof() -> io::Error {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "The provided buffer is not big enough!",
        )
    }

    /// Attempt to store into the provided buffer the length of `bytes` as big-endian, and then
    /// the bytes itself. The `bytes` will be padded to be word-aligned.
    ///
    /// If the buffer is big enough to store length+bytes, will return the amount of bytes written
    /// and the remainder of the buffer. Return [`std::io::Error`] otherwise.
    pub fn store_bytes<'a>(
        mut buf: &'a mut [u8],
        bytes: &[u8],
    ) -> io::Result<(usize, &'a mut [u8])> {
        let len = (bytes.len() as Word).to_be_bytes();
        let pad = bytes.len() % WORD_SIZE;
        let pad = if pad == 0 { 0 } else { WORD_SIZE - pad };
        if buf.len() < WORD_SIZE + bytes.len() + pad {
            return Err(eof());
        }

        buf[..WORD_SIZE].copy_from_slice(&len);
        buf = &mut buf[WORD_SIZE..];

        buf[..bytes.len()].copy_from_slice(bytes);
        buf = &mut buf[bytes.len()..];

        for i in &mut buf[..pad] {
            *i = 0
        }
        buf = &mut buf[pad..];

        Ok((WORD_SIZE + bytes.len() + pad, buf))
    }

    /// Attempt to store into the provided buffer the provided bytes. They will be padded to be
    /// word-aligned.
    ///
    /// If the buffer is big enough to store the padded bytes, will return the amount of bytes
    /// written and the remainder of the buffer. Return [`std::io::Error`] otherwise.
    pub fn store_raw_bytes<'a>(
        mut buf: &'a mut [u8],
        bytes: &[u8],
    ) -> io::Result<(usize, &'a mut [u8])> {
        let pad = bytes.len() % WORD_SIZE;
        let pad = if pad == 0 { 0 } else { WORD_SIZE - pad };
        if buf.len() < bytes.len() + pad {
            return Err(eof());
        }

        buf[..bytes.len()].copy_from_slice(bytes);
        buf = &mut buf[bytes.len()..];

        for i in &mut buf[..pad] {
            *i = 0
        }
        buf = &mut buf[pad..];

        Ok((bytes.len() + pad, buf))
    }

    /// Attempt to restore a variable size bytes from a buffer.
    ///
    /// Will read the length, the bytes amount (word-aligned), and return the remainder buffer.
    pub fn restore_bytes(mut buf: &[u8]) -> io::Result<(usize, Vec<u8>, &[u8])> {
        // Safety: chunks_exact will guarantee the size of the slice is correct
        let len = buf
            .chunks_exact(WORD_SIZE)
            .next()
            .map(|b| unsafe { from_slice_unchecked(b) })
            .map(|len| Word::from_be_bytes(len) as usize)
            .ok_or_else(eof)?;

        buf = &buf[WORD_SIZE..];

        let pad = len % WORD_SIZE;
        let pad = if pad == 0 { 0 } else { WORD_SIZE - pad };
        if buf.len() < len + pad {
            return Err(eof());
        }

        let data = Vec::from(&buf[..len]);
        let buf = &buf[len + pad..];

        Ok((WORD_SIZE + len + pad, data, buf))
    }

    /// Attempt to restore a variable size bytes with the length specified as argument.
    pub fn restore_raw_bytes(buf: &[u8], len: usize) -> io::Result<(usize, Vec<u8>, &[u8])> {
        let pad = len % WORD_SIZE;
        let pad = if pad == 0 { 0 } else { WORD_SIZE - pad };
        if buf.len() < len + pad {
            return Err(eof());
        }

        let data = Vec::from(&buf[..len]);
        let buf = &buf[len + pad..];

        Ok((len + pad, data, buf))
    }

    /// Convert a number to a word, store it in the buffer, and return a tuple containing the
    /// number of written bytes and the remainder of the buffer.
    pub fn store_number<T>(buf: &mut [u8], number: T) -> io::Result<(usize, &mut [u8])>
    where
        T: Into<Word>,
    {
        buf.chunks_exact_mut(WORD_SIZE)
            .next()
            .map(|chunk| chunk.copy_from_slice(&number.into().to_be_bytes()))
            .ok_or_else(eof)?;

        Ok((WORD_SIZE, &mut buf[WORD_SIZE..]))
    }

    /// Attempt to read a word-aligned number stored in the buffer.
    ///
    /// Return the readon number and the remainder buffer, if successful.
    pub fn restore_number<T>(buf: &[u8]) -> io::Result<(T, &[u8])>
    where
        T: From<Word>,
    {
        // Safe checked memory bounds
        let number = buf
            .chunks_exact(WORD_SIZE)
            .next()
            .map(|b| unsafe { from_slice_unchecked(b) })
            .map(|chunk| Word::from_be_bytes(chunk).into())
            .ok_or_else(eof)?;

        Ok((number, &buf[WORD_SIZE..]))
    }

    /// Store a statically sized array into a buffer, returning the remainder of the buffer.
    pub fn store_array<'a, const N: usize>(
        buf: &'a mut [u8],
        array: &[u8; N],
    ) -> io::Result<&'a mut [u8]> {
        buf.chunks_exact_mut(N)
            .next()
            .map(|chunk| chunk.copy_from_slice(array))
            .ok_or_else(eof)?;

        Ok(&mut buf[N..])
    }

    /// Restore a statically sized array from a buffer, returning the array and the remainder of
    /// the buffer.
    pub fn restore_array<const N: usize>(buf: &[u8]) -> io::Result<([u8; N], &[u8])> {
        <[u8; N]>::try_from(&buf[..N])
            .map_err(|_| eof())
            .map(|array| (array, &buf[N..]))
    }
}
