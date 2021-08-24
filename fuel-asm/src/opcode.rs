use crate::types::{Immediate06, Immediate12, Immediate18, Immediate24, RegisterId};
use consts::*;

use core::convert::TryFrom;

#[cfg(feature = "std")]
use std::{io, iter};

pub mod consts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    any(feature = "serde-types", feature = "serde-types-default"),
    derive(serde::Serialize, serde::Deserialize)
)]
/// Instruction representation for the interpreter.
///
/// ## Memory Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation. Every instruction is guaranteed to fit in `u32`
/// representation.
///
/// ## Arithmetic/Logic (ALU) Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation.
///
/// If the [`F_UNSAFEMATH`](./main.md#flags) flag is unset, an operation that
/// would have set `$err` to `true` is instead a panic.
///
/// If the [`F_WRAPPING`](./main.md#flags) flag is unset, an operation that
/// would have set `$of` to a non-zero value is instead a panic. ## Contract
/// Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation, except for [CALL](#call-call-contract) and
/// [REVERT](#revert-revert).
///
/// ## Cryptographic Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation.
pub enum Opcode {
    /// Adds two registers.
    ///
    /// | Operation   | ```$rA = $rB + $rC;``` |
    /// | Syntax      | `add $rA, $rB, $rC`    |
    /// | Encoding    | `0x00 rA rB rC -`      |
    ///
    /// #### Panics
    /// - `$rA` is a reserved register.
    ///
    /// #### Execution
    /// `$of` is assigned the overflow of the operation.
    /// `$err` is cleared.
    ADD(RegisterId, RegisterId, RegisterId),

    /// Adds a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB + imm;```                  |
    /// | Syntax      | `addi $rA, $rB, immediate`              |
    /// | Encoding    | `0x00 rA rB i i`                        |
    ///
    /// #### Panics
    /// - `$rA` is a reserved register.
    ///
    /// #### Execution
    /// `$of` is assigned the overflow of the operation.
    /// `$err` is cleared.
    ADDI(RegisterId, RegisterId, Immediate12),

    /// Bitwise ANDs two registers.
    ///
    /// | Operation   | ```$rA = $rB & $rC;```      |
    /// | Syntax      | `and $rA, $rB, $rC`         |
    /// | Encoding    | `0x00 rA rB rC -`           |
    ///
    /// #### Panics
    /// - `$rA` is a reserved register.
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    AND(RegisterId, RegisterId, RegisterId),

    /// Bitwise ANDs a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB & imm;```                          |
    /// | Syntax      | `andi $rA, $rB, imm`                            |
    /// | Encoding    | `0x00 rA rB i i`                                |
    ///
    /// #### Panics
    /// - `$rA` is a reserved register.
    ///
    /// #### Execution
    /// `imm` is extended to 64 bits, with the high 52 bits set to `0`.
    /// `$of` and `$err` are cleared.
    ANDI(RegisterId, RegisterId, Immediate12),

    /// Divides two registers.
    ///
    /// | Operation   | ```$rA = $rB // $rC;``` |
    /// | Syntax      | `div $rA, $rB, $rC`     |
    /// | Encoding    | `0x00 rA rB rC -`       |
    ///
    /// #### Panics
    /// - `$rA` is a reserved register.
    ///
    /// #### Execution
    /// If `$rC == 0`, `$rA` is cleared and `$err` is set to `true`.
    /// Otherwise, `$err` is cleared.
    /// `$of` is cleared.
    DIV(RegisterId, RegisterId, RegisterId),

    /// Divides a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB // imm;```                    |
    /// | Syntax      | `divi $rA, $rB, imm`                       |
    /// | Encoding    | `0x00 rA rB i i`                           |
    ///
    /// #### Panics
    /// - `$rA` is a reserved register.
    ///
    /// #### Execution
    /// If `imm == 0`, `$rA` is cleared and `$err` is set to `true`.
    /// Otherwise, `$err` is cleared.
    /// `$of` is cleared.
    DIVI(RegisterId, RegisterId, Immediate12),

    /// Compares two registers for equality.
    ///
    /// | Operation   | ```$rA = $rB == $rC;```              |
    /// | Syntax      | `eq $rA, $rB, $rC`                   |
    /// | Encoding    | `0x00 rA rB rC -`                    |
    ///
    /// #### Panics
    /// - `$rA` is a reserved register,
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    EQ(RegisterId, RegisterId, RegisterId),

    /// Raises one register to the power of another.
    ///
    /// | Operation   | ```$rA = $rB ** $rC;```                      |
    /// | Syntax      | `exp $rA, $rB, $rC`                          |
    /// | Encoding    | `0x00 rA rB rC -`                            |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// If the result cannot fit in 8 bytes, `$of` is set to `1`, otherwise
    /// `$of` is cleared.
    /// `$err` is cleared.
    EXP(RegisterId, RegisterId, RegisterId),

    /// Raises one register to the power of an immediate value.
    ///
    /// | Operation   | ```$rA = $rB ** imm;```             |
    /// | Syntax      | `expi $rA, $rB, imm`                |
    /// | Encoding    | `0x00 rA rB i i`                    |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// If the result cannot fit in 8 bytes, `$of` is set to `1`, otherwise
    /// `$of` is cleared.
    /// `$err` is cleared.
    EXPI(RegisterId, RegisterId, Immediate12),

    /// Compares two registers for greater-than.
    ///
    /// | Operation   | ```$rA = $rB > $rC;```                   |
    /// | Syntax      | `gt $rA, $rB, $rC`                       |
    /// | Encoding    | `0x00 rA rB rC -`                        |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    GT(RegisterId, RegisterId, RegisterId),

    /// Compares two registers for less-than.
    ///
    /// | Operation   | ```$rA = $rB < $rC;```                |
    /// | Syntax      | `lt $rA, $rB, $rC`                    |
    /// | Encoding    | `0x00 rA rB rC -`                     |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    LT(RegisterId, RegisterId, RegisterId),

    /// The (integer) logarithm base `$rC` of `$rB`.
    ///
    /// | Operation   | ```$rA = math.floor(math.log($rB, $rC));```  |
    /// | Syntax      | `mlog $rA, $rB, $rC`                         |
    /// | Encoding    | `0x00 rA rB rC -`                            |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// If `$rB == 0`, both `$rA` and `$of` are cleared and `$err` is set to
    /// `true`.
    ///
    /// If `$rC <= 1`, both `$rA` and `$of` are cleared and `$err` is set to
    /// `true`.
    ///
    /// Otherwise, `$of` and `$err` are cleared.
    MLOG(RegisterId, RegisterId, RegisterId),

    /// The (integer) `$rC`th root of `$rB`.
    ///
    /// | Operation   | ```$rA = math.floor(math.root($rB, $rC));``` |
    /// | Syntax      | `mroo $rA, $rB, $rC`                         |
    /// | Encoding    | `0x00 rA rB rC -`                            |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// If `$rC == 0`, both `$rA` and `$of` are cleared and `$err` is set to
    /// `true`.
    ///
    /// Otherwise, `$of` and `$err` are cleared.
    MROO(RegisterId, RegisterId, RegisterId),

    /// Modulo remainder of two registers.
    ///
    /// | Operation   | ```$rA = $rB % $rC;```             |
    /// | Syntax      | `mod $rA, $rB, $rC`                |
    /// | Encoding    | `0x00 rA rB rC -`                  |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// If `$rC == 0`, both `$rA` and `$of` are cleared and `$err` is set to
    /// `true`.
    ///
    /// Otherwise, `$of` and `$err` are cleared.
    MOD(RegisterId, RegisterId, RegisterId),

