use crate::{LayoutType, MemLoc, MemLocType, Word};
use core::borrow::Borrow;
use core::ops::Index;
use core::ops::IndexMut;
use core::ops::Range;

/// Memory size of a [`Word`]
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

/// Compile time assert that the array is big enough to contain the memory location.
const fn check_range_bounds<const ARR: usize, const ADDR: usize, const SIZE: usize>() -> Range<usize> {
    assert!(
        ARR >= ADDR + SIZE,
        "ARR length must be greater than or equal to sub arrays ADDR + SIZE"
    );
    ADDR..ADDR + SIZE
}

/// A trait to get safe sub-arrays from a larger array.
pub(super) trait SubArray<const ARR: usize, const ADDR: usize, const SIZE: usize>:
    Index<Range<usize>, Output = [u8]>
{
    /// Compile-time checked range of the sub-array.
    const RANGE: Range<usize> = check_range_bounds::<ARR, ADDR, SIZE>();

    /// Creates a new sub-array from the parent array where the size is checked at compile time.
    fn sub_array(&self) -> [u8; SIZE] {
        self[Self::RANGE]
            .try_into()
            .expect("This can't ever fail due to the compile-time check")
    }

    /// Creates a new fixed size slice from the parent array where the size is checked at compile time.
    fn sized_slice(&self) -> &[u8; SIZE] {
        (&self[Self::RANGE])
            .try_into()
            .expect("This can't ever fail due to the compile-time check")
    }
}

/// A trait to get safe mutable sub-arrays from a larger array.
pub(super) trait SubArrayMut<const ARR: usize, const ADDR: usize, const SIZE: usize>:
    SubArray<ARR, ADDR, SIZE> + IndexMut<Range<usize>>
{
    /// Creates a new mutable sub-array from the parent array where the size is checked at compile time.
    fn sized_slice_mut(&mut self) -> &mut [u8; SIZE] {
        (&mut self[Self::RANGE])
            .try_into()
            .expect("This can't ever fail due to the compile-time check")
    }
}

impl<const ARR: usize, const ADDR: usize, const SIZE: usize> SubArray<ARR, ADDR, SIZE> for [u8; ARR] {}
impl<const ARR: usize, const ADDR: usize, const SIZE: usize> SubArrayMut<ARR, ADDR, SIZE> for [u8; ARR] {}

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

/// Read a word-padded u8 from a specific location in a buffer.
/// Won't compile if the buffer is too small.
pub fn restore_u8_at<const ARR: usize, const ADDR: usize, T>(buf: &[u8; ARR], loc: LayoutType<ADDR, WORD_SIZE, T>) -> u8
where
    T: MemLocType<ADDR, WORD_SIZE, Type = u8>,
{
    Word::from_be_bytes(from_loc(loc.loc(), buf)) as u8
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
