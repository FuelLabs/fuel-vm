use ethnum::U256;

use fuel_asm::{widemath::*, PanicReason};
use fuel_types::{RegisterId, Word};

use super::super::{internal::inc_pc, is_unsafe_math, is_wrapping, ExecutableTransaction, Interpreter};
use crate::interpreter::memory::{read_bytes, write_bytes};
use crate::{constraints::reg_key::*, error::RuntimeError};

macro_rules! wideint_ops {
    ($t:ident) => {
        paste::paste! {
            impl<S, Tx> Interpreter<S, Tx>
            where
                Tx: ExecutableTransaction,
            {
                pub(crate) fn [<alu_wideint_cmp_ $t:lower>](
                    &mut self,
                    ra: RegisterId,
                    b: Word,
                    c: Word,
                    args: CompareArgs,
                ) -> Result<(), RuntimeError> {
                    let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);

                    // LHS argument is always indirect, load it
                    let lhs: $t = $t::from_be_bytes(read_bytes(&self.memory, b)?);

                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(read_bytes(&self.memory, c)?)
                    } else {
                        c.into()
                    };

                    let result = [<cmp_ $t:lower>](lhs, rhs, args.mode);
                    let dest: &mut Word = &mut w[ra.try_into()?];
                    *dest = result as Word;

                    inc_pc(pc)
                }

                pub(crate) fn [<alu_wideint_op_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    args: MathArgs,
                ) -> Result<(), RuntimeError> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    // LHS argument is always indirect, load it
                    let lhs: $t = $t::from_be_bytes(read_bytes(&self.memory, b)?);

                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(read_bytes(&self.memory, c)?)
                    } else {
                        c.into()
                    };

                    let (wrapped, overflow) = [<op_overflowing_ $t:lower>](lhs, rhs, args);

                    if overflow && !is_wrapping(flag.into()) {
                        return Err(PanicReason::ArithmeticOverflow.into());
                    }

                    *of = overflow as Word;
                    *err = 0;

                    write_bytes(&mut self.memory, owner_regs, dest_addr, wrapped.to_be_bytes())?;

                    inc_pc(pc)
                }

                pub(crate) fn [<alu_wideint_mul_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    args: MulArgs,
                ) -> Result<(), RuntimeError> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    // LHS is only indirect if the flag is set
                    let lhs: $t = if args.indirect_lhs {
                        $t::from_be_bytes(read_bytes(&self.memory, b)?)
                    } else {
                        b.into()
                    };
                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(read_bytes(&self.memory, c)?)
                    } else {
                        c.into()
                    };

                    let (wrapped, overflow) = $t::overflowing_mul(lhs, rhs);

                    if overflow && !is_wrapping(flag.into()) {
                        return Err(PanicReason::ArithmeticOverflow.into());
                    }

                    *of = overflow as Word;
                    *err = 0;

                    write_bytes(&mut self.memory, owner_regs, dest_addr, wrapped.to_be_bytes())?;

                    inc_pc(pc)
                }

                pub(crate) fn [<alu_wideint_div_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    args: DivArgs,
                ) -> Result<(), RuntimeError> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    // LHS is always indirect
                    let lhs: $t = $t::from_be_bytes(read_bytes(&self.memory, b)?);

                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(read_bytes(&self.memory, c)?)
                    } else {
                        c.into()
                    };

                    let result = match $t::checked_div(lhs, rhs) {
                        Some(d) => d,
                        None => {
                            if is_unsafe_math(flag.into()) {
                                $t::default() // Zero
                            } else {
                                return Err(PanicReason::ErrorFlag.into());
                            }
                        }
                    };

                    *of = 0;
                    *err = 0;

                    write_bytes(&mut self.memory, owner_regs, dest_addr, result.to_be_bytes())?;

                    inc_pc(pc)
                }

                pub(crate) fn [<alu_wideint_addmod_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    d: Word,
                ) -> Result<(), RuntimeError> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    let lhs: $t = $t::from_be_bytes(read_bytes(&self.memory, b)?);
                    let rhs: $t = $t::from_be_bytes(read_bytes(&self.memory, c)?);
                    let modulus: $t = $t::from_be_bytes(read_bytes(&self.memory, d)?);

                    let result: $t = if modulus == 0 {
                        if is_unsafe_math(flag.into()) {
                            $t::default() // Zero
                        } else {
                            return Err(PanicReason::ErrorFlag.into());
                        }
                    } else {
                        let (wrapped, carry) = lhs.overflowing_add(rhs);
                        if carry {
                            // TODO: benchmark this against a single wider-int modulo
                            let mod_base = wrapped % modulus;
                            let mod_max = $t::MAX % modulus;
                            (mod_base + mod_max + 1) % modulus // TODO: can this overflow?
                        } else {
                            wrapped % modulus
                        }
                    };

                    *of = 0;
                    *err = 0;

                    write_bytes(&mut self.memory, owner_regs, dest_addr, result.to_be_bytes())?;

                    inc_pc(pc)
                }

                pub(crate) fn [<alu_wideint_mulmod_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    d: Word,
                ) -> Result<(), RuntimeError> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    let lhs: $t = $t::from_be_bytes(read_bytes(&self.memory, b)?);
                    let rhs: $t = $t::from_be_bytes(read_bytes(&self.memory, c)?);
                    let modulus: $t = $t::from_be_bytes(read_bytes(&self.memory, d)?);

                    let result: $t = if modulus == 0 {
                        if is_unsafe_math(flag.into()) {
                            $t::default() // Zero
                        } else {
                            return Err(PanicReason::ErrorFlag.into());
                        }
                    } else {
                        const S: usize = core::mem::size_of::<$t>();

                        let mut buffer = [0u8; S];
                        buffer[..].copy_from_slice(&lhs.to_le_bytes());
                        let lhs = primitive_types::[<$t:upper>]::from_little_endian(&buffer);
                        buffer[..].copy_from_slice(&rhs.to_le_bytes());
                        let rhs = primitive_types::[<$t:upper>]::from_little_endian(&buffer);

                        let mut buffer = [0u8; 2 * S];
                        buffer[S..].copy_from_slice(&modulus.to_le_bytes());
                        let modulus = primitive_types::[<$t:upper>]::from_little_endian(&buffer);

                        let result = lhs.full_mul(rhs) % modulus;
                        result.to_little_endian(&mut buffer);

                        // This never loses data, since the modulus type has same witdth as the result
                        let truncated: [u8; S] = buffer[S..].try_into().unwrap_or_else(|_| unreachable!());
                        $t::from_le_bytes(truncated)
                    };

                    *of = 0;
                    *err = 0;

                    write_bytes(&mut self.memory, owner_regs, dest_addr, result.to_be_bytes())?;

                    inc_pc(pc)
                }

                pub(crate) fn [<alu_wideint_muldiv_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    d: Word,
                ) -> Result<(), RuntimeError> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    let lhs: $t = $t::from_be_bytes(read_bytes(&self.memory, b)?);
                    let rhs: $t = $t::from_be_bytes(read_bytes(&self.memory, c)?);
                    let divider: $t = $t::from_be_bytes(read_bytes(&self.memory, d)?);

                    const S: usize = core::mem::size_of::<$t>();

                    let mut buffer = [0u8; S];
                    buffer[..].copy_from_slice(&lhs.to_le_bytes());
                    let lhs = primitive_types::[<$t:upper>]::from_little_endian(&buffer);
                    buffer[..].copy_from_slice(&rhs.to_le_bytes());
                    let rhs = primitive_types::[<$t:upper>]::from_little_endian(&buffer);

                    // TODO: optimize this, especially for divider == 0
                    let one = primitive_types::[<$t:upper>]::one();
                    let divider = if divider == 0 {
                        one.full_mul(one) << (S * 8)
                    } else {
                        let mut buffer = [0u8; 2 * S];
                        buffer[S..].copy_from_slice(&divider.to_le_bytes());
                        primitive_types::[<$t:upper>]::from_little_endian(&buffer)
                        .full_mul(one)
                    };

                    let result = lhs.full_mul(rhs) / divider;

                    let mut buffer = [0u8; 2 * S];
                    result.to_little_endian(&mut buffer);
                    let truncated: [u8; S] = buffer[S..].try_into().unwrap_or_else(|_| unreachable!());
                    let result = $t::from_le_bytes(truncated);

                    if buffer[..S] != [0u8; S] {
                        *of = 1;
                    } else {
                        *of = 0;
                    }

                    *err = 0;

                    write_bytes(&mut self.memory, owner_regs, dest_addr, result.to_be_bytes())?;

                    inc_pc(pc)
                }
            }

            pub(crate) fn [<cmp_ $t:lower>](
                lhs: $t,
                rhs: $t,
                mode: CompareMode,
            ) -> bool {
                match mode {
                    CompareMode::EQ => lhs == rhs,
                    CompareMode::NE => lhs != rhs,
                    CompareMode::GT => lhs > rhs,
                    CompareMode::LT => lhs < rhs,
                    CompareMode::GTE => lhs >= rhs,
                    CompareMode::LTE => lhs <= rhs,
                }
            }

            /// Returns (wrapped, overflow) just like overflowing_* operations of Rust integers
            pub(crate) fn [<op_overflowing_ $t:lower>] (
                lhs: $t,
                rhs: $t,
                args: MathArgs,
            ) -> ($t, bool) {
                match args.op {
                    MathOp::ADD => { $t::overflowing_add(lhs, rhs) }
                    MathOp::SUB => { $t::overflowing_sub(lhs, rhs) }
                    MathOp::OR => { (lhs | rhs, false) }
                    MathOp::XOR => { (lhs ^ rhs, false) }
                    MathOp::AND => {(lhs & rhs, false) }
                    MathOp::NOT => { (!lhs, false) }
                    MathOp::SHL => {
                        if let Ok(rhs) = rhs.try_into() {
                            ($t::checked_shl(lhs, rhs).unwrap_or_default(), false)
                        } else {
                            ($t::default(), false) // Zero
                        }
                    }
                    MathOp::SHR => {
                        if let Ok(rhs) = rhs.try_into() {
                            ($t::checked_shr(lhs, rhs).unwrap_or_default(), false)
                        } else {
                            ($t::default(), false) // Zero
                        }
                    }
                }
            }
        }
    };
}

wideint_ops!(u128);
wideint_ops!(U256);

// TODO: benchmark primitive_types against ethnum for each operation