    /// Modulo remainder of a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB % imm;```                                 |
    /// | Syntax      | `modi $rA, $rB, imm`                                   |
    /// | Encoding    | `0x00 rA rB i i`                                       |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// If `imm == 0`, both `$rA` and `$of` are cleared and `$err` is set to
    /// `true`.
    ///
    /// Otherwise, `$of` and `$err` are cleared.
    MODI(RegisterId, RegisterId, Immediate12),

    /// Copy from one register to another.
    ///
    /// | Operation   | ```$rA = $rB;```                   |
    /// | Syntax      | `move $rA, $rB`                    |
    /// | Encoding    | `0x00 rA rB - -`                   |
    /// | Notes       |                                    |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    MOVE(RegisterId, RegisterId),

    /// Multiplies two registers.
    ///
    /// | Operation   | ```$rA = $rB * $rC;```    |
    /// | Syntax      | `mul $rA, $rB, $rC`       |
    /// | Encoding    | `0x00 rA rB rC -`         |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the overflow of the operation.
    ///
    /// `$err` is cleared.
    MUL(RegisterId, RegisterId, RegisterId),

    /// Multiplies a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB * imm;```                        |
    /// | Syntax      | `mul $rA, $rB, imm`                           |
    /// | Encoding    | `0x00 rA rB i i`                              |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the overflow of the operation.
    ///
    /// `$err` is cleared.
    MULI(RegisterId, RegisterId, Immediate12),

    /// Bitwise NOT a register.
    ///
    /// | Operation   | ```$rA = ~$rB;```       |
    /// | Syntax      | `not $rA, $rB`          |
    /// | Encoding    | `0x00 rA rB - -`        |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    NOT(RegisterId, RegisterId),

    /// Bitwise ORs two registers.
    ///
    /// | Operation   | ```$rA = $rB \| $rC;```    |
    /// | Syntax      | `or $rA, $rB, $rC`         |
    /// | Encoding    | `0x00 rA rB rC -`          |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    OR(RegisterId, RegisterId, RegisterId),

    /// Bitwise ORs a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB \| imm;```                        |
    /// | Syntax      | `ori $rA, $rB, imm`                            |
    /// | Encoding    | `0x00 rA rB i i`                               |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `imm` is extended to 64 bits, with the high 52 bits set to `0`.
    ///
    /// `$of` and `$err` are cleared.
    ORI(RegisterId, RegisterId, Immediate12),

    /// Left shifts a register by a register.
    ///
    /// | Operation   | ```$rA = $rB << $rC;```               |
    /// | Syntax      | `sll $rA, $rB, $rC`                   |
    /// | Encoding    | `0x00 rA rB rC -`                     |
    /// | Notes       | Zeroes are shifted in.                |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the overflow of the operation.
    ///
    /// `$err` is cleared.
    SLL(RegisterId, RegisterId, RegisterId),

    /// Left shifts a register by an immediate value.
    ///
    /// | Operation   | ```$rA = $rB << imm;```                       |
    /// | Syntax      | `slli $rA, $rB, imm`                          |
    /// | Encoding    | `0x00 rA rB i i`                              |
    /// | Notes       | Zeroes are shifted in.                        |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the overflow of the operation.
    ///
    /// `$err` is cleared.
    SLLI(RegisterId, RegisterId, Immediate12),

    /// Right shifts a register by a register.
    ///
    /// | Operation   | ```$rA = $rB >> $rC;```                |
    /// | Syntax      | `srl $rA, $rB, $rC`                    |
    /// | Encoding    | `0x00 rA rB rC -`                      |
    /// | Notes       | Zeroes are shifted in.                 |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the underflow of the operation, as though `$of` is the
    /// high byte of a 128-bit register.
    ///
    /// `$err` is cleared.
    SRL(RegisterId, RegisterId, RegisterId),

    /// Right shifts a register by an immediate value.
    ///
    /// | Operation   | ```$rA = $rB >> imm;```                        |
    /// | Syntax      | `srli $rA, $rB, imm`                           |
    /// | Encoding    | `0x00 rA rB i i`                               |
    /// | Notes       | Zeroes are shifted in.                         |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the underflow of the operation, as though `$of` is the
    /// high byte of a 128-bit register.
    ///
    /// `$err` is cleared.
    SRLI(RegisterId, RegisterId, Immediate12),

    /// Subtracts two registers.
    ///
    /// | Operation   | ```$rA = $rB - $rC;```                           |
    /// | Syntax      | `sub $rA, $rB, $rC`                              |
    /// | Encoding    | `0x00 rA rB rC -`                                |
    /// | Notes       | `$of` is assigned the overflow of the operation. |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the underflow of the operation, as though `$of` is the
    /// high byte of a 128-bit register.
    ///
    /// `$err` is cleared.
    SUB(RegisterId, RegisterId, RegisterId),

    /// Subtracts a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB - imm;```                           |
    /// | Syntax      | `subi $rA, $rB, imm`                             |
    /// | Encoding    | `0x00 rA rB i i`                                 |
    /// | Notes       | `$of` is assigned the overflow of the operation. |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` is assigned the underflow of the operation, as though `$of` is the
    /// high byte of a 128-bit register.
    ///
    /// `$err` is cleared.
    SUBI(RegisterId, RegisterId, Immediate12),

    /// Bitwise XORs two registers.
    ///
    /// | Operation   | ```$rA = $rB ^ $rC;```      |
    /// | Syntax      | `xor $rA, $rB, $rC`         |
    /// | Encoding    | `0x00 rA rB rC -`           |
    /// | Notes       |                             |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    XOR(RegisterId, RegisterId, RegisterId),

    /// Bitwise XORs a register and an immediate value.
    ///
    /// | Operation   | ```$rA = $rB ^ imm;```                          |
    /// | Syntax      | `xori $rA, $rB, imm`                            |
    /// | Encoding    | `0x00 rA rB i i`                                |
    /// | Notes       |                                                 |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    ///
    /// #### Execution
    /// `$of` and `$err` are cleared.
    XORI(RegisterId, RegisterId, Immediate12),

    /// Set `$rA` to `true` if the `$rC <= tx.input[$rB].maturity`.
    ///
    /// | Operation   | ```$rA = checkinputmaturityverify($rB, $rC);``` |
    /// | Syntax      | `cimv $rA $rB $rC`                              |
    /// | Encoding    | `0x00 rA rB rC -`                               |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rC > tx.input[$rB].maturity`
    /// - the input `$rB` is not of type
    ///   [`InputType.Coin`](../protocol/tx_format.md)
    /// - `$rB > tx.inputsCount`
    ///
    /// #### Execution
    /// Otherwise, advance the program counter `$pc` by `4`.
    ///
    /// See also: [BIP-112](https://github.com/bitcoin/bips/blob/master/bip-0112.mediawiki) and [CLTV](#cltv-check-lock-time-verify).
    CIMV(RegisterId, RegisterId, RegisterId),

    /// Set `$rA` to `true` if `$rB <= tx.maturity`.
    ///
    /// | Operation   | ```$rA = checktransactionmaturityverify($rB);``` |
    /// | Syntax      | `ctmv $rA $rB`                                   |
    /// | Encoding    | `0x00 rA rB - -`                                 |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rB > tx.maturity`
    ///
    /// #### Execution
    /// Otherwise, advance the program counter `$pc` by `4`.
    ///
    /// See also: [BIP-65](https://github.com/bitcoin/bips/blob/master/bip-0065.mediawiki) and [Bitcoin's Time Locks](https://prestwi.ch/bitcoin-time-locks).
    CTMV(RegisterId, RegisterId),

    /// Jumps to the code instruction offset by `imm`.
    ///
    /// | Operation   | ```$pc = $is + imm * 4;```                     |
    /// | Syntax      | `ji imm`                                       |
    /// | Encoding    | `0x00 i i i i`                                 |
    ///
    /// #### Panics
    /// - `$is + imm * 4 > VM_MAX_RAM - 1`
    JI(Immediate24),

