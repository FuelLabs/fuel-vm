use crate::Word;

const WORD_SIZE: usize = core::mem::size_of::<Word>();

pub trait SizedBytes {
    fn serialized_size(&self) -> usize;
}

pub const fn padded_len(bytes: &[u8]) -> usize {
    let pad = bytes.len() % WORD_SIZE;

    if pad == 0 {
        bytes.len()
    } else {
        bytes.len() + WORD_SIZE - pad
    }
}

pub fn store_number_unchecked<T>(buf: &mut [u8], number: T) -> &mut [u8]
where
    T: Into<Word>,
{
    buf[..WORD_SIZE].copy_from_slice(&number.into().to_be_bytes());

    &mut buf[WORD_SIZE..]
}

pub unsafe fn restore_number_unchecked<T>(buf: &[u8]) -> (T, &[u8])
where
    T: From<Word>,
{
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number).into();

    (number, &buf[WORD_SIZE..])
}

pub unsafe fn restore_word_unchecked(buf: &[u8]) -> (Word, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number);

    (number, &buf[WORD_SIZE..])
}

pub unsafe fn restore_u8_unchecked(buf: &[u8]) -> (u8, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u8;

    (number, &buf[WORD_SIZE..])
}

pub unsafe fn restore_u16_unchecked(buf: &[u8]) -> (u16, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u16;

    (number, &buf[WORD_SIZE..])
}

pub unsafe fn restore_u32_unchecked(buf: &[u8]) -> (u32, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u32;

    (number, &buf[WORD_SIZE..])
}

pub unsafe fn restore_usize_unchecked(buf: &[u8]) -> (usize, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as usize;

    (number, &buf[WORD_SIZE..])
}

pub fn store_array_unchecked<'a, const N: usize>(
    buf: &'a mut [u8],
    array: &[u8; N],
) -> &'a mut [u8] {
    buf[..N].copy_from_slice(array);

    &mut buf[N..]
}

pub unsafe fn restore_array_unchecked<const N: usize>(buf: &[u8]) -> ([u8; N], &[u8]) {
    (from_slice_unchecked(buf), &buf[N..])
}

/// Add a conversion from arbitrary slices into arrays
///
/// # Warning
///
/// This function will not panic if the length of the slice is smaller than `N`. Instead, it will
/// cause undefined behavior and read random disowned bytes.
pub unsafe fn from_slice_unchecked<const N: usize>(buf: &[u8]) -> [u8; N] {
    let ptr = buf.as_ptr() as *const [u8; N];

    // Static assertions are not applicable to runtime length check (e.g. slices).
    // This is safe if the size of `bytes` is consistent to `N`
    *ptr
}

#[cfg(feature = "std")]
pub use vec_io::*;

#[cfg(feature = "std")]
mod vec_io {
    use super::*;
    use std::convert::TryFrom;
    use std::io;

    pub trait SerializableVec: SizedBytes {
        fn to_bytes(&mut self) -> Vec<u8>;
    }

    pub trait Deserializable: Sized {
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

    pub fn eof() -> io::Error {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "The provided buffer is not big enough!",
        )
    }

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

    pub fn restore_array<const N: usize>(buf: &[u8]) -> io::Result<([u8; N], &[u8])> {
        <[u8; N]>::try_from(&buf[..N])
            .map_err(|_| eof())
            .map(|array| (array, &buf[N..]))
    }
}
