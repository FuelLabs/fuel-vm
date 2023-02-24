use core::borrow::Borrow;

use crate::LayoutType;
use crate::MemLoc;
use crate::MemLocType;
use crate::Word;

#[cfg(feature = "std")]
pub use use_std::*;

#[cfg(feature = "alloc")]
pub use use_alloc::*;

use const_layout::*;

mod const_layout;

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

/// Store a number into this buffer.
pub fn store_number<T>(buf: &mut [u8; WORD_SIZE], number: T)
where
    T: Into<Word>,
{
    buf.copy_from_slice(&number.into().to_be_bytes());
}

/// Store a number at a specific location in this buffer.
pub fn store_number_at<const ARR: usize, const ADDR: usize, const SIZE: usize, T>(
    buf: &mut [u8; ARR],
    layout: LayoutType<ADDR, SIZE, T>,
    number: T::Type,
) where
    T: MemLocType<ADDR, SIZE>,
    <T as MemLocType<ADDR, SIZE>>::Type: Into<Word>,
{
    from_loc_mut(layout.loc(), buf).copy_from_slice(&number.into().to_be_bytes());
}

#[cfg(feature = "unsafe")]
/// Read the initial bytes of a buffer to fetch a word.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// Extends the safety properties of [`from_slice_unchecked`]
///
/// # Panics
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

#[cfg(feature = "unsafe")]
/// Read the initial bytes of a buffer to fetch a word.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// Extends the safety properties of [`from_slice_unchecked`]
///
/// # Panics
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_word_unchecked(buf: &[u8]) -> (Word, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number);

    (number, &buf[WORD_SIZE..])
}

/// Read a number from a buffer.
pub fn restore_number<T>(buf: [u8; WORD_SIZE]) -> T
where
    T: From<Word>,
{
    Word::from_be_bytes(buf).into()
}

/// Read a number from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_number_at<const ARR: usize, const ADDR: usize, T>(
    buf: &[u8; ARR],
    loc: LayoutType<ADDR, WORD_SIZE, T>,
) -> T::Type
where
    T: MemLocType<ADDR, WORD_SIZE>,
    Word: Into<<T as MemLocType<ADDR, WORD_SIZE>>::Type>,
{
    Word::from_be_bytes(from_loc(loc.loc(), buf)).into()
}

/// Read a word from a buffer.
pub fn restore_word(buf: [u8; WORD_SIZE]) -> Word {
    Word::from_be_bytes(buf)
}

/// Read a word from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_word_at<const ARR: usize, const ADDR: usize, T>(
    buf: &[u8; ARR],
    loc: LayoutType<ADDR, WORD_SIZE, T>,
) -> Word
where
    T: MemLocType<ADDR, WORD_SIZE, Type = Word>,
{
    Word::from_be_bytes(from_loc(loc.loc(), buf))
}

#[cfg(feature = "unsafe")]
/// Read the a word-padded u8 from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// Extends the safety properties of [`from_slice_unchecked`]
///
/// # Panics
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_u8_unchecked(buf: &[u8]) -> (u8, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u8;

    (number, &buf[WORD_SIZE..])
}

/// Read a word-padded u8 from a buffer.
pub fn restore_u8(buf: [u8; WORD_SIZE]) -> u8 {
    Word::from_be_bytes(buf) as u8
}

/// Read a word-padded u8 from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_u8_at<const ARR: usize, const ADDR: usize, T>(buf: &[u8; ARR], loc: LayoutType<ADDR, WORD_SIZE, T>) -> u8
where
    T: MemLocType<ADDR, WORD_SIZE, Type = u8>,
{
    Word::from_be_bytes(from_loc(loc.loc(), buf)) as u8
}

#[cfg(feature = "unsafe")]
/// Read the a word-padded u16 from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// Extends the safety properties of [`from_slice_unchecked`]
///
/// # Panics
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_u16_unchecked(buf: &[u8]) -> (u16, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u16;

    (number, &buf[WORD_SIZE..])
}

/// Read the a word-padded u16 from a buffer.
pub fn restore_u16(buf: [u8; WORD_SIZE]) -> u16 {
    Word::from_be_bytes(buf) as u16
}

/// Read the a word-padded u16 from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_u16_at<const ARR: usize, const ADDR: usize, T>(
    buf: &[u8; ARR],
    loc: LayoutType<ADDR, WORD_SIZE, T>,
) -> u16
where
    T: MemLocType<ADDR, WORD_SIZE, Type = u16>,
{
    Word::from_be_bytes(from_loc(loc.loc(), buf)) as u16
}

#[cfg(feature = "unsafe")]
/// Read the a word-padded u32 from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// Extends the safety properties of [`from_slice_unchecked`]
///
/// # Panics
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_u32_unchecked(buf: &[u8]) -> (u32, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as u32;

    (number, &buf[WORD_SIZE..])
}

/// Read the a word-padded u32 from a buffer.
pub fn restore_u32(buf: [u8; WORD_SIZE]) -> u32 {
    Word::from_be_bytes(buf) as u32
}

