use fuel_types::{
    bytes::{
        self,
        WORD_SIZE,
    },
    MemLoc,
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
