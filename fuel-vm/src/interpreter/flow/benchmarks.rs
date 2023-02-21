use criterion::{BenchmarkId, Criterion};
use fuel_storage::StorageAsMut;

use crate::{interpreter::InitialBalances, storage::MemoryStorage};

use super::*;

/// Benchmark `prepare_call`.
pub fn bench_prepare_call(c: &mut Criterion) {
    let params = PrepareCallParams {
        call_params_mem_address: 2032,
        amount_of_coins_to_forward: 20,
        asset_id_mem_address: 2000,
        amount_of_gas_to_forward: 30,
    };
    let contract_id = ContractId::from([1u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut registers = [0; VM_REGISTER_COUNT];
    registers[RegId::HP] = 2_000_000;
    registers[RegId::SP] = 200;
    registers[RegId::SSP] = 200;
    let call_bytes: Vec<u8> = Call::new(ContractId::from([1u8; 32]), 4, 5).into();
    let mut memory = mem(&[(2000, vec![2; 32]), (2032, call_bytes.clone())]);
    let mut context = Context::Script { block_height: 0 };
    let balance: InitialBalances = [(AssetId::from([2u8; 32]), 30)].into_iter().collect();
    let mut runtime_balances = RuntimeBalances::from(balance);
    let mut storage = MemoryStorage::new(0, Default::default());
    storage
        .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, 30)
        .unwrap();
    let mut panic_context = PanicContext::None;
    let mut receipts = Vec::default();
    let consensus = ConsensusParameters::default();
    let mut frames = Vec::default();
    let current_contract = context.is_internal().then_some(contract_id);

    for mut script in [None, Some(Script::default())] {
        for input in [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000] {
            StorageAsMut::storage::<ContractsRawCode>(&mut storage)
                .write(&contract_id, vec![0u8; input])
                .unwrap();
            c.bench_with_input(
                BenchmarkId::new(
                    if script.is_none() {
                        "prepare_call"
                    } else {
                        "prepare_call_external"
                    },
                    input,
                ),
                &input,
                |b, _| {
                    b.iter({
                        || {
                            let input = PrepareCallInput {
                                params: PrepareCallParams {
                                    call_params_mem_address: 2032,
                                    amount_of_coins_to_forward: 20,
                                    asset_id_mem_address: 2000,
                                    amount_of_gas_to_forward: 30,
                                },
                                registers: (&mut registers).into(),
                                memory: PrepareCallMemory::try_from((memory.as_mut(), &params)).unwrap(),
                                context: &mut context,
                                gas_cost: DependentCost {
                                    base: 0,
                                    dep_per_unit: 0,
                                },
                                runtime_balances: &mut runtime_balances,
                                storage: &mut storage,
                                input_contracts: vec![contract_id],
                                panic_context: &mut panic_context,
                                receipts: &mut receipts,
                                script: script.as_mut(),
                                consensus: &consensus,
                                frames: &mut frames,
                                current_contract,
                                profiler: &mut (),
                            };
                            input.prepare_call().unwrap();
                            storage
                                .merkle_contract_asset_id_balance_insert(&ContractId::zeroed(), &asset_id, 30)
                                .unwrap();
                            runtime_balances.checked_balance_add(&mut memory, &asset_id, 30);
                            registers[RegId::HP] = 2_000_000;
                            registers[RegId::SP] = 200;
                            registers[RegId::SSP] = 200;
                            registers[RegId::PC] = 0;
                            frames.clear();
                            receipts.clear();
                            memory[2000..2000 + 32].copy_from_slice(&[2; 32]);
                            memory[2032..2032 + Call::LEN].copy_from_slice(&call_bytes);
                        }
                    })
                },
            );
        }
    }
}

fn mem(set: &[(usize, Vec<u8>)]) -> Box<[u8; VM_MEMORY_SIZE]> {
    let mut memory: Box<[u8; VM_MEMORY_SIZE]> = vec![0u8; VM_MEMORY_SIZE].try_into().unwrap();
    for (addr, data) in set {
        memory[*addr..*addr + data.len()].copy_from_slice(data);
    }
    memory
}
