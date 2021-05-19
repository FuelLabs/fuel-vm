use crate::types::{Immediate06, Immediate12, Immediate18, Immediate24, RegisterId};

use consts::*;

use std::convert::TryFrom;
use std::io;

pub mod consts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
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
    ADD(RegisterId, RegisterId, RegisterId) = OP_ADD,

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
    ADDI(RegisterId, RegisterId, Immediate12) = OP_ADDI,

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
    AND(RegisterId, RegisterId, RegisterId) = OP_AND,

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
    ANDI(RegisterId, RegisterId, Immediate12) = OP_ANDI,

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
    DIV(RegisterId, RegisterId, RegisterId) = OP_DIV,

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
    DIVI(RegisterId, RegisterId, Immediate12) = OP_DIVI,

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
    EQ(RegisterId, RegisterId, RegisterId) = OP_EQ,

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
    EXP(RegisterId, RegisterId, RegisterId) = OP_EXP,

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
    EXPI(RegisterId, RegisterId, Immediate12) = OP_EXPI,

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
    GT(RegisterId, RegisterId, RegisterId) = OP_GT,

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
    MLOG(RegisterId, RegisterId, RegisterId) = OP_MLOG,

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
    MROO(RegisterId, RegisterId, RegisterId) = OP_MROO,

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
    MOD(RegisterId, RegisterId, RegisterId) = OP_MOD,

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
    MODI(RegisterId, RegisterId, Immediate12) = OP_MODI,

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
    MOVE(RegisterId, RegisterId) = OP_MOVE,

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
    MUL(RegisterId, RegisterId, RegisterId) = OP_MUL,

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
    MULI(RegisterId, RegisterId, Immediate12) = OP_MULI,

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
    NOT(RegisterId, RegisterId) = OP_NOT,

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
    OR(RegisterId, RegisterId, RegisterId) = OP_OR,

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
    ORI(RegisterId, RegisterId, Immediate12) = OP_ORI,

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
    SLL(RegisterId, RegisterId, RegisterId) = OP_SLL,

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
    SLLI(RegisterId, RegisterId, Immediate12) = OP_SLLI,

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
    SRL(RegisterId, RegisterId, RegisterId) = OP_SRL,

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
    SRLI(RegisterId, RegisterId, Immediate12) = OP_SRLI,

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
    SUB(RegisterId, RegisterId, RegisterId) = OP_SUB,

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
    SUBI(RegisterId, RegisterId, Immediate12) = OP_SUBI,

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
    XOR(RegisterId, RegisterId, RegisterId) = OP_XOR,

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
    XORI(RegisterId, RegisterId, Immediate12) = OP_XORI,

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
    CIMV(RegisterId, RegisterId, RegisterId) = OP_CIMV,

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
    CTMV(RegisterId, RegisterId) = OP_CTMV,

    /// Jumps to the code instruction offset by `imm`.
    ///
    /// | Operation   | ```$pc = $is + imm * 4;```                     |
    /// | Syntax      | `ji imm`                                       |
    /// | Encoding    | `0x00 i i i i`                                 |
    ///
    /// #### Panics
    /// - `$is + imm * 4 > VM_MAX_RAM - 1`
    JI(Immediate24) = OP_JI,

    /// Jump to the code instruction offset by `imm` if `$rA` is not equal to
    /// `$rB`.
    ///
    /// | Operation   | ```if $rA != $rB:```<br>```$pc = $is + imm *
    /// 4;```<br>```else:```<br>```$pc += 4;``` | Syntax      | `jnei $rA
    /// $rB imm` | Encoding    | `0x00 rA rB i i`
    ///
    /// #### Panics
    /// - `$is + imm * 4 > VM_MAX_RAM - 1`
    JNEI(RegisterId, RegisterId, Immediate12) = OP_JNEI,

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
    RET(RegisterId) = OP_RET,

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
    CFEI(Immediate24) = OP_CFEI,

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
    CFSI(Immediate24) = OP_CFSI,

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
    LB(RegisterId, RegisterId, Immediate12) = OP_LB,

    /// A word is loaded from the specified address offset by `imm`.
    /// | Operation   | ```$rA = MEM[$rB + imm, 8];```
    /// | Syntax      | `lw $rA, $rB, imm`
    /// | Encoding    | `0x00 rA rB i i`
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    /// - `$rB + imm + 8` overflows
    /// - `$rB + imm + 8 > VM_MAX_RAM`
    LW(RegisterId, RegisterId, Immediate12) = OP_LW,

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
    ALOC(RegisterId) = OP_ALOC,

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
    MCL(RegisterId, RegisterId) = OP_MCL,

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
    MCLI(RegisterId, Immediate18) = OP_MCLI,

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
    MCP(RegisterId, RegisterId, RegisterId) = OP_MCP,

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
    MEQ(RegisterId, RegisterId, RegisterId, RegisterId) = OP_MEQ,

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
    SB(RegisterId, RegisterId, Immediate12) = OP_SB,

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
    SW(RegisterId, RegisterId, Immediate12) = OP_SW,

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
    BHSH(RegisterId, RegisterId) = OP_BHSH,

    /// Get Fuel block height.
    ///
    /// | Operation   | ```$rA = blockheight();``` |
    /// | Syntax      | `bhei $rA`                 |
    /// | Encoding    | `0x00 rA - - -`            |
    ///
    /// #### Panics
    /// - `$rA` is a [reserved register](./main.md#semantics)
    BHEI(RegisterId) = OP_BHEI,

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
    BURN(RegisterId) = OP_BURN,

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
    CALL(RegisterId, RegisterId, RegisterId, RegisterId) = OP_CALL,

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
    CCP(RegisterId, RegisterId, RegisterId, RegisterId) = OP_CCP,

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
    CROO(RegisterId, RegisterId) = OP_CROO,

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
    CSIZ(RegisterId, RegisterId) = OP_CSIZ,

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
    CB(RegisterId) = OP_CB,

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
    LDC(RegisterId, RegisterId, RegisterId) = OP_LDC,

    /// Log an event. This is a no-op.
    ///
    /// | Operation   | ```log($rA, $rB, $rC, $rD);``` |
    /// | Syntax      | `log $rA, $rB, $rC, $rD`       |
    /// | Encoding    | `0x00 rA rB rC rD`             |
    LOG(RegisterId, RegisterId, RegisterId, RegisterId) = OP_LOG,

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
    MINT(RegisterId) = OP_MINT,

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
    RVRT(RegisterId) = OP_RVRT,

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
    SLDC(RegisterId, RegisterId, RegisterId) = OP_SLDC,

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
    SRW(RegisterId, RegisterId) = OP_SRW,

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
    SRWQ(RegisterId, RegisterId) = OP_SRWQ,

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
    SWW(RegisterId, RegisterId) = OP_SWW,

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
    SWWQ(RegisterId, RegisterId) = OP_SWWQ,

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
    TR(RegisterId, RegisterId, RegisterId) = OP_TR,

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
    TRO(RegisterId, RegisterId, RegisterId, RegisterId) = OP_TRO,

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
    ECR(RegisterId, RegisterId, RegisterId) = OP_ECR,

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
    K256(RegisterId, RegisterId, RegisterId) = OP_K256,

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
    S256(RegisterId, RegisterId, RegisterId) = OP_S256,

    /// Set `$rA` to the length in bytes of [the `$rB`th
    /// input](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xil($rB);```      |
    /// | Syntax      | `xil $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.inputsCount`
    XIL(RegisterId, RegisterId) = OP_XIL,

    /// Set `$rA` to the memory addess of the start of [the `$rB`th
    /// input](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xis($rB);```      |
    /// | Syntax      | `xis $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.inputsCount`
    XIS(RegisterId, RegisterId) = OP_XIS,

    /// Set `$rA` to the length in bytes of [the `$rB`th
    /// output](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xol($rB);```      |
    /// | Syntax      | `xol $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.outputsCount`
    XOL(RegisterId, RegisterId) = OP_XOL,

    /// Set `$rA` to the memory addess of the start of [the `$rB`th
    /// output](./main.md#vm-initialization).
    ///
    /// | Operation   | ```$rA = xos($rB);```      |
    /// | Syntax      | `xos $rA, $rB`             |
    /// | Encoding    | `0x00 rA rB - -`           |
    ///
    /// #### Panics
    /// - `$rB >= tx.outputsCount`
    XOS(RegisterId, RegisterId) = OP_XOS,

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
    XWL(RegisterId, RegisterId) = OP_XWL,

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
    XWS(RegisterId, RegisterId) = OP_XWS,

    /// Performs no operation.
    ///
    /// | Operation   |                        |
    /// | Syntax      | `noop`                 |
    /// | Encoding    | `0x00 - - - -`         |
    ///
    /// `$of` and `$err` are cleared.
    NOOP = OP_NOOP,

    /// Set `$flag` to `$rA`.
    ///
    /// | Operation   | ```$flag = $rA;```    |
    /// | Syntax      | `flag $rA`            |
    /// | Encoding    | `0x00 rA - - -`       |
    FLAG(RegisterId) = OP_FLAG,

    /// Undefined opcode, potentially from inconsistent serialization
    Undefined = 0x0f,
}

