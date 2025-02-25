#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::{
    vec,
    vec::Vec,
};

use crate::{
    storage::{
        MemoryStorage,
        MemoryStorageError,
    },
    verification::Normal,
};

use super::*;
use crate::crypto;
use fuel_storage::StorageAsMut;
use fuel_tx::{
    field::ReceiptsRoot,
    Script,
};
use fuel_types::{
    canonical::Serialize,
    ContractId,
};
use test_case::test_case;

struct Input {
    params: PrepareCallParams,
    reg: RegInput,
    context: Context,
    balance: Vec<(AssetId, Word)>,
    input_contracts: Vec<ContractId>,
    storage_balance: Vec<(AssetId, Word)>,
    memory: MemoryInstance,
    gas_cost: DependentCost,
    storage_contract: Vec<(ContractId, Vec<u8>)>,
    script: Option<Script>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            params: Default::default(),
            reg: RegInput {
                cgas: 20,
                ggas: 40,
                ..Default::default()
            },
            context: Context::Script {
                block_height: Default::default(),
            },
            balance: Default::default(),
            input_contracts: vec![Default::default()],
            storage_balance: Default::default(),
            memory: vec![0u8; MEM_SIZE].try_into().unwrap(),
            gas_cost: DependentCost::from_units_per_gas(10, 10),
            storage_contract: vec![(ContractId::default(), vec![0u8; 10])],
            script: None,
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug, Clone)]
struct RegInput {
    hp: u64,
    sp: u64,
    ssp: u64,
    fp: u64,
    pc: u64,
    is: u64,
    bal: u64,
    cgas: u64,
    ggas: u64,
}

#[derive(PartialEq, Eq)]
enum CheckMem {
    Check(Vec<(usize, Vec<u8>)>),
    Mem(MemoryInstance),
}

#[derive(PartialEq, Eq)]
struct Output {
    reg: RegInput,
    memory: CheckMem,
    frames: Vec<CallFrame>,
    receipts: ReceiptsCtx,
    context: Context,
    script: Option<Script>,
}

impl core::fmt::Debug for Output {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Output")
            .field("reg", &self.reg)
            .field("memory", &"[..]")
            .field("frames", &self.frames)
            .field("receipts", &self.receipts)
            .field("context", &self.context)
            .finish()
    }
}

fn make_reg(changes: &[(u8, u64)]) -> [Word; VM_REGISTER_COUNT] {
    let mut registers = [0u64; VM_REGISTER_COUNT];
    for (reg, val) in changes {
        registers[*reg as usize] = *val;
    }
    registers
}

impl Default for Output {
    fn default() -> Self {
        Self {
            reg: Default::default(),
            memory: CheckMem::Check(vec![]),
            frames: vec![CallFrame::new(
                Default::default(),
                Default::default(),
                make_reg(&[(HP, 1000), (SP, 100), (SSP, 100), (CGAS, 20), (GGAS, 20)]),
                16,
                0,
                0,
            )
            .unwrap()],
            receipts: vec![Receipt::call(
                Default::default(),
                Default::default(),
                0,
                Default::default(),
                0,
                0,
                0,
                700,
                700,
            )]
            .into(),
            context: Context::Call {
                block_height: Default::default(),
            },
            script: None,
        }
    }
}

fn mem(set: &[(usize, Vec<u8>)]) -> MemoryInstance {
    let mut memory: MemoryInstance = vec![0u8; MEM_SIZE].try_into().unwrap();
    for (addr, data) in set {
        memory[*addr..*addr + data.len()].copy_from_slice(data);
    }
    memory
}

