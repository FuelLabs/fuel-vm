use ethnum::U256;

use fuel_asm::{
    wideint::*,
    PanicReason,
};
use fuel_types::{
    RegisterId,
    Word,
};

use super::super::{
    internal::inc_pc,
    is_unsafe_math,
    is_wrapping,
    ExecutableTransaction,
    Interpreter,
};
use crate::{
    constraints::reg_key::*,
    error::SimpleResult,
    interpreter::Memory,
};

// This macro is used to duplicate the implementation for both 128-bit and 256-bit
// versions. It takes two type parameters: the current type and type that has double-width
// of it. The appropriate type is chosen based on benchmarks for each operation.
// Currently, `primitive_types` is used for anything requiring division, modulo or 512-bit
// precision. Otherwise, `ethnum` is used for 256-bit operations, and the builtin `u128`
// for 128-bit operations.
macro_rules! wideint_ops {
    ($t:ident, $wider_t:ident) => {
        paste::paste! {
            // Conversion helpers

            /// Converts to `primitive_types` version of the current type
            fn [<to_prim_ $t:lower>](value: $t) -> primitive_types::[<$t:upper>] {
                let mut buffer = [0u8; core::mem::size_of::<$t>()];
                buffer[..].copy_from_slice(&value.to_le_bytes());
                primitive_types::[<$t:upper>]::from_little_endian(&buffer)
            }

            /// Converts to `primitive_types` version that has double the size of the current type
            fn [<to_wider_prim_ $t:lower>](value: $t) -> primitive_types::[<$wider_t:upper>] {
                let mut buffer = [0u8; 2 * core::mem::size_of::<$t>()];
                buffer[..core::mem::size_of::<$t>()].copy_from_slice(&value.to_le_bytes());
                primitive_types::[<$wider_t:upper>]::from_little_endian(&buffer)
            }

            /// Converts to `u128` or `ethnum::U256`
            fn [<from_prim_ $t:lower>](value: primitive_types::[<$t:upper>]) -> $t {
                let mut buffer = [0u8; ::core::mem::size_of::<$t>()];
                value.to_little_endian(&mut buffer);
                $t::from_le_bytes(buffer)
            }

            /// Drops higher half of the value and converts to `u128` or `ethnum::U256`
            fn [<truncate_from_prim_ $t:lower>](value: primitive_types::[<$wider_t:upper>]) -> $t {
                const S: usize = ::core::mem::size_of::<$t>();
                let mut buffer = [0u8; 2 * S];
                value.to_little_endian(&mut buffer);
                let truncated: [u8; S] = buffer[..S].try_into().unwrap_or_else(|_| unreachable!());
                $t::from_le_bytes(truncated)
            }

            impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
            where
                M: Memory,
                Tx: ExecutableTransaction,
            {
                pub(crate) fn [<alu_wideint_cmp_ $t:lower>](
                    &mut self,
                    ra: RegisterId,
                    b: Word,
                    c: Word,
                    args: CompareArgs,
                ) -> SimpleResult<()> {
                    let (SystemRegisters { mut of, mut err, pc, .. }, mut w) = split_registers(&mut self.registers);
                    let dest: &mut Word = &mut w[ra.try_into()?];

                    // LHS argument is always indirect, load it
                    let lhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(b)?);

                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(self.memory.as_ref().read_bytes(c)?)
                    } else {
                        c.into()
                    };

                    *dest = [<cmp_ $t:lower>](lhs, rhs, args.mode);
                    *of = 0;
                    *err = 0;

                    inc_pc(pc)?;
                    Ok(())
                }

                pub(crate) fn [<alu_wideint_op_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    args: MathArgs,
                ) -> SimpleResult<()> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    // LHS argument is always indirect, load it
                    let lhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(b)?);

                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(self.memory.as_ref().read_bytes(c)?)
                    } else {
                        c.into()
                    };

                    let (wrapped, overflow) = [<op_overflowing_ $t:lower>](lhs, rhs, args);

                    if overflow && !is_wrapping(flag.into()) {
                        return Err(PanicReason::ArithmeticOverflow.into());
                    }

                    *of = overflow as Word;
                    *err = 0;

                    self.memory.as_mut().write_bytes(owner_regs, dest_addr, wrapped.to_be_bytes())?;

                    Ok(inc_pc(pc)?)
                }

                pub(crate) fn [<alu_wideint_mul_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    args: MulArgs,
                ) -> SimpleResult<()> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    // LHS is only indirect if the flag is set
                    let lhs: $t = if args.indirect_lhs {
                        $t::from_be_bytes(self.memory.as_ref().read_bytes(b)?)
                    } else {
                        b.into()
                    };
                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(self.memory.as_ref().read_bytes(c)?)
                    } else {
                        c.into()
                    };

                    let (wrapped, overflow) = $t::overflowing_mul(lhs, rhs);

                    if overflow && !is_wrapping(flag.into()) {
                        return Err(PanicReason::ArithmeticOverflow.into());
                    }

                    *of = overflow as Word;
                    *err = 0;

                    self.memory.as_mut().write_bytes(owner_regs, dest_addr, wrapped.to_be_bytes())?;

                    Ok(inc_pc(pc)?)
                }

                pub(crate) fn [<alu_wideint_div_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    args: DivArgs,
                ) -> SimpleResult<()> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    // LHS is always indirect
                    let lhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(b)?);

                    // RHS is only indirect if the flag is set
                    let rhs: $t = if args.indirect_rhs {
                        $t::from_be_bytes(self.memory.as_ref().read_bytes(c)?)
                    } else {
                        c.into()
                    };

                    let lhs = [<to_prim_ $t:lower>](lhs);
                    let rhs = [<to_prim_ $t:lower>](rhs);

                    let result = match lhs.checked_div(rhs) {
                        Some(d) => {
                            *err = 0;
                            [<from_prim_ $t:lower>](d)
                        },
                        None => {
                            if is_unsafe_math(flag.into()) {
                                *err = 1;
                                $t::default() // Zero
                            } else {
                                return Err(PanicReason::ArithmeticError.into());
                            }
                        }
                    };

                    *of = 0;

                    self.memory.as_mut().write_bytes(owner_regs, dest_addr, result.to_be_bytes())?;

                    Ok(inc_pc(pc)?)
                }

                pub(crate) fn [<alu_wideint_addmod_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    d: Word,
                ) -> SimpleResult<()> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    let lhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(b)?);
                    let rhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(c)?);
                    let modulus: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(d)?);

                    // Use wider types to avoid overflow
                    let lhs = [<to_wider_prim_ $t:lower>](lhs);
                    let rhs = [<to_wider_prim_ $t:lower>](rhs);
                    let modulus = [<to_wider_prim_ $t:lower>](modulus);

                    let pre_mod = lhs.checked_add(rhs)
                    .expect("Cannot overflow as we're using wider types");
                    let result: $t = match pre_mod.checked_rem(modulus) {
                        Some(result) => {
                            *err = 0;
                            // Truncate never loses data as modulus is still in domain of the original type
                            [<truncate_from_prim_ $t:lower>](result)
                        },
                        None => {
                            if is_unsafe_math(flag.into()) {
                                *err = 1;
                                $t::default() // Zero
                            } else {
                                return Err(PanicReason::ArithmeticError.into());
                            }
                        }
                    };

                    *of = 0;

                    self.memory.as_mut().write_bytes(owner_regs, dest_addr, result.to_be_bytes())?;

                    Ok(inc_pc(pc)?)
                }

                pub(crate) fn [<alu_wideint_mulmod_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    d: Word,
                ) -> SimpleResult<()> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { flag, mut of, mut err, pc, .. }, _) = split_registers(&mut self.registers);

                    let lhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(b)?);
                    let rhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(c)?);
                    let modulus: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(d)?);

                    let lhs = [<to_prim_ $t:lower>](lhs);
                    let rhs = [<to_prim_ $t:lower>](rhs);
                    let modulus = [<to_wider_prim_ $t:lower>](modulus);

                    let result = match lhs.full_mul(rhs).checked_rem(modulus) {
                        None => {
                            if is_unsafe_math(flag.into()) {
                                *err = 1;
                                $t::default() // Zero
                            } else {
                                return Err(PanicReason::ArithmeticError.into());
                            }
                        },
                        Some(result) => {
                            *err = 0;
                            // This never loses data, since the modulus type has same width as the result
                            [<truncate_from_prim_ $t:lower>](result)
                        }

                    };

                    *of = 0;

                    self.memory.as_mut().write_bytes(owner_regs, dest_addr, result.to_be_bytes())?;

                    Ok(inc_pc(pc)?)
                }

                pub(crate) fn [<alu_wideint_muldiv_ $t:lower>](
                    &mut self,
                    dest_addr: Word,
                    b: Word,
                    c: Word,
                    d: Word,
                ) -> SimpleResult<()> {
                    let owner_regs = self.ownership_registers();
                    let (SystemRegisters { mut of, mut err, pc, flag, .. }, _) = split_registers(&mut self.registers);

                    let lhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(b)?);
                    let rhs: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(c)?);
                    let divider: $t = $t::from_be_bytes(self.memory.as_ref().read_bytes(d)?);

                    const S: usize = core::mem::size_of::<$t>();

                    let lhs = [<to_prim_ $t:lower>](lhs);
                    let rhs = [<to_prim_ $t:lower>](rhs);

                    let product = lhs.full_mul(rhs);
                    #[allow(clippy::arithmetic_side_effects)] // Safety: the shift has less bits than the product
                    let product_div_max = product >> (S * 8);
                    let result = product.checked_div([<to_wider_prim_ $t:lower>](divider)).unwrap_or(product_div_max);

                    let mut buffer = [0u8; 2 * S];
                    result.to_little_endian(&mut buffer);
                    let lower_half: [u8; S] = buffer[..S].try_into().unwrap_or_else(|_| unreachable!());
                    let higher_half: [u8; S] = buffer[S..].try_into().unwrap_or_else(|_| unreachable!());
                    let result = $t::from_le_bytes(lower_half);

                    let overflows = higher_half != [0u8; S];
                    if overflows && !is_wrapping(flag.into()) {
                        return Err(PanicReason::ArithmeticOverflow.into());
                    }
                    *of = overflows as Word;
                    *err = 0;

                    self.memory.as_mut().write_bytes(owner_regs, dest_addr, result.to_be_bytes())?;

                    Ok(inc_pc(pc)?)
                }
            }

            pub(crate) fn [<cmp_ $t:lower>](
                lhs: $t,
                rhs: $t,
                mode: CompareMode,
            ) -> Word {
                match mode {
                    CompareMode::EQ => (lhs == rhs) as Word,
                    CompareMode::NE => (lhs != rhs) as Word,
                    CompareMode::GT => (lhs > rhs) as Word,
                    CompareMode::LT => (lhs < rhs) as Word,
                    CompareMode::GTE => (lhs >= rhs) as Word,
                    CompareMode::LTE => (lhs <= rhs) as Word,
                    CompareMode::LZC => lhs.leading_zeros() as Word,
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

wideint_ops!(u128, U256);
wideint_ops!(U256, U512);
