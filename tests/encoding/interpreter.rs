use super::assert_encoding_correct;
use super::common::r;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;

#[test]
fn call() {
    assert_encoding_correct(
        (0..10)
            .map(|_| Call::new(r(), r(), r()))
            .collect::<Vec<Call>>()
            .as_slice(),
    );
}

#[test]
fn call_frame() {
    assert_encoding_correct(
        (0..10)
            .map(|_| CallFrame::new(r(), r(), [r(); VM_REGISTER_COUNT], r(), r(), vec![r(); 200].into()))
            .collect::<Vec<CallFrame>>()
            .as_slice(),
    );
}
