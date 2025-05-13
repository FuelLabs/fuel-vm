use super::*;
use test_case::test_case;

use crate::error::PanicOrBug;

#[test_case(0, 0, 0, 0 => Ok(0); "noop jump")]
#[test_case(0, 10, 0, 0 => Ok(0); "jump to zero")]
#[test_case(0, 10, 0, 4 => Ok(16); "jump to zero plus offset")]
#[test_case(0, 10, 8, 0 => Ok(8); "jump to nonzero")]
#[test_case(0, 10, 8, 4 => Ok(24); "jump to nonzero plus offset")]
#[test_case(1234, 10, 0, 4 => Ok(16); "is won't affect jump")]
#[test_case(0, 0, VM_MAX_RAM + 1, 0  => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "jump too far forward")]
#[test_case(0, 0, 0, VM_MAX_RAM => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "jump too far forward with offset")]
fn test_assign_jump(is: Word, mut pc: Word, j: Word, f: Word) -> SimpleResult<Word> {
    JumpArgs::new(JumpMode::Assign)
        .to_address(j)
        .plus_fixed(f)
        .jump(Reg::new(&is), RegMut::new(&mut pc))
        .map(|_| pc)
}

#[test_case(0, 0, 0 => Ok(0); "noop jump")]
#[test_case(0, 0, 20 => Ok(80); "jump forwards")]
#[test_case(0, 80, 10 => Ok(40); "jump backwards")]
#[test_case(0, 40, VM_MAX_RAM => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "jump too far forward")]
fn test_absolute_jump(is: Word, mut pc: Word, j: Word) -> SimpleResult<Word> {
    JumpArgs::new(JumpMode::RelativeIS)
        .to_address(j)
        .jump(Reg::new(&is), RegMut::new(&mut pc))
        .map(|_| pc)
}

#[test_case(0, 0, 20 => Ok(84); "jump from zero")]
#[test_case(0, 80, 10 => Ok(124); "jump from nonzero")]
#[test_case(0, 40, VM_MAX_RAM => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "jump too far forward")]
fn test_relative_forwards_jump(is: Word, mut pc: Word, j: Word) -> SimpleResult<Word> {
    JumpArgs::new(JumpMode::RelativeForwards)
        .to_address(j)
        .jump(Reg::new(&is), RegMut::new(&mut pc))
        .map(|_| pc)
}

#[test_case(0, 20, 4 => Ok(0); "jump to zero")]
#[test_case(0, 80, 10 => Ok(36); "jump to nonzero")]
#[test_case(0, 0, 0 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "jump below zero from zero 0")]
#[test_case(0, 0, 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "jump below zero from zero 1")]
#[test_case(0, 20, 50 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "jump below zero from nonzero")]
fn test_relative_backwards_jump(is: Word, mut pc: Word, j: Word) -> SimpleResult<Word> {
    JumpArgs::new(JumpMode::RelativeBackwards)
        .to_address(j)
        .jump(Reg::new(&is), RegMut::new(&mut pc))
        .map(|_| pc)
}

#[test_case(JumpMode::RelativeIS, 0, 0, 100 => Ok(4); "absolute jump")]
#[test_case(JumpMode::RelativeForwards, 0, 1000, 100 => Ok(1004); "relative jump forwards")]
#[test_case(JumpMode::RelativeBackwards, 0, 1000, 100 => Ok(1004); "relative jump backwards")]
#[test_case(JumpMode::RelativeIS, 0, 40, VM_MAX_RAM => Ok(44); "abslute jump too far forward")]
#[test_case(JumpMode::RelativeForwards, 0, 40, VM_MAX_RAM => Ok(44); "relative jump too far forward")]
#[test_case(JumpMode::RelativeBackwards, 0, 40, 100 => Ok(44); "relative too far backwards")]
fn test_not_performed_conditional_jump(
    mode: JumpMode,
    is: Word,
    mut pc: Word,
    j: Word,
) -> SimpleResult<Word> {
    JumpArgs::new(mode)
        .with_condition(false)
        .to_address(j)
        .jump(Reg::new(&is), RegMut::new(&mut pc))
        .map(|_| pc)
}