    /// Jump to the code instruction offset by `imm` if `$rA` is not equal to
    /// `$rB`.
    ///
    /// | Operation   | ```if $rA != $rB:```<br>```$pc = $is + imm *
    /// 4;```<br>```else:```<br>```$pc += 4;``` | Syntax      | `jnei $rA
    /// $rB imm` | Encoding    | `0x00 rA rB i i`
    ///
    /// #### Panics
    /// - `$is + imm * 4 > VM_MAX_RAM - 1`
    JNEI(RegisterId, RegisterId, Immediate12),

    /// Returns from [context](./main.md#contexts) with value `$rA`.
    ///
    /// | Operation   | ```return($rA);```
    /// | Syntax      | `ret $rA`
    /// | Encoding    | `0x00 rA - - -`
    ///
    /// If current context is external, cease VM execution and return `$rA`.
    ///
    /// Returns from contract call, popping the call frame. Before popping:
    ///
    /// 1. Return the unused forwarded gas to the caller:
    ///     - `$cgas = $cgas + $fp->$cgas` (add remaining context gas from
    ///       previous context to current remaining context gas)
    ///
    /// Then pop the call frame and restoring registers _except_ `$ggas` and
    /// `$cgas`. Afterwards, set the following registers:
    ///
    /// 1. `$pc = $pc + 4` (advance program counter from where we called)
    RET(RegisterId),

    /// Extend the current call frame's stack by an immediate value.
    ///
    /// | Operation   | ```$sp = $sp + imm```
    /// | Syntax      | `cfei imm`
    /// | Encoding    | `0x00 i i i i`
    /// | Notes       | Does not initialize memory.
    ///
    /// #### Panics
    /// - `$sp + imm` overflows
    /// - `$sp + imm > $hp`
    CFEI(Immediate24),

    /// Shrink the current call frame's stack by an immediate value.
    ///
    /// | Operation   | ```$sp = $sp - imm```
    /// | Syntax      | `cfsi imm`
    /// | Encoding    | `0x00 i i i i`
    /// | Notes       | Does not clear memory.
    ///
    /// #### Panics
    /// - `$sp - imm` underflows
    /// - `$sp - imm < $ssp`
    CFSI(Immediate24),

    /// A byte is loaded from the specified address offset by `imm`.
    ///
    /// | Operation   | ```$rA = MEM[$rB + imm, 1];```
    /// | Syntax      | `lb $rA, $rB, imm`
    /// | Encoding    | `0x00 rA rB i i`
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rB + imm + 1` overflows
    /// - `$rB + imm + 1 > VM_MAX_RAM`
    LB(RegisterId, RegisterId, Immediate12),

    /// A word is loaded from the specified address offset by `imm`.
    /// | Operation   | ```$rA = MEM[$rB + imm, 8];```
    /// | Syntax      | `lw $rA, $rB, imm`
    /// | Encoding    | `0x00 rA rB i i`
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rB + imm + 8` overflows
    /// - `$rB + imm + 8 > VM_MAX_RAM`
    LW(RegisterId, RegisterId, Immediate12),

    /// Allocate a number of bytes from the heap.
    ///
    /// | Operation   | ```$hp = $hp - $rA;```                    |
    /// | Syntax      | `aloc $rA`                                |
    /// | Encoding    | `0x00 rA - - -`                           |
    /// | Notes       | Does not initialize memory.               |
    ///
    /// #### Panics
    /// - `$hp - $rA` underflows
    /// - `$hp - $rA < $sp`
    ALOC(RegisterId),

    /// Clear bytes in memory.
    ///
    /// | Operation   | ```MEM[$rA, $rB] = 0;``` |
    /// | Syntax      | `mcl $rA, $rB`           |
    /// | Encoding    | `0x00 rA rB - -`         |
    ///
    /// #### Panics
    /// - `$rA + $rB` overflows
    /// - `$rA + $rB > VM_MAX_RAM`
    /// - `$rB > MEM_MAX_ACCESS_SIZE`
    /// - The memory range `MEM[$rA, $rB]`  does not pass [ownership
    ///   check](./main.md#ownership)
    MCL(RegisterId, RegisterId),

    /// Clear bytes in memory.
    ///
    /// | Operation   | ```MEM[$rA, imm] = 0;``` |
    /// | Syntax      | `mcli $rA, imm`          |
    /// | Encoding    | `0x00 rA i i i`          |
    ///
    /// #### Panics
    /// - `$rA + imm` overflows
    /// - `$rA + imm > VM_MAX_RAM`
    /// - `imm > MEM_MAX_ACCESS_SIZE`
    /// - The memory range `MEM[$rA, imm]`  does not pass [ownership
    ///   check](./main.md#ownership)
    MCLI(RegisterId, Immediate18),

    /// Copy bytes in memory.
    ///
    /// | Operation   | ```MEM[$rA, $rC] = MEM[$rB, $rC];``` |
    /// | Syntax      | `mcp $rA, $rB, $rC`                  |
    /// | Encoding    | `0x00 rA rB rC -`                    |
    ///
    /// #### Panics
    /// - `$rA + $rC` overflows
    /// - `$rB + $rC` overflows
    /// - `$rA + $rC > VM_MAX_RAM`
    /// - `$rB + $rC > VM_MAX_RAM`
    /// - `$rC > MEM_MAX_ACCESS_SIZE`
    /// - The memory ranges `MEM[$rA, $rC]` and `MEM[$rB, $rC]` overlap
    /// - The memory range `MEM[$rA, $rC]`  does not pass [ownership
    ///   check](./main.md#ownership)
    MCP(RegisterId, RegisterId, RegisterId),

    /// Compare bytes in memory.
    ///
    /// | Operation   | ```$rA = MEM[$rB, $rD] == MEM[$rC, $rD];``` |
    /// | Syntax      | `meq $rA, $rB, $rC, $rD`                    |
    /// | Encoding    | `0x00 rA rB rC rD`                          |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rB + $rD` overflows
    /// - `$rC + $rD` overflows
    /// - `$rB + $rD > VM_MAX_RAM`
    /// - `$rC + $rD > VM_MAX_RAM`
    /// - `$rD > MEM_MAX_ACCESS_SIZE`
    MEQ(RegisterId, RegisterId, RegisterId, RegisterId),

    /// The least significant byte of `$rB` is stored at the address `$rA`
    /// offset by `imm`.
    ///
    /// | Operation   | ```MEM[$rA + imm, 1] = $rB[7, 1];```    |
    /// | Syntax      | `sb $rA, $rB, imm`                      |
    /// | Encoding    | `0x00 rA rB i i`                        |
    ///
    /// #### Panics
    /// - `$rA + imm + 1` overflows
    /// - `$rA + imm + 1 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA + imm, 1]`  does not pass [ownership
    ///   check](./main.md#ownership)
    SB(RegisterId, RegisterId, Immediate12),

    /// The value of `$rB` is stored at the address `$rA` offset by `imm`.
    ///
    /// | Operation   | ```MEM[$rA + imm, 8] = $rB;```
    /// | Syntax      | `sw $rA, $rB, imm`
    /// | Encoding    | `0x00 rA rB i i`
    ///
    /// #### Panics
    /// - `$rA + imm + 8` overflows
    /// - `$rA + imm + 8 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA + imm, 8]`  does not pass [ownership
    ///   check](./main.md#ownership)
    SW(RegisterId, RegisterId, Immediate12),

