use alloc::{
    vec,
    vec::Vec,
};

use rand::Rng;
use rstest::rstest;
use test_case::test_case;

use fuel_asm::{
    op,
    GTFArgs,
    Instruction,
    RegId,
    Word,
};
use fuel_tx::{
    field::Outputs,
    Address,
    AssetId,
    ContractId,
    ContractIdExt,
    Output,
    PanicReason,
    Receipt,
};
use fuel_types::{
    canonical::Serialize,
    SubAssetId,
};

use crate::{
    call::Call,
    consts::VM_MAX_RAM,
    prelude::TestBuilder,
    tests::test_helpers::set_full_word,
    util::test_helpers::find_change,
};

use super::test_helpers::RunResult;

fn run(mut test_context: TestBuilder, call_contract_id: ContractId) -> Vec<Receipt> {
    let script_ops = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ];
    let script_data: Vec<u8> = [Call::new(call_contract_id, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(call_contract_id)
        .fee_input()
        .contract_output(&call_contract_id)
        .execute()
        .receipts()
        .to_vec()
}
fn first_log(receipts: &[Receipt]) -> Option<Word> {
    receipts
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .next()
}

fn first_tro(receipts: &[Receipt]) -> Option<Word> {
    receipts
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::TransferOut { amount, .. } => Some(*amount),
            _ => None,
        })
        .next()
}

const REG_DATA_PTR: u8 = 0x3f;
const REG_HELPER: u8 = 0x3e;