#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 0,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 0,
        },
        reg: RegInput{hp: 1000, sp: 100, ssp: 100, fp: 0, pc: 0, is: 0, bal: 0, cgas: 21, ggas: 21 },
        context: Context::Script{ block_height: Default::default() },
        ..Default::default()
    } => using check_output(Ok(Output{
        reg: RegInput{hp: 1000, sp: 716, ssp: 716, fp: 100, pc: 700, is: 700, bal: 0, cgas: 0, ggas: 20 },
        ..Default::default()
    })); "basic call working"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 2032,
            amount_of_coins_to_forward: 20,
            asset_id_pointer: 2000,
            amount_of_gas_to_forward: 30,
        },
        reg: RegInput{hp: 1000, sp: 200, ssp: 200, fp: 0, pc: 0, is: 0, bal: 0, cgas: 201, ggas: 201 },
        context: Context::Script{ block_height: Default::default() },
        input_contracts: vec![ContractId::from([1u8; 32])],
        memory: mem(&[(2000, vec![2; 32]), (2032, Call::new(ContractId::from([1u8; 32]), 4, 5).into())]),
        storage_contract: vec![(ContractId::from([1u8; 32]), vec![0u8; 100])],
        balance: [(AssetId::from([2u8; 32]), 30)].into_iter().collect(),
        script: Some(Default::default()),
        ..Default::default()
    } => using check_output({
        let frame = CallFrame::new(ContractId::from([1u8; 32]), AssetId::from([2u8; 32]), make_reg(&[(HP, 1000), (SP, 200), (SSP, 200), (CGAS, 161), (GGAS, 191)]), 104, 4, 5).unwrap();
        let receipt = Receipt::call(ContractId::zeroed(), ContractId::from([1u8; 32]), 20, AssetId::from([2u8; 32]), 30, 4, 5, 800, 800);
        let mut script = Script::default();
        *script.receipts_root_mut() = crypto::ephemeral_merkle_root([receipt.to_bytes()].into_iter());
        Ok(Output{
            reg: RegInput{hp: 1000, sp: 904, ssp: 904, fp: 200, pc: 800, is: 800, bal: 20, cgas: 30, ggas: 191 },
            receipts: vec![receipt].into(),
            frames: vec![frame.clone()],
            memory: CheckMem::Check(vec![(200, frame.into()), (2000, vec![2; 32]), (2032, Call::new(ContractId::from([1u8; 32]), 4, 5).into())]),
            script: Some(script),
            ..Default::default()
        })
    }); "call working with real memory settings"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 20,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 0,
        },
        reg: RegInput{hp: 1000, sp: 100, ssp: 100, fp: 0, pc: 0, is: 0, bal: 0, cgas: 11, ggas: 11 },
        context: Context::Script{ block_height: Default::default() },
        balance: [(AssetId::default(), 30)].into_iter().collect(),
        ..Default::default()
    } => using check_output(Ok(Output{
        reg: RegInput{hp: 1000, sp: 716, ssp: 716, fp: 100, pc: 700, is: 700, bal: 20, cgas: 0, ggas: 10 },
        receipts: vec![Receipt::call(Default::default(), Default::default(), 20, Default::default(), 0, 0, 0, 700, 700)].into(),
        frames: vec![CallFrame::new(Default::default(), Default::default(), make_reg(&[(HP, 1000), (SP, 100), (SSP, 100), (CGAS, 10), (GGAS, 10)]), 16, 0, 0).unwrap()],
        ..Default::default()
    })); "transfers with enough balance external"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 20,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 10,
        },
        reg: RegInput{hp: 1000, sp: 100, ssp: 100, fp: 0, pc: 0, is: 0, bal: 0, cgas: 40, ggas: 80 },
        context: Context::Script{ block_height: Default::default() },
        balance: [(AssetId::default(), 30)].into_iter().collect(),
        ..Default::default()
    } => using check_output(Ok(Output{
        reg: RegInput{hp: 1000, sp: 716, ssp: 716, fp: 100, pc: 700, is: 700, bal: 20, cgas: 10, ggas: 79 },
        receipts: vec![Receipt::call(Default::default(), Default::default(), 20, Default::default(), 10, 0, 0, 700, 700)].into(),
        frames: vec![CallFrame::new(Default::default(), Default::default(), make_reg(&[(HP, 1000), (SP, 100), (SSP, 100), (CGAS, 29), (GGAS, 79)]), 16, 0, 0).unwrap()],
        ..Default::default()
    })); "forwards gas"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 20,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 100,
        },
        reg: RegInput{hp: 1000, sp: 100, ssp: 100, fp: 0, pc: 0, is: 0, bal: 0, cgas: 40, ggas: 80 },
        context: Context::Script{ block_height: Default::default() },
        balance: [(AssetId::default(), 30)].into_iter().collect(),
        ..Default::default()
    } => using check_output(Ok(Output{
        reg: RegInput{hp: 1000, sp: 716, ssp: 716, fp: 100, pc: 700, is: 700, bal: 20, cgas: 39, ggas: 79 },
        receipts: vec![Receipt::call(Default::default(), Default::default(), 20, Default::default(), 39, 0, 0, 700, 700)].into(),
        frames: vec![CallFrame::new(Default::default(), Default::default(), make_reg(&[(HP, 1000), (SP, 100), (SSP, 100), (CGAS, 0), (GGAS, 79)]), 16, 0, 0).unwrap()],
        ..Default::default()
    })); "the receipt shows forwarded gas correctly when limited by available gas"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 20,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 0,
        },
        reg: RegInput{hp: 1000, sp: 100, ssp: 100, fp: 0, pc: 0, is: 0, bal: 0, cgas: 11, ggas: 11 },
        context: Context::Call{ block_height: Default::default() },
        storage_balance: [(AssetId::default(), 30)].into_iter().collect(),
        ..Default::default()
    } => using check_output(Ok(Output{
        reg: RegInput{hp: 1000, sp: 716, ssp: 716, fp: 100, pc: 700, is: 700, bal: 20, cgas: 0, ggas: 10 },
        receipts: vec![Receipt::call(Default::default(), Default::default(), 20, Default::default(), 0, 0, 0, 700, 700)].into(),
        frames: vec![CallFrame::new(Default::default(), Default::default(), make_reg(&[(HP, 1000), (SP, 100), (SSP, 100), (CGAS, 10), (GGAS, 10)]), 16, 0, 0).unwrap()],
        ..Default::default()
    })); "transfers with enough balance internal"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 20,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 0,
        },
        reg: RegInput{hp: 1000, sp: 0, ssp: 0, fp: 0, pc: 0, is: 0, bal: 0, cgas: 11, ggas: 11 },
        context: Context::Script{ block_height: Default::default() },
        ..Default::default()
    } => using check_output(Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance))); "Tries to forward more coins than the contract has"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: VM_MAX_RAM - 40,
            amount_of_coins_to_forward: 0,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 0,
        },
        reg: RegInput{hp: 1000, sp: 0, ssp: 0, fp: 0, pc: 0, is: 0, bal: 0, cgas: 11, ggas: 11 },
        context: Context::Script{ block_height: Default::default() },
        ..Default::default()
    } => using check_output(Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))); "call_params_pointer overflow"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 0,
            asset_id_pointer: VM_MAX_RAM - 31,
            amount_of_gas_to_forward: 0,
        },
        reg: RegInput{hp: 1000, sp: 0, ssp: 0, fp: 0, pc: 0, is: 0, bal: 0, cgas: 11, ggas: 11 },
        context: Context::Script{ block_height: Default::default() },
        ..Default::default()
    } => using check_output(Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))); "asset_id_pointer overflow"
)]
#[test_case(
    Input{
        params: PrepareCallParams {
            call_params_pointer: 0,
            amount_of_coins_to_forward: 10,
            asset_id_pointer: 0,
            amount_of_gas_to_forward: 0,
        },
        reg: RegInput{hp: 1000, sp: 0, ssp: 0, fp: 0, pc: 0, is: 0, bal: 0, cgas: 11, ggas: 11 },
        context: Context::Call{ block_height: Default::default() },
        balance: [(AssetId::default(), 30)].into_iter().collect(),
        ..Default::default()
    } => using check_output(Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance))); "Transfer too many coins internally"
)]
fn test_prepare_call(input: Input) -> Result<Output, RuntimeError<MemoryStorageError>> {
    let Input {
        params,
        mut reg,
        mut context,
        balance,
        storage_balance,
        input_contracts,
        mut memory,
        gas_cost,
        storage_contract,
        script,
    } = input;
    let mut registers = [0; VM_REGISTER_COUNT];
    let mut registers: PrepareCallRegisters = (&mut registers).into();
    registers.system_registers.hp = Reg::new(&reg.hp);
    registers.system_registers.sp = RegMut::new(&mut reg.sp);
    registers.system_registers.ssp = RegMut::new(&mut reg.ssp);
    registers.system_registers.fp = RegMut::new(&mut reg.fp);
    registers.system_registers.pc = RegMut::new(&mut reg.pc);
    registers.system_registers.is = RegMut::new(&mut reg.is);
    registers.system_registers.bal = RegMut::new(&mut reg.bal);
    registers.system_registers.cgas = RegMut::new(&mut reg.cgas);
    registers.system_registers.ggas = RegMut::new(&mut reg.ggas);
    let mut runtime_balances =
        RuntimeBalances::try_from_iter(balance).expect("Balance should be valid");
    let mut storage = MemoryStorage::default();
    for (id, code) in storage_contract {
        StorageAsMut::storage::<ContractsRawCode>(&mut storage)
            .write_bytes(&id, code.as_ref())
            .unwrap();
    }
    for (a, n) in storage_balance.iter() {
        let old_balance = storage
            .contract_asset_id_balance_replace(&ContractId::default(), a, *n)
            .unwrap();
        assert!(old_balance.is_none());
    }
    let mut panic_context = PanicContext::None;
    let mut receipts = Default::default();
    let mut frames = Vec::default();
    let current_contract = context.is_internal().then_some(ContractId::default());

    let input_contracts = input_contracts.into_iter().collect();
    let input = PrepareCallCtx {
        params,
        registers,
        memory: &mut memory,
        context: &mut context,
        gas_cost,
        runtime_balances: &mut runtime_balances,
        storage: &mut storage,
        input_contracts: &input_contracts,
        panic_context: &mut panic_context,
        new_storage_gas_per_byte: 0,
        receipts: &mut receipts,
        frames: &mut frames,
        current_contract,
        verifier: &mut Normal,
    };
    input.prepare_call().map(|_| Output {
        reg,
        frames,
        memory: CheckMem::Mem(memory),
        receipts,
        context,
        script,
    })
}

fn check_output(
    expected: Result<Output, RuntimeError<MemoryStorageError>>,
) -> impl FnOnce(Result<Output, RuntimeError<MemoryStorageError>>) {
    move |result| match (expected, result) {
        (Ok(e), Ok(r)) => {
            assert_eq!(e.reg, r.reg);
            assert_eq!(e.receipts, r.receipts);
            assert_eq!(e.frames, r.frames);
            assert_eq!(e.context, r.context);
            match (e.memory, r.memory) {
                (CheckMem::Check(e), CheckMem::Mem(r)) => {
                    for (i, bytes) in e {
                        assert_eq!(
                            r[i..i + bytes.len()],
                            bytes,
                            "memory mismatch at {i}"
                        );
                    }
                }
                _ => unreachable!(),
            }
        }
        t => assert_eq!(t.0, t.1),
    }
}