impl Opcode {
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
        use Opcode::*;

        match op {
            OP_ADD => ADD(ra, rb, rc),
            OP_ADDI => ADDI(ra, rb, imm12),
            OP_AND => AND(ra, rb, rc),
            OP_ANDI => ANDI(ra, rb, imm12),
            OP_DIV => DIV(ra, rb, rc),
            OP_DIVI => DIVI(ra, rb, imm12),
            OP_EQ => EQ(ra, rb, rc),
            OP_EXP => EXP(ra, rb, rc),
            OP_EXPI => EXPI(ra, rb, imm12),
            OP_GT => GT(ra, rb, rc),
            OP_MLOG => MLOG(ra, rb, rc),
            OP_MROO => MROO(ra, rb, rc),
            OP_MOD => MOD(ra, rb, rc),
            OP_MODI => MODI(ra, rb, imm12),
            OP_MOVE => MOVE(ra, rb),
            OP_MUL => MUL(ra, rb, rc),
            OP_MULI => MULI(ra, rb, imm12),
            OP_NOT => NOT(ra, rb),
            OP_OR => OR(ra, rb, rc),
            OP_ORI => ORI(ra, rb, imm12),
            OP_SLL => SLL(ra, rb, rc),
            OP_SLLI => SLLI(ra, rb, imm12),
            OP_SRL => SRL(ra, rb, rc),
            OP_SRLI => SRLI(ra, rb, imm12),
            OP_SUB => SUB(ra, rb, rc),
            OP_SUBI => SUBI(ra, rb, imm12),
            OP_XOR => XOR(ra, rb, rc),
            OP_XORI => XORI(ra, rb, imm12),
            OP_CIMV => CIMV(ra, rb, rc),
            OP_CTMV => CTMV(ra, rb),
            OP_JI => JI(imm24),
            OP_JNEI => JNEI(ra, rb, imm12),
            OP_RET => RET(ra),
            OP_CFEI => CFEI(imm24),
            OP_CFSI => CFSI(imm24),
            OP_LB => LB(ra, rb, imm12),
            OP_LW => LW(ra, rb, imm12),
            OP_ALOC => ALOC(ra),
            OP_MCL => MCL(ra, rb),
            OP_MCLI => MCLI(ra, imm18),
            OP_MCP => MCP(ra, rb, rc),
            OP_MEQ => MEQ(ra, rb, rc, rd),
            OP_SB => SB(ra, rb, imm12),
            OP_SW => SW(ra, rb, imm12),
            OP_BHSH => BHSH(ra, rb),
            OP_BHEI => BHEI(ra),
            OP_BURN => BURN(ra),
            OP_CALL => CALL(ra, rb, rc, rd),
            OP_CCP => CCP(ra, rb, rc, rd),
            OP_CROO => CROO(ra, rb),
            OP_CSIZ => CSIZ(ra, rb),
            OP_CB => CB(ra),
            OP_LDC => LDC(ra, rb, rc),
            OP_LOG => LOG(ra, rb, rc, rd),
            OP_MINT => MINT(ra),
            OP_RVRT => RVRT(ra),
            OP_SLDC => SLDC(ra, rb, rc),
            OP_SRW => SRW(ra, rb),
            OP_SRWQ => SRWQ(ra, rb),
            OP_SWW => SWW(ra, rb),
            OP_SWWQ => SWWQ(ra, rb),
            OP_TR => TR(ra, rb, rc),
            OP_TRO => TRO(ra, rb, rc, rd),
            OP_ECR => ECR(ra, rb, rc),
            OP_K256 => K256(ra, rb, rc),
            OP_S256 => S256(ra, rb, rc),
            OP_XIL => XIL(ra, rb),
            OP_XIS => XIS(ra, rb),
            OP_XOL => XOL(ra, rb),
            OP_XOS => XOS(ra, rb),
            OP_XWL => XWL(ra, rb),
            OP_XWS => XWS(ra, rb),
            OP_NOOP => NOOP,
            OP_FLAG => FLAG(ra),
            _ => Undefined,
        }
    }

    /// Gas cost for this operation
    pub const fn gas_cost(&self) -> u64 {
        // TODO define gas costs
        1
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
                ((OP_ADD as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::ADDI(ra, rb, imm12) => {
                ((OP_ADDI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::AND(ra, rb, rc) => {
                ((OP_AND as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::ANDI(ra, rb, imm12) => {
                ((OP_ANDI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::DIV(ra, rb, rc) => {
                ((OP_DIV as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::DIVI(ra, rb, imm12) => {
                ((OP_DIVI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::EQ(ra, rb, rc) => {
                ((OP_EQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::EXP(ra, rb, rc) => {
                ((OP_EXP as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::EXPI(ra, rb, imm12) => {
                ((OP_EXPI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::GT(ra, rb, rc) => {
                ((OP_GT as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::MLOG(ra, rb, rc) => {
                ((OP_MLOG as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::MROO(ra, rb, rc) => {
                ((OP_MROO as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::MOD(ra, rb, rc) => {
                ((OP_MOD as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::MODI(ra, rb, imm12) => {
                ((OP_MODI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::MOVE(ra, rb) => ((OP_MOVE as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::MUL(ra, rb, rc) => {
                ((OP_MUL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::MULI(ra, rb, imm12) => {
                ((OP_MULI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::NOT(ra, rb) => ((OP_NOT as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::OR(ra, rb, rc) => {
                ((OP_OR as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::ORI(ra, rb, imm12) => {
                ((OP_ORI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::SLL(ra, rb, rc) => {
                ((OP_SLL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::SLLI(ra, rb, imm12) => {
                ((OP_SLLI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::SRL(ra, rb, rc) => {
                ((OP_SRL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::SRLI(ra, rb, imm12) => {
                ((OP_SRLI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::SUB(ra, rb, rc) => {
                ((OP_SUB as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::SUBI(ra, rb, imm12) => {
                ((OP_SUBI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::XOR(ra, rb, rc) => {
                ((OP_XOR as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::XORI(ra, rb, imm12) => {
                ((OP_XORI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::CIMV(ra, rb, rc) => {
                ((OP_CIMV as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::CTMV(ra, rb) => ((OP_CTMV as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::JI(imm24) => ((OP_JI as u32) << 24) | (imm24 as u32),
            Opcode::JNEI(ra, rb, imm12) => {
                ((OP_JNEI as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::RET(ra) => ((OP_RET as u32) << 24) | ((ra as u32) << 18),
            Opcode::CFEI(imm24) => ((OP_CFEI as u32) << 24) | (imm24 as u32),
            Opcode::CFSI(imm24) => ((OP_CFSI as u32) << 24) | (imm24 as u32),
            Opcode::LB(ra, rb, imm12) => {
                ((OP_LB as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::LW(ra, rb, imm12) => {
                ((OP_LW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::ALOC(ra) => ((OP_ALOC as u32) << 24) | ((ra as u32) << 18),
            Opcode::MCL(ra, rb) => ((OP_MCL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::MCLI(ra, imm18) => ((OP_MCLI as u32) << 24) | ((ra as u32) << 18) | (imm18 as u32),
            Opcode::MCP(ra, rb, rc) => {
                ((OP_MCP as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::MEQ(ra, rb, rc, rd) => {
                ((OP_MEQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6) | (rd as u32)
            }
            Opcode::SB(ra, rb, imm12) => {
                ((OP_SB as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::SW(ra, rb, imm12) => {
                ((OP_SW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | (imm12 as u32)
            }
            Opcode::BHSH(ra, rb) => ((OP_BHSH as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::BHEI(ra) => ((OP_BHEI as u32) << 24) | ((ra as u32) << 18),
            Opcode::BURN(ra) => ((OP_BURN as u32) << 24) | ((ra as u32) << 18),
            Opcode::CALL(ra, rb, rc, rd) => {
                ((OP_CALL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6) | (rd as u32)
            }
            Opcode::CCP(ra, rb, rc, rd) => {
                ((OP_CCP as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6) | (rd as u32)
            }
            Opcode::CROO(ra, rb) => ((OP_CROO as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::CSIZ(ra, rb) => ((OP_CSIZ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::CB(ra) => ((OP_CB as u32) << 24) | ((ra as u32) << 18),
            Opcode::LDC(ra, rb, rc) => {
                ((OP_LDC as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::LOG(ra, rb, rc, rd) => {
                ((OP_LOG as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6) | (rd as u32)
            }
            Opcode::MINT(ra) => ((OP_MINT as u32) << 24) | ((ra as u32) << 18),
            Opcode::RVRT(ra) => ((OP_RVRT as u32) << 24) | ((ra as u32) << 18),
            Opcode::SLDC(ra, rb, rc) => {
                ((OP_SLDC as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::SRW(ra, rb) => ((OP_SRW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::SRWQ(ra, rb) => ((OP_SRWQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::SWW(ra, rb) => ((OP_SWW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::SWWQ(ra, rb) => ((OP_SWWQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::TR(ra, rb, rc) => {
                ((OP_TR as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::TRO(ra, rb, rc, rd) => {
                ((OP_TRO as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6) | (rd as u32)
            }
            Opcode::ECR(ra, rb, rc) => {
                ((OP_ECR as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::K256(ra, rb, rc) => {
                ((OP_K256 as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::S256(ra, rb, rc) => {
                ((OP_S256 as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12) | ((rc as u32) << 6)
            }
            Opcode::XIL(ra, rb) => ((OP_XIL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::XIS(ra, rb) => ((OP_XIS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::XOL(ra, rb) => ((OP_XOL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::XOS(ra, rb) => ((OP_XOS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::XWL(ra, rb) => ((OP_XWL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::XWS(ra, rb) => ((OP_XWS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12),
            Opcode::NOOP => (OP_NOOP as u32) << 24,
            Opcode::FLAG(ra) => ((OP_FLAG as u32) << 24) | ((ra as u32) << 18),
            Opcode::Undefined => (0x00 << 24),
        }
    }
}

impl io::Read for Opcode {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        buf.chunks_exact_mut(4)
            .next()
            .map(|chunk| chunk.copy_from_slice(&u32::from(*self).to_be_bytes()))
            .map(|_| 4)
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "The provided buffer is not big enough!"))
    }
}

impl io::Write for Opcode {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        buf.chunks_exact(4)
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "The provided buffer is not big enough!"))
            .and_then(|chunk| <[u8; 4]>::try_from(chunk).map_err(|_| unreachable!()))
            .map(|bytes| *self = u32::from_be_bytes(bytes).into())
            .map(|_| 4)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