#[test_case(0, 0, op::bal(0x20, RegId::HP, REG_DATA_PTR) => RunResult::Success(0); "Works correctly with balance 0")]
#[test_case(1234, 0, op::bal(0x20, RegId::HP, REG_DATA_PTR) => RunResult::Success(1234); "Works correctly with balance 1234")]
#[test_case(Word::MAX, 0, op::bal(0x20, RegId::HP, REG_DATA_PTR) => RunResult::Success(Word::MAX); "Works correctly with balance Word::MAX")]
#[test_case(0, Word::MAX - 31, op::bal(0x20, REG_HELPER, REG_DATA_PTR) => RunResult::Panic(PanicReason::MemoryOverflow); "$rB + 32 overflows")]
#[test_case(0, VM_MAX_RAM - 31, op::bal(0x20, REG_HELPER, REG_DATA_PTR) => RunResult::Panic(PanicReason::MemoryOverflow); "$rB + 32 > VM_MAX_RAM")]
#[test_case(0, Word::MAX - 31, op::bal(0x20, RegId::HP, REG_HELPER) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 overflows")]
#[test_case(0, VM_MAX_RAM - 31, op::bal(0x20, RegId::HP, REG_HELPER) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 > VM_MAX_RAM")]
#[test_case(0, 0, op::bal(0x20, RegId::HP, RegId::ZERO) => RunResult::Panic(PanicReason::ContractNotInInputs); "Contract not in inputs")]
fn bal_external(amount: Word, helper: Word, bal_instr: Instruction) -> RunResult<Word> {
    let reg_len: u8 = 0x10;

    let mut ops = set_full_word(REG_HELPER.into(), helper);
    ops.extend(set_full_word(
        reg_len.into(),
        (ContractId::LEN + AssetId::LEN) as Word,
    ));
    ops.extend([
        // Compute asset id from contract id and sub asset id
        op::aloc(reg_len),
        op::gtf_args(REG_DATA_PTR, RegId::ZERO, GTFArgs::ScriptData),
        bal_instr,
        op::log(0x20, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let contract_id = test_context
        .setup_contract(
            vec![op::ret(RegId::ONE)],
            Some((AssetId::zeroed(), amount)),
            None,
        )
        .contract_id;

    let result = test_context
        .start_script(ops, contract_id.to_bytes())
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    RunResult::extract(result.receipts(), first_log)
}

#[test_case(0, 0, op::bal(0x20, RegId::HP, REG_DATA_PTR) => RunResult::Success(0); "Works correctly with balance 0")]
#[test_case(1234, 0, op::bal(0x20, RegId::HP, REG_DATA_PTR) => RunResult::Success(1234); "Works correctly with balance 1234")]
#[test_case(Word::MAX, 0, op::bal(0x20, RegId::HP, REG_DATA_PTR) => RunResult::Success(Word::MAX); "Works correctly with balance Word::MAX")]
#[test_case(0, Word::MAX - 31, op::bal(0x20, REG_HELPER, REG_DATA_PTR) => RunResult::Panic(PanicReason::MemoryOverflow); "$rB + 32 overflows")]
#[test_case(0, VM_MAX_RAM - 31, op::bal(0x20, REG_HELPER, REG_DATA_PTR) => RunResult::Panic(PanicReason::MemoryOverflow); "$rB + 32 > VM_MAX_RAM")]
#[test_case(0, Word::MAX - 31, op::bal(0x20, RegId::HP, REG_HELPER) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 overflows")]
#[test_case(0, VM_MAX_RAM - 31, op::bal(0x20, RegId::HP, REG_HELPER) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 > VM_MAX_RAM")]
#[test_case(0, 0, op::bal(0x20, RegId::HP, RegId::ZERO) => RunResult::Panic(PanicReason::ContractNotInInputs); "Contract not in inputs")]
fn mint_and_bal(
    mint_amount: Word,
    helper: Word,
    bal_instr: Instruction,
) -> RunResult<Word> {
    let reg_len: u8 = 0x10;
    let reg_mint_amount: u8 = 0x11;

    let mut ops = set_full_word(reg_mint_amount.into(), mint_amount);
    ops.extend(set_full_word(REG_HELPER.into(), helper));
    ops.extend(set_full_word(
        reg_len.into(),
        (ContractId::LEN + AssetId::LEN) as Word,
    ));
    ops.extend([
        // Compute asset id from contract id and sub asset id
        op::aloc(reg_len),
        op::mint(reg_mint_amount, RegId::HP), // Mint using the zero subid.
        op::gtf_args(REG_DATA_PTR, RegId::ZERO, GTFArgs::ScriptData),
        op::mcpi(RegId::HP, REG_DATA_PTR, ContractId::LEN.try_into().unwrap()),
        op::s256(RegId::HP, RegId::HP, reg_len),
        bal_instr,
        op::log(0x20, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let contract_id = test_context.setup_contract(ops, None, None).contract_id;
    RunResult::extract(&run(test_context, contract_id), first_log)
}

#[rstest]
#[case(0, RegId::HP, RunResult::Success(()))]
#[case(Word::MAX, RegId::HP, RunResult::Success(()))]
#[case(Word::MAX - 31, REG_HELPER, RunResult::Panic(PanicReason::MemoryOverflow))]
#[case(VM_MAX_RAM - 31, REG_HELPER, RunResult::Panic(PanicReason::MemoryOverflow))]
fn mint_burn_bounds<R: Into<u8>>(
    #[values(op::mint, op::burn)] instr: fn(RegId, R) -> Instruction,
    #[case] helper: Word,
    #[case] sub_id_ptr_reg: R,
    #[case] result: RunResult<()>,
) {
    let reg_len: u8 = 0x10;

    let mut ops = set_full_word(REG_HELPER.into(), helper);
    ops.extend(set_full_word(
        reg_len.into(),
        (ContractId::LEN + AssetId::LEN) as Word,
    ));
    ops.extend([
        // Compute asset id from contract id and sub asset id
        op::gtf_args(REG_DATA_PTR, RegId::ZERO, GTFArgs::ScriptData),
        op::aloc(reg_len),
        instr(RegId::ZERO, sub_id_ptr_reg),
        op::log(RegId::ZERO, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let contract_id = test_context
        .setup_contract(ops, Some((AssetId::zeroed(), Word::MAX - helper)), None) // Ensure enough but not too much balance
        .contract_id;
    assert_eq!(
        RunResult::extract_novalue(&run(test_context, contract_id)),
        result
    );
}

type MintOrBurnOpcode = fn(RegId, RegId) -> Instruction;

#[test_case(vec![(op::mint, 0, 0)] => RunResult::Success(()); "Mint 0")]
#[test_case(vec![(op::burn, 0, 0)] => RunResult::Success(()); "Burn 0")]
#[test_case(vec![(op::mint, 100, 0)] => RunResult::Success(()); "Mint 100")]
#[test_case(vec![(op::mint, Word::MAX, 0)] => RunResult::Success(()); "Mint Word::MAX")]
#[test_case(vec![(op::mint, 100, 0), (op::burn, 100, 0)] => RunResult::Success(()); "Mint 100, Burn all")]
#[test_case(vec![(op::mint, Word::MAX, 0), (op::burn, Word::MAX, 0)] => RunResult::Success(()); "Mint Word::MAX, Burn all")]
#[test_case(vec![(op::mint, 100, 0), (op::mint, 10, 0), (op::burn, 20, 0)] => RunResult::Success(()); "Mint 10 and 10, Burn all")]
#[test_case(vec![(op::mint, 2, 0), (op::mint, 3, 1), (op::burn, 2, 0), (op::burn, 3, 1)] => RunResult::Success(()); "Mint multiple assets, Burn all")]
#[test_case(vec![(op::burn, 1, 0)] => RunResult::Panic(PanicReason::NotEnoughBalance); "Burn nonexisting 1")]
#[test_case(vec![(op::burn, Word::MAX, 0)] => RunResult::Panic(PanicReason::NotEnoughBalance); "Burn nonexisting Word::MAX")]
#[test_case(vec![(op::mint, Word::MAX, 0), (op::mint, 1, 0)] => RunResult::Panic(PanicReason::BalanceOverflow); "Mint overflow")]
#[test_case(vec![(op::mint, Word::MAX, 0), (op::burn, 1, 0), (op::mint, 2, 0)] => RunResult::Panic(PanicReason::BalanceOverflow); "Mint,Burn,Mint overflow")]
fn mint_burn_single_sequence(seq: Vec<(MintOrBurnOpcode, Word, u8)>) -> RunResult<()> {
    let reg_len: u8 = 0x10;
    let reg_mint_amount: u8 = 0x11;

    let mut ops = vec![
        // Allocate space for sub asset id
        op::movi(reg_len, 32),
        op::aloc(reg_len),
    ];

    for (mint_or_burn, amount, sub_id) in seq {
        ops.push(op::sb(RegId::HP, sub_id, 0));
        ops.extend(set_full_word(reg_mint_amount.into(), amount));
        ops.push(mint_or_burn(reg_mint_amount.into(), RegId::HP));
    }
    ops.push(op::ret(RegId::ONE));

    let mut test_context = TestBuilder::new(1234u64);
    let contract_id = test_context.setup_contract(ops, None, None).contract_id;
    RunResult::extract_novalue(&run(test_context, contract_id))
}

enum MintOrBurn {
    Mint,
    Burn,
}

#[test_case(vec![MintOrBurn::Burn] => RunResult::Panic(PanicReason::NotEnoughBalance); "Burn")]
#[test_case(vec![MintOrBurn::Mint, MintOrBurn::Burn, MintOrBurn::Burn] => RunResult::Panic(PanicReason::NotEnoughBalance); "Mint,Burn,Burn")]
#[test_case(vec![MintOrBurn::Mint, MintOrBurn::Mint, MintOrBurn::Burn] => RunResult::Success(1); "Mint,Mint,Burn")]
fn mint_burn_many_calls_sequence(seq: Vec<MintOrBurn>) -> RunResult<Word> {
    let reg_len: u8 = 0x10;
    let reg_jump: u8 = 0x11;

    let ops = vec![
        // Allocate space for zero sub asset id
        op::movi(reg_len, 32),
        op::aloc(reg_len),
        op::jmpf(reg_jump, 0),
        op::mint(RegId::ONE, RegId::HP), // Jump of 0 - mint 1
        op::ret(RegId::ONE),             // Jump of 1 - do nothing
        op::burn(RegId::ONE, RegId::HP), // Jump of 2 - burn 1
        op::ret(RegId::ONE),             // Jump of 3 - do nothing
    ];

    let mut test_context = TestBuilder::new(1234u64);
    let contract_id = test_context.setup_contract(ops, None, None).contract_id;

    for instr in seq {
        let script_ops = vec![
            op::movi(
                reg_jump,
                match instr {
                    MintOrBurn::Mint => 0,
                    MintOrBurn::Burn => 2,
                },
            ),
            op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ];
        let script_data: Vec<u8> = [Call::new(contract_id, 0, 0).to_bytes().as_slice()]
            .into_iter()
            .flatten()
            .copied()
            .collect();

        let receipts = test_context
            .start_script(script_ops, script_data)
            .script_gas_limit(1_000_000)
            .contract_input(contract_id)
            .fee_input()
            .contract_output(&contract_id)
            .execute()
            .receipts()
            .to_vec();

        let result = RunResult::extract_novalue(&receipts);
        if !result.is_ok() {
            return result.map(|_| unreachable!());
        }
    }

    RunResult::Success(
        test_context.get_contract_balance(
            &contract_id,
            &contract_id.asset_id(&SubAssetId::zeroed()),
        ),
    )
}

#[test_case(0, 10, 0 => RunResult::Panic(PanicReason::TransferZeroCoins); "Cannot transfer 0 coins")]
#[test_case(1, 10, 0 => RunResult::Success((9, 1)); "Can transfer 1 coins to empty")]
#[test_case(1, 10, 5 => RunResult::Success((9, 6)); "Can transfer 1 coins to non-empty")]
#[test_case(11, 10, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "Cannot transfer just over balance coins")]
#[test_case(Word::MAX, 0, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "Cannot transfer max over balance coins")]
#[test_case(1, 1, Word::MAX => RunResult::Panic(PanicReason::BalanceOverflow); "Cannot overflow balance of contract")]
#[test_case(Word::MAX, Word::MAX, 0 => RunResult::Success((0, Word::MAX)); "Can transfer Word::MAX coins to empty contract")]
fn transfer_to_contract_external(
    amount: Word,
    balance: Word,
    other_balance: Word,
) -> RunResult<(Word, Word)> {
    let contract_id_ptr = 0x11;
    let asset_id_ptr = 0x12;
    let reg_amount = 0x13;

    let mut ops = set_full_word(reg_amount.into(), amount);

    ops.extend(&[
        op::gtf_args(contract_id_ptr, RegId::ZERO, GTFArgs::ScriptData),
        op::addi(
            asset_id_ptr,
            contract_id_ptr,
            ContractId::LEN.try_into().unwrap(),
        ),
        op::tr(contract_id_ptr, reg_amount, asset_id_ptr),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let asset_id: AssetId = test_context.rng.gen();

    let contract = test_context
        .setup_contract(
            vec![op::ret(RegId::ONE)],
            Some((asset_id, other_balance)),
            None,
        )
        .contract_id;

    let script_data: Vec<u8> = contract
        .to_bytes()
        .into_iter()
        .chain(asset_id.to_bytes())
        .collect();

    let (_, tx, receipts) = test_context
        .start_script(ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(contract)
        .coin_input(asset_id, balance)
        .fee_input()
        .contract_output(&contract)
        .change_output(asset_id)
        .execute()
        .into_inner();

    let change = find_change(tx.outputs().to_vec(), asset_id);
    let result = RunResult::extract_novalue(&receipts);
    if !result.is_ok() {
        assert_eq!(change, balance, "Revert should not change balance")
    }
    result.map(|()| {
        (
            change,
            test_context.get_contract_balance(&contract, &asset_id),
        )
    })
}

enum TrTo {
    /// Transfer to self
    This,
    /// Transfer to other contract
    Other,
    /// Transfer to non-existing contract
    NonExisting,
}

#[test_case(TrTo::Other, 0, 10, 0 => RunResult::Panic(PanicReason::TransferZeroCoins); "Cannot transfer 0 coins to empty other")]
#[test_case(TrTo::Other, 0, 10, 5 => RunResult::Panic(PanicReason::TransferZeroCoins); "Cannot transfer 0 coins to non-empty other")]
#[test_case(TrTo::This, 0, 10, 0 => RunResult::Panic(PanicReason::TransferZeroCoins); "Cannot transfer 0 coins to self")]
#[test_case(TrTo::Other, 1, 10, 0 => RunResult::Success((9, 1)); "Can transfer 1 coins to other")]
#[test_case(TrTo::This, 1, 10, 0 => RunResult::Success((10, 0)); "Can transfer 1 coins to self")]
#[test_case(TrTo::Other, 11, 10, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "Cannot transfer just over balance coins to other")]
#[test_case(TrTo::This, 11, 10, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "Cannot transfer just over balance coins to self")]
#[test_case(TrTo::Other, Word::MAX, 0, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "Cannot transfer max over balance coins to other")]
#[test_case(TrTo::This, Word::MAX, 0, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "Cannot transfer max over balance coins to self")]
#[test_case(TrTo::Other, 1, 1, Word::MAX => RunResult::Panic(PanicReason::BalanceOverflow); "Cannot overflow balance of other contract")]
#[test_case(TrTo::This, Word::MAX, Word::MAX, 0 => RunResult::Success((Word::MAX, 0)); "Can transfer Word::MAX coins to self")]
#[test_case(TrTo::Other, Word::MAX, Word::MAX, 0 => RunResult::Success((0, Word::MAX)); "Can transfer Word::MAX coins to empty other")]
#[test_case(TrTo::NonExisting, 1, 1, 0 => RunResult::Panic(PanicReason::ContractNotInInputs); "Transfer target not in inputs")]
fn transfer_to_contract_internal(
    to: TrTo,
    amount: Word,
    balance: Word,
    other_balance: Word,
) -> RunResult<(Word, Word)> {
    let reg_tmp = 0x10;
    let contract_id_ptr = 0x11;
    let asset_id_ptr = 0x12;
    let reg_amount = 0x13;

    let mut ops = set_full_word(reg_amount.into(), amount);

    ops.extend(&[
        op::gtf_args(reg_tmp, RegId::ZERO, GTFArgs::ScriptData),
        op::addi(contract_id_ptr, reg_tmp, Call::LEN.try_into().unwrap()),
        op::addi(
            asset_id_ptr,
            contract_id_ptr,
            ContractId::LEN.try_into().unwrap(),
        ),
        op::tr(contract_id_ptr, reg_amount, asset_id_ptr),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let asset_id: AssetId = test_context.rng.gen();

    let this_contract = test_context
        .setup_contract(ops, Some((asset_id, balance)), None)
        .contract_id;

    let other_contract = test_context
        .setup_contract(
            vec![op::ret(RegId::ONE)],
            Some((asset_id, other_balance)),
            None,
        )
        .contract_id;

    let script_ops = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ];
    let script_data: Vec<u8> = [Call::new(this_contract, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .chain(match to {
            TrTo::This => this_contract.to_bytes(),
            TrTo::Other => other_contract.to_bytes(),
            TrTo::NonExisting => vec![1u8; 32], // Non-existing contract
        })
        .chain(asset_id.to_bytes())
        .collect();

    let result = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(this_contract)
        .contract_input(other_contract)
        .fee_input()
        .contract_output(&this_contract)
        .contract_output(&other_contract)
        .execute();

    RunResult::extract_novalue(result.receipts()).map(|()| {
        (
            test_context.get_contract_balance(&this_contract, &asset_id),
            test_context.get_contract_balance(&other_contract, &asset_id),
        )
    })
}

#[test_case(None, None => RunResult::Success(()); "Normal case works")]
#[test_case(Some(Word::MAX - 31), None => RunResult::Panic(PanicReason::MemoryOverflow); "$rA + 32 overflows")]
#[test_case(Some(VM_MAX_RAM - 31), None => RunResult::Panic(PanicReason::MemoryOverflow); "$rA + 32 > VM_MAX_RAM")]
#[test_case(None, Some(Word::MAX - 31) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 overflows")]
#[test_case(None, Some(VM_MAX_RAM - 31) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 > VM_MAX_RAM")]
fn transfer_to_contract_bounds(
    overwrite_contract_id_ptr: Option<Word>,
    overwrite_asset_id_ptr: Option<Word>,
) -> RunResult<()> {
    let reg_tmp = 0x10;
    let contract_id_ptr = 0x11;
    let asset_id_ptr = 0x12;

    let mut ops = vec![
        op::gtf_args(reg_tmp, RegId::ZERO, GTFArgs::ScriptData),
        op::addi(contract_id_ptr, reg_tmp, Call::LEN.try_into().unwrap()),
        op::addi(
            asset_id_ptr,
            contract_id_ptr,
            ContractId::LEN.try_into().unwrap(),
        ),
    ];

    if let Some(value) = overwrite_contract_id_ptr {
        ops.extend(set_full_word(contract_id_ptr.into(), value));
    }

    if let Some(value) = overwrite_asset_id_ptr {
        ops.extend(set_full_word(asset_id_ptr.into(), value));
    }

    ops.extend(&[
        op::tr(contract_id_ptr, RegId::ONE, asset_id_ptr),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let asset_id: AssetId = test_context.rng.gen();

    let this_contract = test_context
        .setup_contract(ops, Some((asset_id, Word::MAX)), None)
        .contract_id;

    let script_ops = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ];
    let script_data: Vec<u8> = [Call::new(this_contract, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .chain(this_contract.to_bytes())
        .chain(asset_id.to_bytes())
        .collect();

    let result = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(this_contract)
        .fee_input()
        .contract_output(&this_contract)
        .execute();

    RunResult::extract_novalue(result.receipts())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Internal,
    External,
}

const M: Word = Word::MAX;

#[test_case(Ctx::External, 0, 0, 10 => RunResult::Panic(PanicReason::TransferZeroCoins); "(external) Cannot transfer 0 coins to non-Variable output")]
#[test_case(Ctx::External, 1, 0, 10 => RunResult::Panic(PanicReason::TransferZeroCoins); "(external) Cannot transfer 0 coins to valid output")]
#[test_case(Ctx::External, 9, 0, 10 => RunResult::Panic(PanicReason::TransferZeroCoins); "(external) Cannot transfer 0 coins to non-existing output")]
#[test_case(Ctx::External, 1, 1, 10 => RunResult::Success((1, 9)); "(external) Can transfer 1 coins")]
#[test_case(Ctx::External, 1, 11, 10 => RunResult::Panic(PanicReason::NotEnoughBalance); "(external) Cannot transfer just over balance coins")]
#[test_case(Ctx::External, 1, M, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "(external) Cannot transfer max over balance coins")]
#[test_case(Ctx::External, 1, M, M => RunResult::Success((Word::MAX, 0)); "(external) Can transfer Word::MAX coins")]
#[test_case(Ctx::External, 0, 1, 10 => RunResult::Panic(PanicReason::OutputNotFound); "(external) Target output is not Variable")]
#[test_case(Ctx::External, 9, 1, 1 => RunResult::Panic(PanicReason::OutputNotFound); "(external) Target output doesn't exist")]
#[test_case(Ctx::External, M, 1, 1 => RunResult::Panic(PanicReason::OutputNotFound); "(external) Target output is Word::MAX")]
#[test_case(Ctx::Internal, 0, 0, 10 => RunResult::Panic(PanicReason::TransferZeroCoins); "(internal) Cannot transfer 0 coins to non-Variable output")]
#[test_case(Ctx::Internal, 1, 0, 10 => RunResult::Panic(PanicReason::TransferZeroCoins); "(internal) Cannot transfer 0 coins to valid output")]
#[test_case(Ctx::Internal, 9, 0, 10 => RunResult::Panic(PanicReason::TransferZeroCoins); "(internal) Cannot transfer 0 coins to non-existing output")]
#[test_case(Ctx::Internal, 1, 1, 10 => RunResult::Success((1, 9)); "(internal) Can transfer 1 coins")]
#[test_case(Ctx::Internal, 1, 11, 10 => RunResult::Panic(PanicReason::NotEnoughBalance); "(internal) Cannot transfer just over balance coins")]
#[test_case(Ctx::Internal, 1, M, 0 => RunResult::Panic(PanicReason::NotEnoughBalance); "(internal) Cannot transfer max over balance coins")]
#[test_case(Ctx::Internal, 1, M, M => RunResult::Success((Word::MAX, 0)); "(internal) Can transfer Word::MAX coins")]
#[test_case(Ctx::Internal, 0, 1, 10 => RunResult::Panic(PanicReason::OutputNotFound); "(internal) Target output is not Variable")]
#[test_case(Ctx::Internal, 9, 1, 1 => RunResult::Panic(PanicReason::OutputNotFound); "(internal) Target output doesn't exist")]
#[test_case(Ctx::Internal, M, 1, 1 => RunResult::Panic(PanicReason::OutputNotFound); "(internal) Target output is Word::MAX")]
fn transfer_to_output(
    ctx: Ctx,
    to_index: Word, // 1 = the variable output
    amount: Word,
    balance: Word,
) -> RunResult<(Word, Word)> {
    let reg_tmp = 0x10;
    let asset_id_ptr = 0x12;
    let reg_amount = 0x13;
    let reg_index = 0x14;

    let mut ops = set_full_word(reg_amount.into(), amount);
    ops.extend(set_full_word(reg_index.into(), to_index));
    ops.extend(&[
        op::gtf_args(reg_tmp, RegId::ZERO, GTFArgs::ScriptData),
        op::addi(asset_id_ptr, reg_tmp, Call::LEN.try_into().unwrap()),
        op::tro(reg_tmp, reg_index, reg_amount, asset_id_ptr),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let asset_id: AssetId = test_context.rng.gen();

    let contract_id = test_context
        .setup_contract(ops.clone(), Some((asset_id, balance)), None)
        .contract_id;

    let script_ops = match ctx {
        Ctx::Internal => vec![
            op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        Ctx::External => ops,
    };

    let script_data: Vec<u8> = [Call::new(contract_id, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .chain(asset_id.to_bytes())
        .collect();

    let mut builder = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .variable_output(asset_id);

    if ctx == Ctx::External {
        builder = builder
            .coin_input(asset_id, balance)
            .change_output(asset_id);
    }

    let (_, tx, receipts) = builder.execute().into_inner();
    let result = RunResult::extract(&receipts, first_tro);

    if let Some(Output::Variable {
        to: var_to,
        amount: var_amount,
        asset_id: var_asset_id,
    }) = tx.outputs().get(to_index as usize).cloned()
    {
        if result.is_ok() {
            assert_eq!(var_amount, amount, "Transfer amount is wrong");
            assert_eq!(var_asset_id, asset_id, "Transfer asset id is wrong");
        } else {
            assert_eq!(
                var_to,
                Address::zeroed(),
                "Transfer target should be zeroed on failure"
            );
            assert_eq!(var_amount, 0, "Transfer amount should be 0 on failure");
            assert_eq!(
                var_asset_id,
                AssetId::zeroed(),
                "Transfer asset id should be zeroed on failure"
            );
        }
    }

    if !result.is_ok() && ctx == Ctx::External {
        assert_eq!(
            find_change(tx.outputs().to_vec(), asset_id),
            balance,
            "Revert should not change balance"
        )
    }

    result.map(|tr| {
        (
            tr,
            if ctx == Ctx::Internal {
                test_context.get_contract_balance(&contract_id, &asset_id)
            } else {
                find_change(tx.outputs().to_vec(), asset_id)
            },
        )
    })
}

#[test_case(None, None => RunResult::Success(()); "Normal case works")]
#[test_case(Some(Word::MAX - 31), None => RunResult::Panic(PanicReason::MemoryOverflow); "$rA + 32 overflows")]
#[test_case(Some(VM_MAX_RAM - 31), None => RunResult::Panic(PanicReason::MemoryOverflow); "$rA + 32 > VM_MAX_RAM")]
#[test_case(None, Some(Word::MAX - 31) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 overflows")]
#[test_case(None, Some(VM_MAX_RAM - 31) => RunResult::Panic(PanicReason::MemoryOverflow); "$rC + 32 > VM_MAX_RAM")]
fn transfer_to_output_bounds(
    overwrite_contract_id_ptr: Option<Word>,
    overwrite_asset_id_ptr: Option<Word>,
) -> RunResult<()> {
    let reg_tmp = 0x10;
    let contract_id_ptr = 0x11;
    let asset_id_ptr = 0x12;

    let mut ops = vec![
        op::gtf_args(reg_tmp, RegId::ZERO, GTFArgs::ScriptData),
        op::addi(contract_id_ptr, reg_tmp, Call::LEN.try_into().unwrap()),
        op::addi(
            asset_id_ptr,
            contract_id_ptr,
            ContractId::LEN.try_into().unwrap(),
        ),
    ];

    if let Some(value) = overwrite_contract_id_ptr {
        ops.extend(set_full_word(contract_id_ptr.into(), value));
    }

    if let Some(value) = overwrite_asset_id_ptr {
        ops.extend(set_full_word(asset_id_ptr.into(), value));
    }

    ops.extend(&[
        op::tro(contract_id_ptr, RegId::ONE, RegId::ONE, asset_id_ptr),
        op::ret(RegId::ONE),
    ]);

    let mut test_context = TestBuilder::new(1234u64);
    let asset_id: AssetId = test_context.rng.gen();

    let this_contract = test_context
        .setup_contract(ops, Some((asset_id, Word::MAX)), None)
        .contract_id;

    let script_ops = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ];
    let script_data: Vec<u8> = [Call::new(this_contract, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .chain(this_contract.to_bytes())
        .chain(asset_id.to_bytes())
        .collect();

    let result = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(this_contract)
        .fee_input()
        .contract_output(&this_contract)
        .variable_output(asset_id)
        .execute();

    RunResult::extract_novalue(result.receipts())
}

// Calls script -> src -> dst
#[test_case(0, 0, 0, 0, 0 => ((0, 0, 0), RunResult::Success(())); "No coins moving, zero balances")]
#[test_case(1, 1, 1, 0, 0 => ((1, 1, 1), RunResult::Success(())); "No coins moving, nonzero balances")]
#[test_case(1, 0, 0, 1, 0 => ((0, 1, 0), RunResult::Success(())); "Fwd 1 from script to src")]
#[test_case(0, 1, 0, 0, 1 => ((0, 0, 1), RunResult::Success(())); "Fwd 1 from src to dst")]
#[test_case(1, 0, 0, 1, 1 => ((0, 0, 1), RunResult::Success(())); "Fwd 1 from script to dst")]
#[test_case(1, 2, 3, 1, 3 => ((0, 0, 6), RunResult::Success(())); "Fwd combination full")]
#[test_case(5, 5, 1, 3, 2 => ((2, 6, 3), RunResult::Success(())); "Fwd combination partial")]
#[test_case(M, 0, 0, M, 0 => ((0, M, 0), RunResult::Success(())); "Fwd Word::MAX from script to src")]
#[test_case(0, M, 0, 0, M => ((0, 0, M), RunResult::Success(())); "Fwd Word::MAX from src to dst")]
#[test_case(M, 0, 0, M, M => ((0, 0, M), RunResult::Success(())); "Fwd Word::MAX from script to dst")]
#[test_case(1, M, 0, 1, 0 => ((1, M, 0), RunResult::Panic(PanicReason::BalanceOverflow)); "Fwd 1 overflow on src")]
#[test_case(0, 1, M, 0, 1 => ((0, 1, M), RunResult::Panic(PanicReason::BalanceOverflow)); "Fwd 1 overflow on dst")]
#[test_case(M, 1, 0, M, 0 => ((M, 1, 0), RunResult::Panic(PanicReason::BalanceOverflow)); "Fwd Word::MAX overflow on src")]
#[test_case(0, M, 1, 0, M => ((0, M, 1), RunResult::Panic(PanicReason::BalanceOverflow)); "Fwd Word::MAX overflow on dst")]
#[test_case(M, M, M, M, M => ((M, M, M), RunResult::Panic(PanicReason::BalanceOverflow)); "Fwd Word::MAX both")]
#[test_case(0, 0, 0, 1, 0 => ((0, 0, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd 1 over empty script balance")]
#[test_case(1, 0, 0, 2, 0 => ((1, 0, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd 1 over script balance")]
#[test_case(0, 0, 0, M, 0 => ((0, 0, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd max over empty script balance")]
#[test_case(1, 0, 0, M, 0 => ((1, 0, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd max over script balance")]
#[test_case(0, 0, 0, 0, 1 => ((0, 0, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd 1 over empty src balance")]
#[test_case(0, 1, 0, 0, 2 => ((0, 1, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd 1 over src balance")]
#[test_case(0, 0, 0, 0, M => ((0, 0, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd max over empty src balance")]
#[test_case(0, 1, 0, 0, M => ((0, 1, 0), RunResult::Panic(PanicReason::NotEnoughBalance)); "Fwd max over src balance")]
fn call_forwarding(
    balance_in: Word,
    balance_src: Word,
    balance_dst: Word,
    fwd_to_src: Word,
    fwd_to_dst: Word,
) -> ((Word, Word, Word), RunResult<()>) {
    let reg_tmp = 0x10;
    let reg_dst_contract_ptr = 0x11;
    let reg_fwd_to_src: u8 = 0x12;
    let reg_fwd_to_dst: u8 = 0x13;
    let reg_asset_id_ptr: u8 = 0x14;

    let mut test_context = TestBuilder::new(1234u64);
    let asset_id: AssetId = test_context.rng.gen();

    // Setup the dst contract. This does nothing, just holds/receives the balance.
    let dst_contract = test_context
        .setup_contract(
            vec![op::ret(RegId::ONE)],
            Some((asset_id, balance_dst)),
            None,
        )
        .contract_id;

    // Setup the src contract. This just calls the dst to forward the coins.
    let src_contract = test_context
        .setup_contract(
            vec![
                op::call(reg_dst_contract_ptr, reg_fwd_to_dst, RegId::HP, RegId::CGAS),
                op::ret(RegId::ONE),
            ],
            Some((asset_id, balance_src)),
            None,
        )
        .contract_id;

    // Setup the script that does the call.
    let mut script_ops = Vec::new();
    script_ops.extend(set_full_word(reg_fwd_to_src.into(), fwd_to_src));
    script_ops.extend(set_full_word(reg_fwd_to_dst.into(), fwd_to_dst));
    script_ops.extend(&[
        op::movi(reg_tmp, AssetId::LEN.try_into().unwrap()),
        op::aloc(reg_tmp),
        op::gtf_args(reg_tmp, RegId::ZERO, GTFArgs::ScriptData),
        op::addi(
            reg_asset_id_ptr,
            reg_tmp,
            (Call::LEN * 2).try_into().unwrap(),
        ),
        op::mcpi(
            RegId::HP,
            reg_asset_id_ptr,
            AssetId::LEN.try_into().unwrap(),
        ),
        op::addi(reg_dst_contract_ptr, reg_tmp, Call::LEN.try_into().unwrap()),
        op::call(reg_tmp, reg_fwd_to_src, RegId::HP, RegId::CGAS),
        op::ret(RegId::ONE),
    ]);

    let script_data: Vec<u8> = [
        Call::new(src_contract, 0, 0).to_bytes().as_slice(),
        Call::new(dst_contract, 0, 0).to_bytes().as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .chain(asset_id.to_bytes())
    .collect();

    let (_, tx, receipts) = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(src_contract)
        .contract_input(dst_contract)
        .coin_input(asset_id, balance_in)
        .fee_input()
        .contract_output(&src_contract)
        .contract_output(&dst_contract)
        .change_output(asset_id)
        .execute()
        .into_inner();

    let result = RunResult::extract_novalue(&receipts);
    let change = find_change(tx.outputs().to_vec(), asset_id);
    (
        (
            change,
            test_context.get_contract_balance(&src_contract, &asset_id),
            test_context.get_contract_balance(&dst_contract, &asset_id),
        ),
        result,
    )
}