/// Read the a word-padded u32 from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_u32_at<const ARR: usize, const ADDR: usize, T>(
    buf: &[u8; ARR],
    loc: LayoutType<ADDR, WORD_SIZE, T>,
) -> u32
where
    T: MemLocType<ADDR, WORD_SIZE, Type = u32>,
{
    Word::from_be_bytes(from_loc(loc.loc(), buf)) as u32
}

#[cfg(feature = "unsafe")]
/// Read the a word-padded usize from a buffer.
///
/// Return the read word and the remainder of the buffer
///
/// # Safety
///
/// Extends the safety properties of [`from_slice_unchecked`]
///
/// # Panics
///
/// This function will panic if the length of the buffer is smaller than a word
pub unsafe fn restore_usize_unchecked(buf: &[u8]) -> (usize, &[u8]) {
    let number = from_slice_unchecked(buf);
    let number = Word::from_be_bytes(number) as usize;

    (number, &buf[WORD_SIZE..])
}

/// Read the a word-padded usize from a buffer.
pub fn restore_usize(buf: [u8; WORD_SIZE]) -> usize {
    Word::from_be_bytes(buf) as usize
}

/// Read the a word-padded usize from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_usize_at<const ARR: usize, const ADDR: usize, T>(
    buf: &[u8; ARR],
    loc: LayoutType<ADDR, WORD_SIZE, T>,
) -> usize
where
    T: MemLocType<ADDR, WORD_SIZE, Type = Word>,
{
    Word::from_be_bytes(from_loc(loc.loc(), buf)) as usize
}

/// Store an array at a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn store_at<const ARR: usize, const ADDR: usize, const SIZE: usize, T>(
    buf: &mut [u8; ARR],
    layout: LayoutType<ADDR, SIZE, T>,
    array: &[u8; SIZE],
) where
    T: MemLocType<ADDR, SIZE>,
    <T as MemLocType<ADDR, SIZE>>::Type: Borrow<[u8; SIZE]>,
{
    from_loc_mut(layout.loc(), buf).copy_from_slice(array);
}

#[cfg(feature = "unsafe")]
/// Read an array of `N` bytes from `buf`.
///
/// Return the read array and the remainder bytes.
///
/// # Safety
///
/// Extends the safety properties of [`from_slice_unchecked`]
///
/// # Panics
///
/// This function will panic if the length of `buf` is smaller than `N`
pub unsafe fn restore_array_unchecked<const N: usize>(buf: &[u8]) -> ([u8; N], &[u8]) {
    (from_slice_unchecked(buf), &buf[N..])
}

/// Read an array of `N` bytes from `buf`.
///
/// # Panics
///
/// This function will panic if the length of `buf` is smaller than `N`
pub fn restore_array_from_slice<const N: usize>(buf: &[u8]) -> [u8; N] {
    buf.try_into().expect("buf must be at least N bytes long")
}

/// Restore an array from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_at<const ARR: usize, const ADDR: usize, const SIZE: usize, T>(
    buf: &[u8; ARR],
    loc: LayoutType<ADDR, SIZE, T>,
) -> [u8; SIZE]
where
    T: MemLocType<ADDR, SIZE>,
    [u8; SIZE]: From<<T as MemLocType<ADDR, SIZE>>::Type>,
{
    from_loc(loc.loc(), buf)
}

#[cfg(feature = "unsafe")]
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

/// Get an array from a fixed sized slice.
/// Won't compile if the buffer is not large enough.
/// ```
/// # use fuel_types::bytes::from_array;
/// let mem = [0u8; 2];
/// let _: [u8; 2] = from_array(&mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_array;
/// let mem = [0u8; 1];
/// let _: [u8; 2] = from_array(&mem);
/// ```
pub fn from_array<const ARR: usize, const SIZE: usize>(buf: &[u8; ARR]) -> [u8; SIZE] {
    SubArray::<ARR, 0, SIZE>::sub_array(buf)
}

/// Get an array from a specific location in a fixed sized slice.
/// This won't compile if the buffer is not large enough.
/// ```
/// # use fuel_types::bytes::from_loc;
/// # use fuel_types::MemLoc;
/// let mem = [0u8; 32];
/// let _: [u8; 2] = from_loc(MemLoc::<1, 2>::new(), &mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_loc;
/// # use fuel_types::MemLoc;
/// let mem = [0u8; 32];
/// let _: [u8; 2] = from_loc(MemLoc::<31, 2>::new(), &mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_loc;
/// # use fuel_types::MemLoc;
/// let mem = [0u8; 32];
/// let _: [u8; 2] = from_loc(MemLoc::<34, 2>::new(), &mem);
/// ```
pub fn from_loc<const ARR: usize, const ADDR: usize, const SIZE: usize>(
    // MemLoc is a zero sized type that makes setting the const generic parameter easier.
    _layout: MemLoc<ADDR, SIZE>,
    buf: &[u8; ARR],
) -> [u8; SIZE] {
    SubArray::<ARR, ADDR, SIZE>::sub_array(buf)
}

