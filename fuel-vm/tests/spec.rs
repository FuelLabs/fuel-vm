use std::fmt;

use fuel_asm::*;
use fuel_tx::{ConsensusParameters, Receipt, ScriptExecutionResult, Transaction};
use fuel_vm::prelude::{IntoChecked, MemoryClient};

struct CaseGroup {
    /// A short description for diagnostic purposes.
    name: &'static str,
    /// Setup done before the cases.
    setup: Vec<Instruction>,
    /// Each case is just a simple instruction.
    /// They are ran in separate transactions, each prepended by the setup.
    cases: Vec<Instruction>,
    /// A function checking that the case did run correctly.
    check: fn(CaseResult<'_>) -> bool,
    /// A function checking that the case without setup did run as it should.
    /// This is useful for i.e. checking cases with and without flags set.
    check_nosetup: fn(CaseResult<'_>) -> bool,
}
impl fmt::Debug for CaseGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CaseGroup")
            .field("name", &self.name)
            .field("setup", &self.setup)
            .field("cases", &self.cases)
            .field("check", &"[closure]")
            .field("check_nosetup", &"[closure]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
struct CaseResultRegs {
    a: Word,
    b: Word,
    c: Word,
    d: Word,
    is: Word,
    pc: Word,
    of: Word,
    err: Word,
}

#[derive(Debug)]
struct CaseResult<'a> {
    group: &'a CaseGroup,
    case: Instruction,
    receipts: &'a [Receipt],
    setup_size: Word,
}
impl<'a> CaseResult<'a> {
    fn regs(&self) -> CaseResultRegs {
        if let Receipt::Log { ra: is, rb: pc, rc: of, rd: err, .. } = self.receipts[0] {
            if let Receipt::Log { ra: a, rb: b, rc: c, rd: d, .. } = self.receipts[1] {
                return CaseResultRegs {
                    a,
                    b,
                    c,
                    d,
                    is,
                    pc,
                    of,
                    err,
                };
            }
        }
        dbg!(self);
        panic!("No log data for case {}: {:?}", self.group.name, self.case);
    }

    fn assert_panics(&self, reason: PanicReason) {
        if let Receipt::ScriptResult { result, .. } = self.receipts.last().unwrap() {
            if *result != ScriptExecutionResult::Panic {
                panic!("No panic for case {}: {:?}", self.group.name, self.case);
            }
        } else {
            unreachable!("No script result for case {}: {:?}", self.group.name, self.case);
        }

        let n = self.receipts.len();
        assert!(n >= 2, "Invalid receipts len");
        if let Receipt::Panic { reason: pr, .. } = self.receipts.get(n - 2).unwrap() {
            assert_eq!(
                reason,
                *pr.reason(),
                "Panic reason for case {}: {:?}",
                self.group.name,
                self.case
            );
        } else {
            unreachable!("No panic receipt for case {}: {:?}", self.group.name, self.case);
        }
    }
}

#[test]
fn test_arith_props() {
    let mut client = MemoryClient::default();

    // Setup some useful values
    // * 0x31 to all ones, i.e. max word
    // * 0x32 to two
    let common_setup = [
        op::not(0x31, RegId::ZERO),
        op::movi(0x32, 2),
    ];

    let groups = vec![
        CaseGroup {
            name: "unsafemath flag",
            setup: vec![op::addi(0x10, RegId::ZERO, 0x01), op::flag(0x10)],
            cases: vec![
                op::div(0x10, RegId::ZERO, RegId::ZERO),
                op::divi(0x10, RegId::ZERO, 0),
                op::mlog(0x10, 0x31, RegId::ZERO),
                op::mod_(0x10, 0x31, RegId::ZERO),
                op::modi(0x10, 0x31, 0),
                op::mroo(0x10, 0x31, RegId::ZERO),
            ],

            check: |res| res.regs().err == 1,
            check_nosetup: |res| {
                res.assert_panics(PanicReason::ErrorFlag);
                true
            },
        },
        CaseGroup {
            name: "wrapping flag",
            setup: vec![op::addi(0x10, RegId::ZERO, 0x02), op::flag(0x10)],
            cases: vec![
                op::add(0x10, 0x31, 0x31),
                op::addi(0x10, 0x31, 0x31),
                op::exp(0x10, 0x31, 0x31),
                op::expi(0x10, 0x31, 0x31),
                op::mul(0x10, 0x31, 0x31),
                op::muli(0x10, 0x31, 0x31),
                op::sub(0x10, RegId::ZERO, RegId::ONE),
                op::subi(0x10, RegId::ZERO, 1),
            ],

            check: |res| res.regs().of != 0,
            check_nosetup: |res| {
                res.assert_panics(PanicReason::ArithmeticOverflow);
                true
            },
        },
        CaseGroup {
            name: "cannot write to a reserved register",
            setup: vec![],
            cases: cases_try_write_reserved(),

            check: |res| {
                res.assert_panics(PanicReason::ReservedRegisterNotWritable);
                true
            },
            check_nosetup: |_res| true,
        },
        CaseGroup {
            name: "increments pc by four",
            setup: vec![op::noop()],
            cases: cases_incr_pc(),
            check: |res| res.regs().is + res.setup_size + 4 == res.regs().pc,
            check_nosetup: |res| res.regs().is + res.setup_size + 4 == res.regs().pc,
        },
    ];

    for group in groups {
        for case in group.cases.iter().copied() {
            for do_group_setup in [true, false] {
                let mut script = common_setup.to_vec();
                if do_group_setup {
                    script.extend(&group.setup);
                }
                let setup_size = (script.len() as Word) * 4;
                script.push(case);
                script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
                script.push(op::log(0x10, 0x11, 0x12, 0x13));
                script.push(op::ret(RegId::ONE));
                let script = script.into_iter().collect::<Vec<u8>>();
                let tx = Transaction::script(0, 1_000_000, 0, script, vec![], vec![], vec![], vec![])
                    .into_checked(0, &ConsensusParameters::DEFAULT, client.gas_costs())
                    .expect("failed to generate a checked tx");

                client.transact(tx);

                let receipts = client.receipts().expect("Expected receipts");
                let res = CaseResult {
                    group: &group,
                    case,
                    receipts,
                    setup_size,
                };

                if do_group_setup {
                    assert!((group.check)(res), "Check failed for case {}: {:?}", group.name, case);
                } else {
                    assert!(
                        (group.check_nosetup)(res),
                        "Nosetup check failed for case {}: {:?}",
                        group.name,
                        case
                    );
                }
            }
        }
    }
}

fn cases_try_write_reserved() -> Vec<Instruction> {
    let mut cases = Vec::new();
    for r in 0..RegId::WRITABLE.to_u8() {
        cases.extend(&[
            op::add(r, 0, 0),
            op::addi(r, 0, 0),
            op::and(r, 0, 0),
            op::andi(r, 0, 0),
            op::div(r, 0, 0),
            op::divi(r, 0, 0),
            op::eq(r, 0, 0),
            op::exp(r, 0, 0),
            op::expi(r, 0, 0),
            op::gt(r, 0, 0),
            op::lt(r, 0, 0),
            op::mlog(r, 0, 0),
            op::mod_(r, 0, 0),
            op::modi(r, 0, 0),
            op::move_(r, 0),
            op::movi(r, 0),
            op::mroo(r, 0, 0),
            op::mul(r, 0, 0),
            op::muli(r, 0, 0),
            op::not(r, 0),
            op::or(r, 0, 0),
            op::ori(r, 0, 0),
            op::sll(r, 0, 0),
            op::slli(r, 0, 0),
            op::srl(r, 0, 0),
            op::srli(r, 0, 0),
            op::sub(r, 0, 0),
            op::subi(r, 0, 0),
            op::xor(r, 0, 0),
            op::xori(r, 0, 0),
        ]);
    }
    cases
}

fn cases_incr_pc() -> Vec<Instruction> {
    let mut cases = Vec::new();
    let r = RegId::WRITABLE;
    cases.extend(&[
        op::add(r, 0, 0),
        op::addi(r, 0, 0),
        op::and(r, 0, 0),
        op::andi(r, 0, 0),
        op::div(r, 0, 1),
        op::divi(r, 0, 1),
        op::eq(r, 0, 0),
        op::exp(r, 0, 0),
        op::expi(r, 0, 0),
        op::gt(r, 0, 0),
        op::lt(r, 0, 0),
        op::mlog(r, 1, 0x32),
        op::mod_(r, 0, 1),
        op::modi(r, 0, 1),
        op::move_(r, 0),
        op::movi(r, 0),
        op::mroo(r, 0, 0x32),
        op::mul(r, 0, 0),
        op::muli(r, 0, 0),
        op::not(r, 0),
        op::or(r, 0, 0),
        op::ori(r, 0, 0),
        op::sll(r, 0, 0),
        op::slli(r, 0, 0),
        op::srl(r, 0, 0),
        op::srli(r, 0, 0),
        op::sub(r, 0, 0),
        op::subi(r, 0, 0),
        op::xor(r, 0, 0),
        op::xori(r, 0, 0),
    ]);
    cases
}
