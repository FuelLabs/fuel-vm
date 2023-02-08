use fuel_asm::*;
use fuel_tx::{ConsensusParameters, Receipt, Transaction};
use fuel_vm::prelude::{IntoChecked, MemoryClient};

pub struct CaseGroup {
    name: &'static str,
    setup: Vec<Instruction>,
    cases: Vec<Instruction>,
    check: fn(&Res) -> bool,
}

pub struct Res {
    a: Word,
    b: Word,
    of: Word,
    err: Word,
}

#[test]
fn test_arith_props() {
    let mut client = MemoryClient::default();

    // Setup some useful values
    // * 0x30 to all ones, i.e. max word
    let common_setup = [op::not(0x30, RegId::ZERO)];

    let groups = vec![
        CaseGroup {
            name: "unsafemath flag",
            setup: vec![op::addi(0x10, RegId::ZERO, 0x01), op::flag(0x10)],
            cases: vec![
                op::div(0x10, RegId::ZERO, RegId::ZERO),
                op::divi(0x10, RegId::ZERO, 0),
                op::mlog(0x10, 0x30, RegId::ZERO),
                op::mod_(0x10, 0x30, RegId::ZERO),
                op::modi(0x10, 0x30, 0),
                op::mroo(0x10, 0x30, RegId::ZERO),
            ],
            check: |res| res.err == 1,
        },
        CaseGroup {
            name: "wrapping flag",
            setup: vec![op::addi(0x10, RegId::ZERO, 0x02), op::flag(0x10)],
            cases: vec![
                op::add(0x10, 0x30, 0x30),
                op::addi(0x10, 0x30, 0x30),
                op::exp(0x10, 0x30, 0x30),
                op::expi(0x10, 0x30, 0x30),
                op::mul(0x10, 0x30, 0x30),
                op::muli(0x10, 0x30, 0x30),
                op::sub(0x10, RegId::ZERO, RegId::ONE),
                op::subi(0x10, RegId::ZERO, 1),
            ],
            check: |res| res.of != 0,
        },
    ];

    for group in groups {
        for case in group.cases {
            let mut script = common_setup.to_vec();
            script.extend(&group.setup);
            script.push(case);
            script.push(op::log(0x10, 0x11, RegId::OF, RegId::ERR));
            script.push(op::ret(RegId::ONE));
            let script = script.into_iter().collect::<Vec<u8>>();
            let tx = Transaction::script(0, 1_000_000, 0, script, vec![], vec![], vec![], vec![])
                .into_checked(0, &ConsensusParameters::DEFAULT, client.gas_costs())
                .expect("failed to generate a checked tx");

            client.transact(tx);

            let receipts = client.receipts().expect("Expected receipts");
            if let Receipt::Log { ra, rb, rc, rd, .. } = receipts[0] {
                let res = Res {
                    a: ra,
                    b: rb,
                    of: rc,
                    err: rd,
                };
                assert!((group.check)(&res), "Check failed for case {}: {:?}", group.name, case);
            } else {
                panic!("No log data for case {}: {:?}", group.name, case);
            }
        }
    }
}
