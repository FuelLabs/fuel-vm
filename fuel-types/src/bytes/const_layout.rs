use core::ops::Index;
use core::ops::IndexMut;
use core::ops::Range;

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
