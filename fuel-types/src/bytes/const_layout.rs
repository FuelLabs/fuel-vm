use crate::{
    MemLoc,
    Word,
};
use core::ops::{
    Index,
    IndexMut,
    Range,
};

/// Memory size of a [`Word`]
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

/// Compile time assert that the array is big enough to contain the memory location.
const fn check_range_bounds<const ARR: usize, const ADDR: usize, const SIZE: usize>(
) -> Range<usize> {
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

    /// Creates a new sub-array from the parent array where the size is checked at compile
    /// time.
    fn sub_array(&self) -> [u8; SIZE] {
        self[Self::RANGE]
            .try_into()
            .expect("This can't ever fail due to the compile-time check")
    }

    /// Creates a new fixed size slice from the parent array where the size is checked at
    /// compile time.
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
    /// Creates a new mutable sub-array from the parent array where the size is checked at
    /// compile time.
    fn sized_slice_mut(&mut self) -> &mut [u8; SIZE] {
        (&mut self[Self::RANGE])
            .try_into()
            .expect("This can't ever fail due to the compile-time check")
    }
}

impl<const ARR: usize, const ADDR: usize, const SIZE: usize> SubArray<ARR, ADDR, SIZE>
    for [u8; ARR]
{
}
impl<const ARR: usize, const ADDR: usize, const SIZE: usize> SubArrayMut<ARR, ADDR, SIZE>
    for [u8; ARR]
{
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
pub fn from_array_ref<const ARR: usize, const SIZE: usize>(
    buf: &[u8; ARR],
) -> &[u8; SIZE] {
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
pub fn from_array_mut<const ARR: usize, const SIZE: usize>(
    buf: &mut [u8; ARR],
) -> &mut [u8; SIZE] {
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
