#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::vec::Vec;

use super::*;

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
