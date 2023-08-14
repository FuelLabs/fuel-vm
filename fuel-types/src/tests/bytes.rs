use fuel_types::{
    bytes::{
        self,
        WORD_SIZE,
    },
    mem_layout,
    MemLoc,
    MemLocType,
    Word,
};

#[test]
#[allow(clippy::erasing_op)]
#[allow(clippy::identity_op)]
fn padded_len_to_fit_word_len() {
    assert_eq!(WORD_SIZE * 0, bytes::padded_len(&[]));
    assert_eq!(WORD_SIZE * 1, bytes::padded_len(&[0]));
    assert_eq!(WORD_SIZE * 1, bytes::padded_len(&[0; WORD_SIZE]));
    assert_eq!(WORD_SIZE * 2, bytes::padded_len(&[0; WORD_SIZE + 1]));
    assert_eq!(WORD_SIZE * 2, bytes::padded_len(&[0; WORD_SIZE * 2]));
}

#[test]
fn store_restore_number_works() {
    let mut buf = [0u8; 255];
    struct Foo;
    impl MemLocType<0, WORD_SIZE> for Foo {
        type Type = Word;
    }
    bytes::store_number_at(
        &mut buf,
        Foo::layout(MemLoc::<0, WORD_SIZE>::new()),
        65 as Word,
    );
    assert_eq!(
        bytes::restore_usize_at(&buf, Foo::layout(MemLoc::<0, WORD_SIZE>::new())),
        65
    );
    assert_eq!(
        bytes::restore_word_at(&buf, Foo::layout(MemLoc::<0, WORD_SIZE>::new())),
        65
    );

    impl MemLocType<1, WORD_SIZE> for Foo {
        type Type = u8;
    }
    bytes::store_number_at(&mut buf, Foo::layout(MemLoc::<1, WORD_SIZE>::new()), 63u8);
    assert_eq!(
        bytes::restore_u8_at(&buf, Foo::layout(MemLoc::<1, WORD_SIZE>::new())),
        63
    );

    impl MemLocType<2, WORD_SIZE> for Foo {
        type Type = u16;
    }
    bytes::store_number_at(&mut buf, Foo::layout(MemLoc::<2, WORD_SIZE>::new()), 3u16);
    assert_eq!(
        bytes::restore_u16_at(&buf, Foo::layout(MemLoc::<2, WORD_SIZE>::new())),
        3
    );
    impl MemLocType<3, WORD_SIZE> for Foo {
        type Type = u32;
    }
    bytes::store_number_at(&mut buf, Foo::layout(MemLoc::<3, WORD_SIZE>::new()), 4u32);
    assert_eq!(
        bytes::restore_u32_at(&buf, Foo::layout(MemLoc::<3, WORD_SIZE>::new())),
        4
    );
}

#[test]
fn test_from_array() {
    let mut mem = [0u8; 1];
    let r: [u8; 1] = bytes::from_array(&mem);
    assert_eq!(r, [0]);
    let r: &[u8; 1] = bytes::from_array_ref(&mem);
    assert_eq!(r, &[0]);
    let r: &mut [u8; 1] = bytes::from_array_mut(&mut mem);
    assert_eq!(r, &mut [0]);
    let _: [u8; 0] = bytes::from_array(&mem);

    let mut mem = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let arr: [_; 1] = bytes::from_array(&mem);
    assert_eq!(arr, [0]);
    let arr: [_; 5] = bytes::from_array(&mem);
    assert_eq!(arr, [0, 1, 2, 3, 4]);
    let arr: [_; 10] = bytes::from_array(&mem);
    assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let slice: &[_; 1] = bytes::from_array_ref(&mem);
    assert_eq!(slice, &[0]);
    let slice: &[_; 5] = bytes::from_array_ref(&mem);
    assert_eq!(slice, &[0, 1, 2, 3, 4]);
    let slice: &[_; 10] = bytes::from_array_ref(&mem);
    assert_eq!(slice, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let slice: &mut [_; 1] = bytes::from_array_mut(&mut mem);
    assert_eq!(slice, &mut [0]);
    let slice: &mut [_; 5] = bytes::from_array_mut(&mut mem);
    assert_eq!(slice, &mut [0, 1, 2, 3, 4]);
    let slice: &mut [_; 10] = bytes::from_array_mut(&mut mem);
    assert_eq!(slice, &mut [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_from_loc() {
    let mut mem = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let r = bytes::from_loc(MemLoc::<3, 4>::new(), &mem);
    assert_eq!(r, [3, 4, 5, 6]);
    let r = bytes::from_loc(MemLoc::<0, 10>::new(), &mem);
    assert_eq!(r, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let r = bytes::from_loc_ref(MemLoc::<3, 4>::new(), &mem);
    assert_eq!(r, &[3, 4, 5, 6]);
    let r = bytes::from_loc_ref(MemLoc::<0, 10>::new(), &mem);
    assert_eq!(r, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let r = bytes::from_loc_mut(MemLoc::<3, 4>::new(), &mut mem);
    assert_eq!(r, &mut [3, 4, 5, 6]);
    let r = bytes::from_loc_mut(MemLoc::<0, 10>::new(), &mut mem);
    assert_eq!(r, &mut [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[derive(Debug, PartialEq, Eq)]
struct SomeType {
    a: u8,
    b: u16,
    c: u32,
    d: usize,
    e: Word,
    arr: [u8; 32],
    arr2: [u8; 64],
    bytes: Vec<u8>,
}

impl Default for SomeType {
    fn default() -> Self {
        Self {
            a: Default::default(),
            b: Default::default(),
            c: Default::default(),
            d: Default::default(),
            e: Default::default(),
            arr: Default::default(),
            arr2: [0u8; 64],
            bytes: Default::default(),
        }
    }
}

mem_layout!(SomeTypeLayout for SomeType
    a: u8 = WORD_SIZE,
    b: u16 = WORD_SIZE,
    c: u32 = WORD_SIZE,
    d: Word = WORD_SIZE,
    e: Word = WORD_SIZE,
    arr: [u8; 32] = 32,
    arr2: [u8; 64] = 64,
    bytes_size: Word = WORD_SIZE
);
