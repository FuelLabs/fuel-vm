use std::ops::Range;

use fuel_asm::PanicReason;
use fuel_types::Word;

use crate::consts::MEM_SIZE;

/// A range of memory, checked to be within the VM memory bounds upon construction.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryRange(Range<usize>);

impl MemoryRange {
    /// Returns `None` if the range doesn't fall within the VM memory.
    pub fn try_new(start: Word, len: Word) -> Result<Self, PanicReason> {
        let start: usize = start.try_into().map_err(|_| PanicReason::MemoryOverflow)?;
        let len: usize = len.try_into().map_err(|_| PanicReason::MemoryOverflow)?;
        Self::try_new_usize(start, len)
    }

    /// Returns `None` if the range doesn't fall within the VM memory.
    pub fn try_new_usize(start: usize, len: usize) -> Result<Self, PanicReason> {
        let end = start.checked_add(len).ok_or(PanicReason::MemoryOverflow)?;

        if end > MEM_SIZE {
            return Err(PanicReason::MemoryOverflow);
        }

        Ok(Self(start..end))
    }

    /// Converts this to a `usize` range. This is needed because `Range` doesn't implement `Copy`.
    pub fn as_usizes(&self) -> Range<usize> {
        self.0.clone()
    }

    /// Converts this to a `Word` range
    pub fn as_words(&self) -> Range<Word> {
        (self.start as Word)..(self.end as Word)
    }

    /// Checks if a range falls fully within another range
    pub fn contains_range(&self, inner: &Self) -> bool {
        self.contains(&inner.start) && inner.end <= self.end
    }

    /// Computes the overlap (intersection) of two ranges. Returns `None` if the ranges do not overlap.
    pub fn overlap_with(&self, other: &Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(Self(start..end))
        } else {
            None
        }
    }

    /// Checks that a range is fully contained within another range, and then returns
    /// the self as offset relative to the outer range.
    pub fn relative_to(&self, outer: &Self) -> Option<Self> {
        if outer.contains_range(self) {
            Some(Self(self.start - outer.start..self.end - outer.start))
        } else {
            None
        }
    }

    /// Shrink the into the given subrange.
    /// Returns `None` if the subrange is not fully contained within the range.
    pub fn subrange(&self, offset: usize, len: usize) -> Option<Self> {
        let new_start = self.start.checked_add(offset)?;
        let new_end = new_start.checked_add(len)?;

        if new_end > self.end {
            return None;
        }

        Some(Self(new_start..new_end))
    }

    /// Splits the range into two ranges at the given offset.
    /// Returns `None` if the offset is not within the range.
    pub fn split_at(&self, offset: usize) -> Option<(Self, Self)> {
        let offset = self.start.checked_add(offset)?;

        if offset > self.end {
            return None;
        }

        Some((Self(self.start..offset), Self(offset..self.end)))
    }
}

impl std::ops::Deref for MemoryRange {
    type Target = Range<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
pub mod tests {
    use crate::consts::{MEM_SIZE, VM_MAX_RAM};

    use super::MemoryRange;

    #[test]
    fn test_init() {
        assert_eq!(MemoryRange::try_new(0, 0).unwrap().as_usizes(), 0..0);
        assert_eq!(MemoryRange::try_new(0, 1).unwrap().as_usizes(), 0..1);
        assert_eq!(MemoryRange::try_new(0, 2).unwrap().as_usizes(), 0..2);
        assert_eq!(MemoryRange::try_new(1, 1).unwrap().as_usizes(), 1..2);
        assert_eq!(MemoryRange::try_new(1, 2).unwrap().as_usizes(), 1..3);
        assert_eq!(MemoryRange::try_new(2, 2).unwrap().as_usizes(), 2..4);
        assert_eq!(MemoryRange::try_new(0, VM_MAX_RAM).unwrap().as_usizes(), 0..MEM_SIZE);
        assert_eq!(
            MemoryRange::try_new(1, VM_MAX_RAM - 1).unwrap().as_usizes(),
            1..MEM_SIZE
        );
        assert_eq!(
            MemoryRange::try_new(VM_MAX_RAM - 1, 0).unwrap().as_usizes(),
            (MEM_SIZE - 1)..(MEM_SIZE - 1)
        );
        assert_eq!(
            MemoryRange::try_new(VM_MAX_RAM - 1, 1).unwrap().as_usizes(),
            (MEM_SIZE - 1)..MEM_SIZE
        );
        assert_eq!(
            MemoryRange::try_new(VM_MAX_RAM, 0).unwrap().as_usizes(),
            MEM_SIZE..MEM_SIZE
        );

        assert!(MemoryRange::try_new(0, VM_MAX_RAM + 1).is_err());
        assert!(MemoryRange::try_new(1, VM_MAX_RAM).is_err());
        assert!(MemoryRange::try_new(2, VM_MAX_RAM).is_err());
        assert!(MemoryRange::try_new(VM_MAX_RAM, VM_MAX_RAM).is_err());
        assert!(MemoryRange::try_new(VM_MAX_RAM, 1).is_err());
        assert!(MemoryRange::try_new(0, u64::MAX).is_err());
        assert!(MemoryRange::try_new(u64::MAX, 0).is_err());
        assert!(MemoryRange::try_new(u64::MAX, u64::MAX).is_err());

        assert!(MemoryRange::try_new_usize(0, usize::MAX).is_err());
        assert!(MemoryRange::try_new_usize(usize::MAX, 0).is_err());
        assert!(MemoryRange::try_new_usize(usize::MAX, usize::MAX).is_err());
    }

