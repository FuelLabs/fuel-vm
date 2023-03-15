use super::*;
use test_case::test_case;

#[test_case(0, 0, 0 => Ok(0); "noop jump")]
#[test_case(0, 0, 20 => Ok(80); "jump forwards")]
#[test_case(0, 80, 10 => Ok(40); "jump backwards")]
#[test_case(0, 40, VM_MAX_RAM => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "jump too far forward")]
fn test_jump(is: Word, mut pc: Word, j: Word) -> Result<Word, RuntimeError> {
    jump(Reg::new(&is), RegMut::new(&mut pc), j).map(|_| pc)
}

#[test]
fn test_jump_ne() {
    let mut pc = 0;
    let is = 0;
    let mut a = 0;
    let b = 0;
    let j = 20;
    jump_not_equal(Reg::new(&is), RegMut::new(&mut pc), a, b, j).unwrap();
    assert_eq!(pc, 4);

    a = 1;
    jump_not_equal(Reg::new(&is), RegMut::new(&mut pc), a, b, j).unwrap();
    assert_eq!(pc, 80);
}

#[test]
fn test_jump_zero() {
    let mut pc = 0;
    let is = 0;
    let mut a = 0;
    let zero = 0;
    let j = 20;
    jump_not_zero(Reg::new(&is), RegMut::new(&mut pc), Reg::new(&zero), a, j).unwrap();
    assert_eq!(pc, 4);

    a = 1;
    jump_not_zero(Reg::new(&is), RegMut::new(&mut pc), Reg::new(&zero), a, j).unwrap();
    assert_eq!(pc, 80);
}