/// Get a fixed sized slice from a fixed sized slice.
/// Won't compile if the buffer is not large enough.
/// ```
/// # use fuel_types::bytes::from_array_ref;
/// let mem = [0u8; 2];
/// let _: &[u8; 2] = from_array_ref(&mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_array_ref;
/// let mem = [0u8; 1];
/// let _: &[u8; 2] = from_array_ref(&mem);
/// ```
pub fn from_array_ref<const ARR: usize, const SIZE: usize>(buf: &[u8; ARR]) -> &[u8; SIZE] {
    SubArray::<ARR, 0, SIZE>::sized_slice(buf)
}

/// Get a fixed sized mutable slice from a fixed sized slice.
/// Won't compile if the buffer is not large enough.
/// ```
/// # use fuel_types::bytes::from_array_mut;
/// let mut mem = [0u8; 2];
/// let _: &mut [u8; 2] = from_array_mut(&mut mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_array_mut;
/// let mem = [0u8; 1];
/// let _: &mut [u8; 2] = from_array_mut(&mut mem);
/// ```
pub fn from_array_mut<const ARR: usize, const SIZE: usize>(buf: &mut [u8; ARR]) -> &mut [u8; SIZE] {
    SubArrayMut::<ARR, 0, SIZE>::sized_slice_mut(buf)
}

/// Get a fixed sized slice from a specific location in a fixed sized slice.
/// Won't compile if the buffer is not large enough.
/// ```
/// # use fuel_types::bytes::from_loc_ref;
/// # use fuel_types::MemLoc;
/// let mem = [0u8; 32];
/// let _: &[u8; 2] = from_loc_ref(MemLoc::<1, 2>::new(), &mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_loc_ref;
/// # use fuel_types::MemLoc;
/// let mem = [0u8; 32];
/// let _: &[u8; 2] = from_loc_ref(MemLoc::<31, 2>::new(), &mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_loc_ref;
/// # use fuel_types::MemLoc;
/// let mem = [0u8; 32];
/// let _: &[u8; 2] = from_loc_ref(MemLoc::<34, 2>::new(), &mem);
/// ```
pub fn from_loc_ref<const ARR: usize, const ADDR: usize, const SIZE: usize>(
    // MemLoc is a zero sized type that makes setting the const generic parameter easier.
    _layout: MemLoc<ADDR, SIZE>,
    buf: &[u8; ARR],
) -> &[u8; SIZE] {
    SubArray::<ARR, ADDR, SIZE>::sized_slice(buf)
}

/// Get a fixed sized mutable slice from a specific location in a fixed sized slice.
/// Won't compile if the buffer is not large enough.
/// ```
/// # use fuel_types::bytes::from_loc_mut;
/// # use fuel_types::MemLoc;
/// let mut mem = [0u8; 32];
/// let _: &mut [u8; 2] = from_loc_mut(MemLoc::<1, 2>::new(), &mut mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_loc_mut;
/// # use fuel_types::MemLoc;
/// let mut mem = [0u8; 32];
/// let _: &mut [u8; 2] = from_loc_mut(MemLoc::<31, 2>::new(), &mut mem);
/// ```
/// ```compile_fail
/// # use fuel_types::bytes::from_loc_mut;
/// # use fuel_types::MemLoc;
/// let mut mem = [0u8; 32];
/// let _: &mut [u8; 2] = from_loc_mut(MemLoc::<34, 2>::new(), &mut mem);
/// ```
pub fn from_loc_mut<const ARR: usize, const ADDR: usize, const SIZE: usize>(
    // MemLoc is a zero sized type that makes setting the const generic parameter easier.
    _layout: MemLoc<ADDR, SIZE>,
    buf: &mut [u8; ARR],
) -> &mut [u8; SIZE] {
    SubArrayMut::<ARR, ADDR, SIZE>::sized_slice_mut(buf)
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
        io::Error::new(io::ErrorKind::UnexpectedEof, "The provided buffer is not big enough!")
    }

    /// Attempt to store into the provided buffer the length of `bytes` as big-endian, and then
    /// the bytes itself. The `bytes` will be padded to be word-aligned.
    ///
    /// If the buffer is big enough to store length+bytes, will return the amount of bytes written
    /// and the remainder of the buffer. Return [`std::io::Error`] otherwise.
    pub fn store_bytes<'a>(mut buf: &'a mut [u8], bytes: &[u8]) -> io::Result<(usize, &'a mut [u8])> {
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
    pub fn store_raw_bytes<'a>(mut buf: &'a mut [u8], bytes: &[u8]) -> io::Result<(usize, &'a mut [u8])> {
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
            .and_then(|b| b.try_into().ok())
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

    /// Store a statically sized array into a buffer, returning the remainder of the buffer.
    pub fn store_array<'a, const N: usize>(buf: &'a mut [u8], array: &[u8; N]) -> io::Result<&'a mut [u8]> {
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
