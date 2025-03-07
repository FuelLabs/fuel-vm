#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::vec::Vec;

use super::*;
use test_case::test_case;

#[test]
fn can_split() {
    let mut reg: [Word; VM_REGISTER_COUNT] =
        core::iter::successors(Some(0), |x| Some(x + 1))
            .take(VM_REGISTER_COUNT)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
    let expect = reg;

    let (r, w) = split_registers(&mut reg);

    let SystemRegisters {
        zero,
        one,
        of,
        pc,
        ssp,
        sp,
        fp,
        hp,
        err,
        ggas,
        cgas,
        bal,
        is,
        ret,
        retl,
        flag,
    } = &r;

    assert_eq!(*zero, RegMut::<ZERO>(&mut (ZERO as u64)));
    assert_eq!(*one, RegMut::<ONE>(&mut (ONE as u64)));
    assert_eq!(*of, RegMut::<OF>(&mut (OF as u64)));
    assert_eq!(*pc, RegMut::<PC>(&mut (PC as u64)));
    assert_eq!(*ssp, RegMut::<SSP>(&mut (SSP as u64)));
    assert_eq!(*sp, RegMut::<SP>(&mut (SP as u64)));
    assert_eq!(*fp, RegMut::<FP>(&mut (FP as u64)));
    assert_eq!(*hp, RegMut::<HP>(&mut (HP as u64)));
    assert_eq!(*err, RegMut::<ERR>(&mut (ERR as u64)));
    assert_eq!(*ggas, RegMut::<GGAS>(&mut (GGAS as u64)));
    assert_eq!(*cgas, RegMut::<CGAS>(&mut (CGAS as u64)));
    assert_eq!(*bal, RegMut::<BAL>(&mut (BAL as u64)));
    assert_eq!(*is, RegMut::<IS>(&mut (IS as u64)));
    assert_eq!(*ret, RegMut::<RET>(&mut (RET as u64)));
    assert_eq!(*retl, RegMut::<RETL>(&mut (RETL as u64)));
    assert_eq!(*flag, RegMut::<FLAG>(&mut (FLAG as u64)));

    for i in 0..VM_REGISTER_PROGRAM_COUNT {
        assert_eq!(w.0[i], i as u64 + 16);
    }

    let reg = copy_registers(&(&r).into(), &(&w).into());

    assert_eq!(reg, expect);
}

#[test_case(0, 1 => Some((0, 1)))]
#[test_case(0, 2 => Some((0, 2)))]
#[test_case(1, 3 => Some((1, 3)))]
#[test_case(2, 4 => Some((2, 4)))]
#[test_case(0, 0 => None)]
#[test_case(1, 1 => None)]
#[test_case(2, 2 => None)]
#[test_case(1, 0 => Some((1, 0)))]
#[test_case(2, 0 => Some((2, 0)))]
#[test_case(3, 1 => Some((3, 1)))]
#[test_case(4, 2 => Some((4, 2)))]
fn can_split_writes(a: usize, b: usize) -> Option<(Word, Word)> {
    let mut reg: [Word; VM_REGISTER_PROGRAM_COUNT] =
        core::iter::successors(Some(0), |x| Some(x + 1))
            .take(VM_REGISTER_PROGRAM_COUNT)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
    let s = VM_REGISTER_SYSTEM_COUNT;
    let mut reg = ProgramRegisters(&mut reg);
    reg.get_mut_two(
        WriteRegKey(RegId::new((s + a) as u8)),
        WriteRegKey(RegId::new((s + b) as u8)),
    )
    .map(|(a, b)| (*a, *b))
}