    #[test]
    fn test_contains_range() {
        let a = MemoryRange::try_new(0, 10).unwrap();
        let b = MemoryRange::try_new(2, 4).unwrap();
        let c = MemoryRange::try_new(8, 4).unwrap();
        let d = MemoryRange::try_new(10, 10).unwrap();
        let e = MemoryRange::try_new(20, 10).unwrap();

        assert!(a.contains_range(&b));
        assert!(!a.contains_range(&c));
        assert!(!a.contains_range(&d));
        assert!(!a.contains_range(&e));
    }

    #[test]
    fn test_overlap() {
        let a = MemoryRange::try_new(0, 10).unwrap();
        let b = MemoryRange::try_new(5, 10).unwrap();
        let c = MemoryRange::try_new(10, 10).unwrap();

        assert_eq!(a.overlap_with(&b), Some(MemoryRange::try_new(5, 5).unwrap()));
        assert_eq!(a.overlap_with(&c), None);
        assert_eq!(b.overlap_with(&c), Some(MemoryRange::try_new(10, 5).unwrap()));
    }

    #[test]
    fn test_relative_to() {
        let a = MemoryRange::try_new(100, 100).unwrap();
        let b = MemoryRange::try_new(120, 10).unwrap();
        let c = MemoryRange::try_new(195, 10).unwrap();
        let d = MemoryRange::try_new(190, 10).unwrap();
        let e = MemoryRange::try_new(190, 11).unwrap();

        assert_eq!(a.relative_to(&b), None);
        assert_eq!(b.relative_to(&a), Some(MemoryRange::try_new(20, 10).unwrap()));
        assert_eq!(c.relative_to(&a), None);
        assert_eq!(a.relative_to(&b), None);
        assert_eq!(d.relative_to(&a), Some(MemoryRange::try_new(90, 10).unwrap()));
        assert_eq!(e.relative_to(&a), None);
    }

    #[test]
    fn test_subrange() {
        let a = MemoryRange::try_new(100, 100).unwrap();

        assert_eq!(a.subrange(0, 0).unwrap().as_usizes(), 100..100);
        assert_eq!(a.subrange(0, 100).unwrap().as_usizes(), 100..200);
        assert_eq!(a.subrange(10, 20).unwrap().as_usizes(), 110..130);
        assert_eq!(a.subrange(10, 90).unwrap().as_usizes(), 110..200);
        assert!(a.subrange(10, 91).is_none());
        assert!(a.subrange(0, usize::MAX).is_none());
        assert!(a.subrange(usize::MAX, 0).is_none());
        assert!(a.subrange(usize::MAX, usize::MAX).is_none());
    }

    #[test]
    fn test_split_at() {
        let a = MemoryRange::try_new(100, 100).unwrap();

        assert_eq!(
            a.split_at(0),
            Some((
                MemoryRange::try_new(100, 0).unwrap(),
                MemoryRange::try_new(100, 100).unwrap(),
            ))
        );
        assert_eq!(
            a.split_at(1),
            Some((
                MemoryRange::try_new(100, 1).unwrap(),
                MemoryRange::try_new(101, 99).unwrap(),
            ))
        );
        assert_eq!(
            a.split_at(50),
            Some((
                MemoryRange::try_new(100, 50).unwrap(),
                MemoryRange::try_new(150, 50).unwrap(),
            ))
        );
        assert_eq!(
            a.split_at(99),
            Some((
                MemoryRange::try_new(100, 99).unwrap(),
                MemoryRange::try_new(199, 1).unwrap(),
            ))
        );
        assert_eq!(
            a.split_at(100),
            Some((
                MemoryRange::try_new(100, 100).unwrap(),
                MemoryRange::try_new(200, 0).unwrap(),
            ))
        );
        assert_eq!(a.split_at(101), None);
        assert_eq!(a.split_at(MEM_SIZE), None);
        assert_eq!(a.split_at(usize::MAX), None);
    }
}
