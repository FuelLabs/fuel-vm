use ethnum::U256;

use fuel_asm::{wideint::*, PanicReason};
use fuel_types::{RegisterId, Word};

use super::super::{internal::inc_pc, is_unsafe_math, is_wrapping, ExecutableTransaction, Interpreter};
use crate::interpreter::memory::{read_bytes, write_bytes};
use crate::{constraints::reg_key::*, error::RuntimeError};

macro_rules! wideint_ops {
    ($t:ident, $wider_t:ident) => {
        paste::paste! {
            // Conversion helpers

            /// Converts `primitive_types` version of the current type
            fn [<to_prim_ $t:lower>](value: $t) -> primitive_types::[<$t:upper>] {
                let mut buffer = [0u8; core::mem::size_of::<$t>()];
                buffer[..].copy_from_slice(&value.to_le_bytes());
                primitive_types::[<$t:upper>]::from_little_endian(&buffer)
            }

            /// Converts `primitive_types` version that has double the size of the current type
            fn [<to_wider_prim_ $t:lower>](value: $t) -> primitive_types::[<$wider_t:upper>] {
                let mut buffer = [0u8; 2 * core::mem::size_of::<$t>()];
                buffer[..core::mem::size_of::<$t>()].copy_from_slice(&value.to_le_bytes());
                primitive_types::[<$wider_t:upper>]::from_little_endian(&buffer)
            }

            /// Drops higher half of the value and converts to `u128` or `ethnum::U256`
            fn [<truncate_from_prim_ $t:lower>](value: primitive_types::[<$wider_t:upper>]) -> $t {
                const S: usize = ::core::mem::size_of::<$t>();
                let mut buffer = [0u8; 2 * S];
                value.to_little_endian(&mut buffer);
                let truncated: [u8; S] = buffer[..S].try_into().unwrap_or_else(|_| unreachable!());
                $t::from_le_bytes(truncated)
            }

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
                        // Use wider types to avoid overflow
                        let lhs = [<to_wider_prim_ $t:lower>](lhs);
                        let rhs = [<to_wider_prim_ $t:lower>](rhs);
                        let modulus = [<to_wider_prim_ $t:lower>](modulus);

                        // Trunacte never loses data as modulus is still in domain of the original type
                        [<truncate_from_prim_ $t:lower>]((lhs + rhs) % modulus)
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
                        let lhs = [<to_prim_ $t:lower>](lhs);
                        let rhs = [<to_prim_ $t:lower>](rhs);
                        let modulus = [<to_wider_prim_ $t:lower>](modulus);

                        let result = lhs.full_mul(rhs) % modulus;

                        // This never loses data, since the modulus type has same width as the result
                        [<truncate_from_prim_ $t:lower>](result)
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

                    let lhs = [<to_prim_ $t:lower>](lhs);
                    let rhs = [<to_prim_ $t:lower>](rhs);

                    // TODO: optimize this, especially for divider == 0
                    let divider = if divider == 0 {
                        primitive_types::[<$wider_t:upper>]::one() << (S * 8)
                    } else {
                        [<to_wider_prim_ $t:lower>](divider)
                    };

                    let result = lhs.full_mul(rhs) / divider;

                    let mut buffer = [0u8; 2 * S];
                    result.to_little_endian(&mut buffer);
                    let lower_half: [u8; S] = buffer[..S].try_into().unwrap_or_else(|_| unreachable!());
                    let higher_half: [u8; S] = buffer[S..].try_into().unwrap_or_else(|_| unreachable!());
                    let result = $t::from_le_bytes(lower_half);

                    if higher_half != [0u8; S] {
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

wideint_ops!(u128, U256);
wideint_ops!(U256, U512);

// TODO: benchmark primitive_types against ethnum for each operation
