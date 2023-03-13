use super::*;
use test_case::test_case;

#[test_case(0, 0, 0 => Ok(0); "noop jump")]
#[test_case(0, 0, 20 => Ok(80); "jump forwards")]
#[test_case(0, 80, 10 => Ok(40); "jump backwards")]
#[test_case(0, 40, VM_MAX_RAM => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "jump too far forward")]
fn test_absolute_jump(is: Word, mut pc: Word, j: Word) -> Result<Word, RuntimeError> {
    jump(true, JumpMode::Absolute, Reg::new(&is), RegMut::new(&mut pc), j, 0).map(|_| pc)
}

#[test_case(0, 0, 0 => Ok(0); "noop jump")]
#[test_case(0, 0, 20 => Ok(80); "jump from zero")]
#[test_case(0, 80, 10 => Ok(120); "jump from nonzero")]
#[test_case(0, 40, VM_MAX_RAM => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "jump too far forward")]
fn test_relative_forwards_jump(is: Word, mut pc: Word, j: Word) -> Result<Word, RuntimeError> {
    jump(
        true,
        JumpMode::RelativeForwards,
        Reg::new(&is),
        RegMut::new(&mut pc),
        j,
        0,
    )
    .map(|_| pc)
}

#[test_case(0, 0, 0 => Ok(0); "noop jump at zero")]
#[test_case(0, 80, 0 => Ok(80); "noop jump at nonzero")]
#[test_case(0, 20, 5 => Ok(0); "jump to zero")]
#[test_case(0, 80, 10 => Ok(40); "jump to nonzero")]
#[test_case(0, 0, 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "jump below zero from zero")]
#[test_case(0, 20, 50 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "jump below zero from nonzero")]
fn test_relative_backwards_jump(is: Word, mut pc: Word, j: Word) -> Result<Word, RuntimeError> {
    jump(
        true,
        JumpMode::RelativeBackwards,
        Reg::new(&is),
        RegMut::new(&mut pc),
        j,
        0,
    )
    .map(|_| pc)
}

#[test_case(JumpMode::Absolute, 0, 0, 100 => Ok(4); "absolute jump")]
#[test_case(JumpMode::RelativeForwards, 0, 1000, 100 => Ok(1004); "relative jump forwards")]
#[test_case(JumpMode::RelativeBackwards, 0, 1000, 100 => Ok(1004); "relative jump backwards")]
#[test_case(JumpMode::Absolute, 0, 40, VM_MAX_RAM => Ok(44); "abslute jump too far forward")]
#[test_case(JumpMode::RelativeForwards, 0, 40, VM_MAX_RAM => Ok(44); "relative jump too far forward")]
#[test_case(JumpMode::RelativeBackwards, 0, 40, 100 => Ok(44); "relative too far backwards")]
fn test_not_performed_conditional_jump(mode: JumpMode, is: Word, mut pc: Word, j: Word) -> Result<Word, RuntimeError> {
    jump(false, mode, Reg::new(&is), RegMut::new(&mut pc), j, 0).map(|_| pc)
}