    /// Get block header hash.
    ///
    /// | Operation   | ```MEM[$rA, 32] = blockhash($rB);``` |
    /// | Syntax      | `bhsh $rA $rB`                       |
    /// | Encoding    | `0x00 rA rB - -`                     |
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, 32]`  does not pass [ownership
    ///   check](./main.md#ownership)
    ///
    /// Block header hashes for blocks with height greater than or equal to
    /// current block height are zero (`0x00**32`).
    BHSH(RegisterId, RegisterId),

    /// Get Fuel block height.
    ///
    /// | Operation   | ```$rA = blockheight();``` |
    /// | Syntax      | `bhei $rA`                 |
    /// | Encoding    | `0x00 rA - - -`            |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    BHEI(RegisterId),

    /// Burn `$rA` coins of the current contract's color.
    ///
    /// | Operation   | ```burn($rA);```                                  |
    /// | Syntax      | `burn $rA`                                        |
    /// | Encoding    | `0x00 rA - - -`                                   |
    ///
    /// #### Panic
    /// - Balance of color `MEM[$fp, 32]` of output with contract ID `MEM[$fp,
    ///   32]` minus `$rA` underflows
    /// - `$fp == 0` (in the script context)
    ///
    /// For output with contract ID `MEM[$fp, 32]`, decrease balance of color
    /// `MEM[$fp, 32]` by `$rA`.
    ///
    /// This modifies the `balanceRoot` field of the appropriate output.
    BURN(RegisterId),

    /// Call contract.
    ///
    /// | Syntax      | `call $rA $rB $rC $rD` |
    /// | Encoding    | `0x00 rA rB rC rD`     |
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rC + 32` overflows
    /// - Contract with ID `MEM[$rA, 32]` is not in `tx.inputs`
    /// - Reading past `MEM[VM_MAX_RAM - 1]`
    /// - Any output range does not pass [ownership check](./main.md#ownership)
    /// - In an external context, if `$rB > MEM[balanceOfStart(MEM[$rC, 32]),
    ///   8]`
    /// - In an internal context, if `$rB` is greater than the balance of color
    ///   `MEM[$rC, 32]` of output with contract ID `MEM[$rA, 32]`
    ///
    /// Register `$rA` is a memory address from which the following fields are
    /// set (word-aligned).
    ///
    /// `$rD` is the amount of gas to forward. If it is set to an amount greater
    /// than the available gas, all available gas is forwarded.
    ///
    /// For output with contract ID `MEM[$rA, 32]`, increase balance of color
    /// `MEM[$rC, 32]` by `$rB`. In an external context, decrease
    /// `MEM[balanceOfStart(MEM[$rC, 32]), 8]` by `$rB`. In an internal context,
    /// decrease color `MEM[$rC, 32]` balance of output with contract ID
    /// `MEM[$fp, 32]` by `$rB`.
    ///
    /// A [call frame](./main.md#call-frames) is pushed at `$sp`. In addition to
    /// filling in the values of the call frame, the following registers are
    /// set:
    ///
    /// 1. `$fp = $sp` (on top of the previous call frame is the beginning of
    /// this call frame) 1. Set `$ssp` and `$sp` to the start of the
    /// writable stack area of the call frame. 1. Set `$pc` and `$is` to the
    /// starting address of the code. 1. `$bal = $rD` (forward coins)
    /// 1. `$cgas = $rD` or all available gas (forward gas)
    ///
    /// This modifies the `balanceRoot` field of the appropriate output(s).
    CALL(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Copy `$rD` bytes of code starting at `$rC` for contract with ID equal to
    /// the 32 bytes in memory starting at `$rB` into memory starting at `$rA`.
    ///
    /// | Operation   | ```MEM[$rA, $rD] = code($rB, $rC, $rD);```
    /// | Syntax      | `ccp $rA, $rB, $rC, $rD`
    /// | Encoding    | `0x00 rA rB rC rD`
    /// | Notes       | If `$rD` is greater than the code size, zero bytes are
    /// filled in.
    ///
    /// #### Panics
    /// - `$rA + $rD` overflows
    /// - `$rB + 32` overflows
    /// - `$rA + $rD > VM_MAX_RAM`
    /// - `$rB + 32 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, $rD]`  does not pass [ownership
    ///   check](./main.md#ownership)
    /// - `$rD > MEM_MAX_ACCESS_SIZE`
    /// - Contract with ID `MEM[$rB, 32]` is not in `tx.inputs`
    CCP(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Set the 32 bytes in memory starting at `$rA` to the code root for
    /// contract with ID equal to the 32 bytes in memory starting at `$rB`.
    ///
    /// | Operation   | ```MEM[$rA, 32] = coderoot(MEM[$rB, 32]);```
    /// | Syntax      | `croo $rA, $rB`
    /// | Encoding    | `0x00 rA rB - -`
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rB + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$rB + 32 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, 32]`  does not pass [ownership
    ///   check](./main.md#ownership)
    /// - Contract with ID `MEM[$rB, 32]` is not in `tx.inputs`
    ///
    /// Code root compuration is defined
    /// [here](../protocol/identifiers.md#contract-id).
    CROO(RegisterId, RegisterId),

    /// Set `$rA` to the size of the code for contract with ID equal to the 32
    /// bytes in memory starting at `$rB`.
    ///
    /// | Operation   | ```$rA = codesize(MEM[$rB, 32]);```
    /// | Syntax      | `csiz $rA, $rB`
    /// | Encoding    | `0x00 rA rB - -`
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rB + 32` overflows
    /// - `$rB + 32 > VM_MAX_RAM`
    /// - Contract with ID `MEM[$rB, 32]` is not in `tx.inputs`
    CSIZ(RegisterId, RegisterId),

    /// Get block proposer address.
    ///
    /// | Operation   | ```MEM[$rA, 32] = coinbase();``` |
    /// | Syntax      | `cb $rA`                         |
    /// | Encoding    | `0x00 rA - - -`                  |
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, 32]`  does not pass [ownership
    ///   check](./main.md#ownership)
    CB(RegisterId),

    /// Copy `$rC` bytes of code starting at `$rB` for contract with ID equal to
    /// the 32 bytes in memory starting at `$rA` into memory starting at `$ssp`.
    ///
    /// | Operation   | ```MEM[$ssp, $rC] = code($rA, $rB, $rC);```
    /// | Syntax      | `ldc $rA, $rB, $rC`
    /// | Encoding    | `0x00 rA rB rC -`
    /// | Notes       | If `$rC` is greater than the code size, zero bytes are
    /// filled in.
    ///
    /// #### Panics
    /// - `$ssp + $rC` overflows
    /// - `$rA + 32` overflows
    /// - `$ssp + $rC > VM_MAX_RAM`
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$ssp != $sp`
    /// - `$ssp + $rC > $hp`
    /// - `$rC > CONTRACT_MAX_SIZE`
    /// - `$rC > MEM_MAX_ACCESS_SIZE`
    /// - Contract with ID `MEM[$rA, 32]` is not in `tx.inputs`
    ///
    /// Increment `$hp->codesize`, `$ssp`, and `$sp` by `$rC` padded to word
    /// alignment.
    ///
    /// This opcode can be used to concatenate the code of multiple contracts
    /// together. It can only be used when the stack area of the call frame is
    /// unused (i.e. prior to being used).
    LDC(RegisterId, RegisterId, RegisterId),

    /// Log an event. This is a no-op.
    ///
    /// | Operation   | ```log($rA, $rB, $rC, $rD);``` |
    /// | Syntax      | `log $rA, $rB, $rC, $rD`       |
    /// | Encoding    | `0x00 rA rB rC rD`             |
    LOG(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Logs the memory range `MEM[$rC, $rD]`. This is a no-op.
    ///
    /// | Syntax      | `logd $rA, $rB, $rC, $rD`       |
    /// | Encoding    | `0x00 rA rB rC rD`              |
    LOGD(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Mint `$rA` coins of the current contract's color.
    ///
    /// | Operation   | ```mint($rA);```                                  |
    /// | Syntax      | `mint $rA`                                        |
    /// | Encoding    | `0x00 rA - - -`                                   |
    ///
    /// #### Panics
    /// - Balance of color `MEM[$fp, 32]` of output with contract ID `MEM[$fp,
    ///   32]` plus `$rA` overflows
    /// - `$fp == 0` (in the script context)
    ///
    /// For output with contract ID `MEM[$fp, 32]`, increase balance of color
    /// `MEM[$fp, 32]` by `$rA`.
    ///
    /// This modifies the `balanceRoot` field of the appropriate output.
    MINT(RegisterId),

    /// Halt execution, reverting state changes and returning value in `$rA`.
    ///
    /// | Operation   | ```revert($rA);```
    /// | Syntax      | `rvrt $rA`
    /// | Encoding    | `0x00 rA - - -`
    ///
    /// After a revert:
    ///
    /// 1. All [OutputContract](../protocol/tx_format.md#outputcontract) outputs
    /// will have the same `amount` and `stateRoot` as on initialization. 1.
    /// All [OutputVariable](../protocol/tx_format.md outputs#outputvariable)
    /// outputs will have `to` and `amount` of zero.
    /// 1. All [OutputContractConditional](../protocol/tx_format.md#
    /// outputcontractconditional) outputs will have `contractID`, `amount`, and
    /// `stateRoot` of zero.
    RVRT(RegisterId),

    /// Copy `$rC` bytes of code starting at `$rB` for contract with static
    /// index `$rA` into memory starting at `$ssp`.
    ///
    /// | Operation   | ```MEM[$ssp, $rC] = scode($rA, $rB, $rC);```
    /// | Syntax      | `sloadcode $rA, $rB, $rC`
    /// | Encoding    | `0x00 rA rB rC -`
    /// | Notes       | If `$rC` is greater than the code size, zero bytes
    /// are filled in.                                               |
    ///
    /// #### Panics
    /// - `$ssp + $rC` overflows
    /// - `$ssp + $rC > VM_MAX_RAM`
    /// - `$rA >= MAX_STATIC_CONTRACTS`
    /// - `$rA` is greater than or equal to `staticContractsCount` for the
    ///   contract with ID `MEM[$fp, 32]`
    /// - `$ssp != $sp`
    /// - `$ssp + $rC > $hp`
    /// - `$rC > CONTRACT_MAX_SIZE`
    /// - `$rC > MEM_MAX_ACCESS_SIZE`
    /// - `$fp == 0` (in the script context)
    ///
    /// Increment `$hp->codesize`, `$ssp`, and `$sp` by `$rC` padded to word
    /// alignment.
    ///
    /// This opcode can be used to concatenate the code of multiple contracts
    /// together. It can only be used when the stack area of the call frame is
    /// unused (i.e. prior to being used).
    SLDC(RegisterId, RegisterId, RegisterId),

    /// A word is read from the current contract's state.
    ///
    /// | Operation   | ```$rA = STATE[MEM[$rB, 32]][0, 8];```            |
    /// | Syntax      | `srw $rA, $rB`                                    |
    /// | Encoding    | `0x00 rA rB - -`                                  |
    /// | Notes       | Returns zero if the state element does not exist. |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rB + 32` overflows
    /// - `$rB + 32 > VM_MAX_RAM`
    /// - `$fp == 0` (in the script context)
    SRW(RegisterId, RegisterId),

    /// 32 bytes is read from the current contract's state.
    ///
    /// | Operation   | ```MEM[$rA, 32] = STATE[MEM[$rB, 32]];```           |
    /// | Syntax      | `srwx $rA, $rB`                                     |
    /// | Encoding    | `0x00 rA rB - -`                                    |
    /// | Notes       | Returns zero if the state element does not exist.   |
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rB + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$rB + 32 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, 32]`  does not pass [ownership
    ///   check](./main.md#ownership)
    /// - `$fp == 0` (in the script context)
    SRWQ(RegisterId, RegisterId),

    /// A word is written to the current contract's state.
    ///
    /// | Operation   | ```STATE[MEM[$rA, 32]][0, 8] = $rB;```             |
    /// | Syntax      | `sww $rA $rB`                                      |
    /// | Encoding    | `0x00 rA rB - -`                                   |
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$fp == 0` (in the script context)
    SWW(RegisterId, RegisterId),

    /// 32 bytes is written to the current contract's state.
    ///
    /// | Operation   | ```STATE[MEM[$rA, 32]] = MEM[$rB, 32];```            |
    /// | Syntax      | `swwx $rA, $rB`                                      |
    /// | Encoding    | `0x00 rA rB - -`                                     |
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rB + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$rB + 32 > VM_MAX_RAM`
    /// - `$fp == 0` (in the script context)
    SWWQ(RegisterId, RegisterId),

    /// Transfer `$rB` coins with color at `$rC` to contract with ID at `$rA`.
    ///
    /// | Operation   | ```transfer(MEM[$rA, 32], $rB, MEM[$rC, 32]);```
    /// | Syntax      | `tr $rA,  $rB, $rC`
    /// | Encoding    | `0x00 rA rB rC -`
    ///
    /// Given helper `balanceOfStart(color: byte[32]) -> uint32` which returns
    /// the memory address of `color` balance, or `0` if `color` has no balance.
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rC + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$rC + 32 > VM_MAX_RAM`
    /// - Contract with ID `MEM[$rA, 32]` is not in `tx.inputs`
    /// - In an external context, if `$rB > MEM[balanceOf(MEM[$rC, 32]), 8]`
    /// - In an internal context, if `$rB` is greater than the balance of color
    ///   `MEM[$rC, 32]` of output with contract ID `MEM[$fp, 32]`
    /// - `$rB == 0`
    ///
    /// For output with contract ID `MEM[$rA, 32]`, increase balance of color
    /// `MEM[$rC, 32]` by `$rB`. In an external context, decrease
    /// `MEM[balanceOfStart(MEM[$rC, 32]), 8]` by `$rB`. In an internal context,
    /// decrease color `MEM[$rC, 32]` balance of output with contract ID
    /// `MEM[$fp, 32]` by `$rB`.
    ///
    /// This modifies the `balanceRoot` field of the appropriate output(s).
    TR(RegisterId, RegisterId, RegisterId),

    /// Transfer `$rC` coins with color at `$rD` to address at `$rA`, with
    /// output `$rB`. | Operation   | ```transferout(MEM[$rA, 32], $rB, $rC,
    /// MEM[$rD, 32]);``` | Syntax      | `tro $rA, $rB, $rC, $rD`
    /// | Encoding    | `0x00 rA rB rC rD`
    ///
    /// Given helper `balanceOfStart(color: byte[32]) -> uint32` which returns
    /// the memory address of `color` balance, or `0` if `color` has no balance.
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rD + 32` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$rD + 32 > VM_MAX_RAM`
    /// - `$rB > tx.outputsCount`
    /// - In an external context, if `$rC > MEM[balanceOf(MEM[$rD, 32]), 8]`
    /// - In an internal context, if `$rC` is greater than the balance of color
    ///   `MEM[$rD, 32]` of output with contract ID `MEM[$fp, 32]`
    /// - `$rC == 0`
    /// - `tx.outputs[$rB].type != OutputType.Variable`
    /// - `tx.outputs[$rB].amount != 0`
    ///
    /// In an external context, decrease `MEM[balanceOfStart(MEM[$rD, 32]), 8]`
    /// by `$rC`. In an internal context, decrease color `MEM[$rD, 32]` balance
    /// of output with contract ID `MEM[$fp, 32]` by `$rC`. Then set:
    ///
    /// - `tx.outputs[$rB].to = MEM[$rA, 32]`
    /// - `tx.outputs[$rB].amount = $rC`
    /// - `tx.outputs[$rB].color = MEM[$rD, 32]`
    ///
    /// This modifies the `balanceRoot` field of the appropriate output(s).
    TRO(RegisterId, RegisterId, RegisterId, RegisterId),

    /// The 64-byte public key (x, y) recovered from 64-byte
    /// signature starting at `$rB` on 32-byte message hash starting at `$rC`. |
    ///
    /// | Operation   | ```MEM[$rA, 64] = ecrecover(MEM[$rB, 64], MEM[$rC,
    /// 32]);``` | Syntax      | `ecr $rA, $rB, $rC`
    /// | Encoding    | `0x00 rA rB rC -`
    ///
    /// #### Panics
    /// - `$rA + 64` overflows
    /// - `$rB + 64` overflows
    /// - `$rC + 32` overflows
    /// - `$rA + 64 > VM_MAX_RAM`
    /// - `$rB + 64 > VM_MAX_RAM`
    /// - `$rC + 32 > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, 64]`  does not pass [ownership
    ///   check](./main.md#ownership)
    ///
    /// To get the address, hash the public key with
    /// [SHA-2-256](#sha256-sha-2-256).
    ECR(RegisterId, RegisterId, RegisterId),

    /// The keccak-256 hash of `$rC` bytes starting at `$rB`.
    ///
    /// | Operation   | ```MEM[$rA, 32] = keccak256(MEM[$rB, $rC]);```
    /// | Syntax      | `k256 $rA, $rB, $rC`
    /// | Encoding    | `0x00 rA rB rC -`
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rB + $rC` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$rB + $rC > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, 32]`  does not pass [ownership
    ///   check](./main.md#ownership)
    /// - `$rC > MEM_MAX_ACCESS_SIZE`
    K256(RegisterId, RegisterId, RegisterId),

    /// The SHA-2-256 hash of `$rC` bytes starting at `$rB`.
    ///
    /// | Operation   | ```MEM[$rA, 32] = sha256(MEM[$rB, $rC]);```          |
    /// | Syntax      | `s256 $rA, $rB, $rC`                                 |
    /// | Encoding    | `0x00 rA rB rC -`                                    |
    ///
    /// #### Panics
    /// - `$rA + 32` overflows
    /// - `$rB + $rC` overflows
    /// - `$rA + 32 > VM_MAX_RAM`
    /// - `$rB + $rC > VM_MAX_RAM`
    /// - The memory range `MEM[$rA, 32]`  does not pass [ownership
    ///   check](./main.md#ownership)
    /// - `$rC > MEM_MAX_ACCESS_SIZE`
    S256(RegisterId, RegisterId, RegisterId),

    /// Set `$rA` to the length in bytes of [the `$rB`th
    /// input](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xil($rB);```      |
    /// | Syntax      | `xil $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.inputsCount`
    XIL(RegisterId, RegisterId),

    /// Set `$rA` to the memory addess of the start of [the `$rB`th
    /// input](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xis($rB);```      |
    /// | Syntax      | `xis $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.inputsCount`
    XIS(RegisterId, RegisterId),

    /// Set `$rA` to the length in bytes of [the `$rB`th
    /// output](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xol($rB);```      |
    /// | Syntax      | `xol $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.outputsCount`
    XOL(RegisterId, RegisterId),

    /// Set `$rA` to the memory addess of the start of [the `$rB`th
    /// output](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xos($rB);```      |
    /// | Syntax      | `xos $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.outputsCount`
    XOS(RegisterId, RegisterId),

    /// Set `$rA` to the length in bytes of [the `$rB`th
    /// witness](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xwl($rB);```      |
    /// | Syntax      | `xwl $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.witnessesCount`
    ///
    /// Note that the returned length includes the [_entire_
    /// witness](../protocol/tx_format.md), not just of the witness's `data`
    /// field.
    XWL(RegisterId, RegisterId),

    /// Set `$rA` to the memory addess of the start of [the `$rB`th
    /// witness](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xws($rB);```      |
    /// | Syntax      | `xws $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.witnessesCount`
    ///
    /// Note that the returned memory address includes the [_entire_
    /// witness](../protocol/tx_format.md), not just of the witness's `data`
    /// field.
    XWS(RegisterId, RegisterId),

    /// Performs no operation.
    ///
    /// | Operation   |                        |
    /// | Syntax      | `noop`                 |
    /// | Encoding    | `0x00 - - - -`         |
    ///
    /// `$of` and `$err` are cleared.
    NOOP,

    /// Set `$flag` to `$rA`.
    ///
    /// | Operation   | ```$flag = $rA;```    |
    /// | Syntax      | `flag $rA`            |
    /// | Encoding    | `0x00 rA - - -`       |
    FLAG(RegisterId),

    /// Undefined opcode, potentially from inconsistent serialization
    Undefined,
}

impl Opcode {
    /// Size of the struct when serialized into bytes
    pub const BYTES_SIZE: usize = 4;

    /// Create a new [`Opcode`] given the internal attributes
    pub const fn new(
        op: u8,
        ra: RegisterId,
        rb: RegisterId,
        rc: RegisterId,
        rd: RegisterId,
        _imm06: Immediate06,
        imm12: Immediate12,
        imm18: Immediate18,
        imm24: Immediate24,
    ) -> Self {
        let op = OpcodeRepr::from_u8(op);

        match op {
            OpcodeRepr::ADD => Opcode::ADD(ra, rb, rc),
            OpcodeRepr::ADDI => Opcode::ADDI(ra, rb, imm12),
            OpcodeRepr::AND => Opcode::AND(ra, rb, rc),
            OpcodeRepr::ANDI => Opcode::ANDI(ra, rb, imm12),
            OpcodeRepr::DIV => Opcode::DIV(ra, rb, rc),
            OpcodeRepr::DIVI => Opcode::DIVI(ra, rb, imm12),
            OpcodeRepr::EQ => Opcode::EQ(ra, rb, rc),
            OpcodeRepr::EXP => Opcode::EXP(ra, rb, rc),
            OpcodeRepr::EXPI => Opcode::EXPI(ra, rb, imm12),
            OpcodeRepr::GT => Opcode::GT(ra, rb, rc),
            OpcodeRepr::LT => Opcode::LT(ra, rb, rc),
            OpcodeRepr::MLOG => Opcode::MLOG(ra, rb, rc),
            OpcodeRepr::MROO => Opcode::MROO(ra, rb, rc),
            OpcodeRepr::MOD => Opcode::MOD(ra, rb, rc),
            OpcodeRepr::MODI => Opcode::MODI(ra, rb, imm12),
            OpcodeRepr::MOVE => Opcode::MOVE(ra, rb),
            OpcodeRepr::MUL => Opcode::MUL(ra, rb, rc),
            OpcodeRepr::MULI => Opcode::MULI(ra, rb, imm12),
            OpcodeRepr::NOT => Opcode::NOT(ra, rb),
            OpcodeRepr::OR => Opcode::OR(ra, rb, rc),
            OpcodeRepr::ORI => Opcode::ORI(ra, rb, imm12),
            OpcodeRepr::SLL => Opcode::SLL(ra, rb, rc),
            OpcodeRepr::SLLI => Opcode::SLLI(ra, rb, imm12),
            OpcodeRepr::SRL => Opcode::SRL(ra, rb, rc),
            OpcodeRepr::SRLI => Opcode::SRLI(ra, rb, imm12),
            OpcodeRepr::SUB => Opcode::SUB(ra, rb, rc),
            OpcodeRepr::SUBI => Opcode::SUBI(ra, rb, imm12),
            OpcodeRepr::XOR => Opcode::XOR(ra, rb, rc),
            OpcodeRepr::XORI => Opcode::XORI(ra, rb, imm12),
            OpcodeRepr::CIMV => Opcode::CIMV(ra, rb, rc),
            OpcodeRepr::CTMV => Opcode::CTMV(ra, rb),
            OpcodeRepr::JI => Opcode::JI(imm24),
            OpcodeRepr::JNEI => Opcode::JNEI(ra, rb, imm12),
            OpcodeRepr::RET => Opcode::RET(ra),
            OpcodeRepr::CFEI => Opcode::CFEI(imm24),
            OpcodeRepr::CFSI => Opcode::CFSI(imm24),
            OpcodeRepr::LB => Opcode::LB(ra, rb, imm12),
            OpcodeRepr::LW => Opcode::LW(ra, rb, imm12),
            OpcodeRepr::ALOC => Opcode::ALOC(ra),
            OpcodeRepr::MCL => Opcode::MCL(ra, rb),
            OpcodeRepr::MCLI => Opcode::MCLI(ra, imm18),
            OpcodeRepr::MCP => Opcode::MCP(ra, rb, rc),
            OpcodeRepr::MEQ => Opcode::MEQ(ra, rb, rc, rd),
            OpcodeRepr::SB => Opcode::SB(ra, rb, imm12),
            OpcodeRepr::SW => Opcode::SW(ra, rb, imm12),
            OpcodeRepr::BHSH => Opcode::BHSH(ra, rb),
            OpcodeRepr::BHEI => Opcode::BHEI(ra),
            OpcodeRepr::BURN => Opcode::BURN(ra),
            OpcodeRepr::CALL => Opcode::CALL(ra, rb, rc, rd),
            OpcodeRepr::CCP => Opcode::CCP(ra, rb, rc, rd),
            OpcodeRepr::CROO => Opcode::CROO(ra, rb),
            OpcodeRepr::CSIZ => Opcode::CSIZ(ra, rb),
            OpcodeRepr::CB => Opcode::CB(ra),
            OpcodeRepr::LDC => Opcode::LDC(ra, rb, rc),
            OpcodeRepr::LOG => Opcode::LOG(ra, rb, rc, rd),
            OpcodeRepr::LOGD => Opcode::LOGD(ra, rb, rc, rd),
            OpcodeRepr::MINT => Opcode::MINT(ra),
            OpcodeRepr::RVRT => Opcode::RVRT(ra),
            OpcodeRepr::SLDC => Opcode::SLDC(ra, rb, rc),
            OpcodeRepr::SRW => Opcode::SRW(ra, rb),
            OpcodeRepr::SRWQ => Opcode::SRWQ(ra, rb),
            OpcodeRepr::SWW => Opcode::SWW(ra, rb),
            OpcodeRepr::SWWQ => Opcode::SWWQ(ra, rb),
            OpcodeRepr::TR => Opcode::TR(ra, rb, rc),
            OpcodeRepr::TRO => Opcode::TRO(ra, rb, rc, rd),
            OpcodeRepr::ECR => Opcode::ECR(ra, rb, rc),
            OpcodeRepr::K256 => Opcode::K256(ra, rb, rc),
            OpcodeRepr::S256 => Opcode::S256(ra, rb, rc),
            OpcodeRepr::XIL => Opcode::XIL(ra, rb),
            OpcodeRepr::XIS => Opcode::XIS(ra, rb),
            OpcodeRepr::XOL => Opcode::XOL(ra, rb),
            OpcodeRepr::XOS => Opcode::XOS(ra, rb),
            OpcodeRepr::XWL => Opcode::XWL(ra, rb),
            OpcodeRepr::XWS => Opcode::XWS(ra, rb),
            OpcodeRepr::NOOP => Opcode::NOOP,
            OpcodeRepr::FLAG => Opcode::FLAG(ra),
            _ => Opcode::Undefined,
        }
    }

    /// Create a `Opcode` from a slice of bytes
    ///
    /// # Panics
    ///
    /// This function will panic if the length of the bytes is smaller than
    /// [`Opcode::BYTES_SIZE`].
    pub fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        assert!(Self::BYTES_SIZE <= bytes.len());

        <[u8; Self::BYTES_SIZE]>::try_from(&bytes[..Self::BYTES_SIZE])
            .map(u32::from_be_bytes)
            .map(Self::from)
            .unwrap_or_else(|_| unreachable!())
    }

    /// Convert the opcode to bytes representation
    pub fn to_bytes(self) -> [u8; Self::BYTES_SIZE] {
        u32::from(self).to_be_bytes()
    }

    /// Transform the [`Opcode`] into an optional array of 4 register
    /// identifiers
    pub const fn registers(&self) -> [Option<RegisterId>; 4] {
        match self {
            Self::ADD(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::ADDI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::AND(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::ANDI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::DIV(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::DIVI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::EQ(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::EXP(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::EXPI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::GT(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::LT(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MLOG(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MROO(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MOD(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MODI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::MOVE(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::MUL(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MULI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::NOT(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::OR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::ORI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SLL(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SLLI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SRL(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SRLI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SUB(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SUBI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::XOR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::XORI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::CIMV(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::CTMV(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::JI(_) => [None; 4],
            Self::JNEI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::RET(ra) => [Some(*ra), None, None, None],
            Self::CFEI(_) => [None; 4],
            Self::CFSI(_) => [None; 4],
            Self::LB(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::LW(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::ALOC(ra) => [Some(*ra), None, None, None],
            Self::MCL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::MCLI(ra, _) => [Some(*ra), None, None, None],
            Self::MCP(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MEQ(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::SB(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SW(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::BHSH(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::BHEI(ra) => [Some(*ra), None, None, None],
            Self::BURN(ra) => [Some(*ra), None, None, None],
            Self::CALL(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::CCP(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::CROO(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::CSIZ(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::CB(ra) => [Some(*ra), None, None, None],
            Self::LDC(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::LOG(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::LOGD(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::MINT(ra) => [Some(*ra), None, None, None],
            Self::RVRT(ra) => [Some(*ra), None, None, None],
            Self::SLDC(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SRW(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::SRWQ(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::SWW(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::SWWQ(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::TR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::TRO(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::ECR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::K256(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::S256(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::XIL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XIS(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XOL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XOS(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XWL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XWS(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::NOOP => [None; 4],
            Self::FLAG(ra) => [Some(*ra), None, None, None],
            Self::Undefined => [None; 4],
        }
    }
}

#[cfg(feature = "std")]
impl Opcode {
    /// Create a `Opcode` from a slice of bytes
    ///
    /// This function will fail if the length of the bytes is smaller than
    /// [`Opcode::BYTES_SIZE`].
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < Self::BYTES_SIZE {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "The provided buffer is not big enough!",
            ))
        } else {
            Ok(Self::from_bytes_unchecked(bytes))
        }
    }
}

impl From<u32> for Opcode {
    fn from(instruction: u32) -> Self {
        // TODO Optimize with native architecture (eg SIMD?) or facilitate
        // auto-vectorization
        let op = (instruction >> 24) as u8;

        let ra = ((instruction >> 18) & 0x3f) as RegisterId;
        let rb = ((instruction >> 12) & 0x3f) as RegisterId;
        let rc = ((instruction >> 6) & 0x3f) as RegisterId;
        let rd = (instruction & 0x3f) as RegisterId;

        let imm06 = (instruction & 0xff) as Immediate06;
        let imm12 = (instruction & 0x0fff) as Immediate12;
        let imm18 = (instruction & 0x3ffff) as Immediate18;
        let imm24 = (instruction & 0xffffff) as Immediate24;

        Self::new(op, ra, rb, rc, rd, imm06, imm12, imm18, imm24)
    }
}

impl From<Opcode> for u32 {
    fn from(opcode: Opcode) -> u32 {
        match opcode {
            Opcode::ADD(ra, rb, rc) => {
                ((OpcodeRepr::ADD as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::ADDI(ra, rb, imm12) => {
                ((OpcodeRepr::ADDI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::AND(ra, rb, rc) => {
                ((OpcodeRepr::AND as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::ANDI(ra, rb, imm12) => {
                ((OpcodeRepr::ANDI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::DIV(ra, rb, rc) => {
                ((OpcodeRepr::DIV as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::DIVI(ra, rb, imm12) => {
                ((OpcodeRepr::DIVI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::EQ(ra, rb, rc) => {
                ((OpcodeRepr::EQ as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::EXP(ra, rb, rc) => {
                ((OpcodeRepr::EXP as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::EXPI(ra, rb, imm12) => {
                ((OpcodeRepr::EXPI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::GT(ra, rb, rc) => {
                ((OpcodeRepr::GT as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::LT(ra, rb, rc) => {
                ((OpcodeRepr::LT as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MLOG(ra, rb, rc) => {
                ((OpcodeRepr::MLOG as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MROO(ra, rb, rc) => {
                ((OpcodeRepr::MROO as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MOD(ra, rb, rc) => {
                ((OpcodeRepr::MOD as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MODI(ra, rb, imm12) => {
                ((OpcodeRepr::MODI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::MOVE(ra, rb) => {
                ((OpcodeRepr::MOVE as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::MUL(ra, rb, rc) => {
                ((OpcodeRepr::MUL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MULI(ra, rb, imm12) => {
                ((OpcodeRepr::MULI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::NOT(ra, rb) => {
                ((OpcodeRepr::NOT as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::OR(ra, rb, rc) => {
                ((OpcodeRepr::OR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::ORI(ra, rb, imm12) => {
                ((OpcodeRepr::ORI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SLL(ra, rb, rc) => {
                ((OpcodeRepr::SLL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SLLI(ra, rb, imm12) => {
                ((OpcodeRepr::SLLI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SRL(ra, rb, rc) => {
                ((OpcodeRepr::SRL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SRLI(ra, rb, imm12) => {
                ((OpcodeRepr::SRLI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SUB(ra, rb, rc) => {
                ((OpcodeRepr::SUB as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SUBI(ra, rb, imm12) => {
                ((OpcodeRepr::SUBI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::XOR(ra, rb, rc) => {
                ((OpcodeRepr::XOR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::XORI(ra, rb, imm12) => {
                ((OpcodeRepr::XORI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::CIMV(ra, rb, rc) => {
                ((OpcodeRepr::CIMV as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::CTMV(ra, rb) => {
                ((OpcodeRepr::CTMV as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::JI(imm24) => ((OpcodeRepr::JI as u32) << 24) | (imm24 as u32),
            Opcode::JNEI(ra, rb, imm12) => {
                ((OpcodeRepr::JNEI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::RET(ra) => ((OpcodeRepr::RET as u32) << 24) | ((ra as u32) << 18),
            Opcode::CFEI(imm24) => ((OpcodeRepr::CFEI as u32) << 24) | (imm24 as u32),
            Opcode::CFSI(imm24) => ((OpcodeRepr::CFSI as u32) << 24) | (imm24 as u32),
            Opcode::LB(ra, rb, imm12) => {
                ((OpcodeRepr::LB as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::LW(ra, rb, imm12) => {
                ((OpcodeRepr::LW as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::ALOC(ra) => ((OpcodeRepr::ALOC as u32) << 24) | ((ra as u32) << 18),
            Opcode::MCL(ra, rb) => {
                ((OpcodeRepr::MCL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::MCLI(ra, imm18) => {
                ((OpcodeRepr::MCLI as u32) << 24) | ((ra as u32) << 18) | (imm18 as u32)
            }
            Opcode::MCP(ra, rb, rc) => {
                ((OpcodeRepr::MCP as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MEQ(ra, rb, rc, rd) => {
                ((OpcodeRepr::MEQ as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::SB(ra, rb, imm12) => {
                ((OpcodeRepr::SB as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SW(ra, rb, imm12) => {
                ((OpcodeRepr::SW as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::BHSH(ra, rb) => {
                ((OpcodeRepr::BHSH as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::BHEI(ra) => ((OpcodeRepr::BHEI as u32) << 24) | ((ra as u32) << 18),
            Opcode::BURN(ra) => ((OpcodeRepr::BURN as u32) << 24) | ((ra as u32) << 18),
            Opcode::CALL(ra, rb, rc, rd) => {
                ((OpcodeRepr::CALL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::CCP(ra, rb, rc, rd) => {
                ((OpcodeRepr::CCP as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::CROO(ra, rb) => {
                ((OpcodeRepr::CROO as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::CSIZ(ra, rb) => {
                ((OpcodeRepr::CSIZ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::CB(ra) => ((OpcodeRepr::CB as u32) << 24) | ((ra as u32) << 18),
            Opcode::LDC(ra, rb, rc) => {
                ((OpcodeRepr::LDC as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::LOG(ra, rb, rc, rd) => {
                ((OpcodeRepr::LOG as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::LOGD(ra, rb, rc, rd) => {
                ((OpcodeRepr::LOGD as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::MINT(ra) => ((OpcodeRepr::MINT as u32) << 24) | ((ra as u32) << 18),
            Opcode::RVRT(ra) => ((OpcodeRepr::RVRT as u32) << 24) | ((ra as u32) << 18),
            Opcode::SLDC(ra, rb, rc) => {
                ((OpcodeRepr::SLDC as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SRW(ra, rb) => {
                ((OpcodeRepr::SRW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::SRWQ(ra, rb) => {
                ((OpcodeRepr::SRWQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::SWW(ra, rb) => {
                ((OpcodeRepr::SWW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::SWWQ(ra, rb) => {
                ((OpcodeRepr::SWWQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::TR(ra, rb, rc) => {
                ((OpcodeRepr::TR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::TRO(ra, rb, rc, rd) => {
                ((OpcodeRepr::TRO as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::ECR(ra, rb, rc) => {
                ((OpcodeRepr::ECR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::K256(ra, rb, rc) => {
                ((OpcodeRepr::K256 as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::S256(ra, rb, rc) => {
                ((OpcodeRepr::S256 as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::XIL(ra, rb) => {
                ((OpcodeRepr::XIL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XIS(ra, rb) => {
                ((OpcodeRepr::XIS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XOL(ra, rb) => {
                ((OpcodeRepr::XOL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XOS(ra, rb) => {
                ((OpcodeRepr::XOS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XWL(ra, rb) => {
                ((OpcodeRepr::XWL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XWS(ra, rb) => {
                ((OpcodeRepr::XWS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::NOOP => (OpcodeRepr::NOOP as u32) << 24,
            Opcode::FLAG(ra) => ((OpcodeRepr::FLAG as u32) << 24) | ((ra as u32) << 18),
            Opcode::Undefined => (0x00 << 24),
        }
    }
}

#[cfg(feature = "std")]
impl io::Read for Opcode {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        buf.chunks_exact_mut(4)
            .next()
            .map(|chunk| chunk.copy_from_slice(&u32::from(*self).to_be_bytes()))
            .map(|_| 4)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "The provided buffer is not big enough!",
                )
            })
    }
}

#[cfg(feature = "std")]
impl io::Write for Opcode {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        buf.chunks_exact(4)
            .next()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "The provided buffer is not big enough!",
                )
            })
            .map(|bytes| *self = Self::from_bytes_unchecked(bytes))
            .map(|_| 4)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "std")]
impl iter::FromIterator<Opcode> for Vec<u8> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Opcode>,
    {
        iter.into_iter().map(Opcode::to_bytes).flatten().collect()
    }
}
