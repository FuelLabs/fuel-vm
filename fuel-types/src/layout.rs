use core::marker::PhantomData;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A location in memory starting at `ADDR` and ending at `ADDR + SIZE`.
/// This is a zero-sized type to encode addresses that are known at compile time.
pub struct MemLoc<const ADDR: usize, const SIZE: usize>;

impl<const ADDR: usize, const SIZE: usize> MemLoc<ADDR, SIZE> {
    /// Creates a new memory location.
    pub const fn new() -> Self {
        Self
    }

    /// Returns the address of the memory location.
    pub const fn addr(&self) -> usize {
        ADDR
    }

    /// Returns the size of the memory location.
    pub const fn size(&self) -> usize {
        SIZE
    }

    /// Returns the range represented by the memory location.
    pub const fn range(&self) -> core::ops::Range<usize> {
        ADDR..ADDR + SIZE
    }
}

/// Trait that combines a memory location and a type.
pub trait MemLocType<const ADDR: usize, const SIZE: usize> {
    /// The type at this memory location.
    type Type;

    /// The memory locations address.
    const ADDR: usize = ADDR;

    /// The memory locations size.
    const SIZE: usize = SIZE;

    /// The memory location.
    const LOC: MemLoc<ADDR, SIZE> = MemLoc::new();

    /// Combine a memory location and a type.
    /// This will only work if this trait is defined for the memory location.
    fn layout(loc: MemLoc<ADDR, SIZE>) -> LayoutType<ADDR, SIZE, Self> {
        LayoutType(loc, PhantomData)
    }
}

/// A memory location combined with a type.
pub struct LayoutType<const ADDR: usize, const SIZE: usize, T>(
    MemLoc<ADDR, SIZE>,
    PhantomData<T>,
)
where
    T: MemLocType<ADDR, SIZE> + ?Sized;

impl<const ADDR: usize, const SIZE: usize, T> LayoutType<ADDR, SIZE, T>
where
    T: MemLocType<ADDR, SIZE>,
{
    /// Create a new layout type.
    pub const fn new() -> Self {
        Self(MemLoc::new(), PhantomData)
    }

    /// The memory location of this type.
    pub const fn loc(&self) -> MemLoc<ADDR, SIZE> {
        self.0
    }
}

/// Trait that defines a memory layout.
pub trait MemLayout {
    /// The associated memory layout type.
    type Type;
    /// A constant instance of the memory layout.
    const LAYOUT: Self::Type;
    /// The length of the memory layout.
    const LEN: usize;
}

#[macro_export]
/// Defines a memory layout for a type.
/// The address starts at 0 and is incremented by the size of each field.
/// The syntax is `field_name: field_type = field_size_in_bytes, ...`.
macro_rules! mem_layout {
    () => {};
    (@accum () -> ($s:ident for $o:ident $($f:ident: $t:ty = $si:expr, $a:expr);*) -> ($($addr:tt)*)) => {
        mem_layout!(@as_expr ($s $o $($f, $si, $t, $a)*) -> ($($addr)*));
    };
    (@accum ($field:ident: $t:ty = $size:expr, $($tail:tt)*) -> ($s:ident for $o:ident $($f:ident: $typ:ty = $si:expr, $a:expr);*) -> ($($addr:tt)*)) => {
        mem_layout!(@accum ($($tail)*) -> ($s for $o $($f: $typ = $si, $a);*; $field: $t = $size, $($addr)*) -> ($($addr)* + $size ));

    };
    (@as_expr ($s:ident $o:ident $($field:ident, $size:expr, $t:ty, $addr:expr)+) -> ($($len:tt)*)) => {
        #[derive(Debug, Default)]
        #[allow(missing_docs)]
        pub struct $s {
            $(pub $field: $crate::MemLoc<{$addr}, $size>,)+
        }
        impl $s {
            #[allow(missing_docs)]
            pub const fn new() -> Self {
                Self {
                    $($field: $crate::MemLoc::new(),)+
                }
            }
            #[allow(missing_docs)]
            pub const LEN: usize = $($len)*;
        }
        impl $crate::MemLayout for $o {
            type Type = $s;
            const LAYOUT: Self::Type = $s::new();
            const LEN: usize = Self::Type::LEN;
        }
        $(
            impl $crate::MemLocType<{ <$o as $crate::MemLayout>::LAYOUT.$field.addr() }, { <$o as $crate::MemLayout>::LAYOUT.$field.size() }> for $o {
                type Type = $t;
            }
        )+

    };
    ($s:ident for $o:ident $field:ident: $t:ty = $size:expr, $($tail:tt)*) => {
        mem_layout!(@accum ($($tail)*,) -> ($s for $o $field: $t = $size, 0) -> (0 + $size));
    };
}
