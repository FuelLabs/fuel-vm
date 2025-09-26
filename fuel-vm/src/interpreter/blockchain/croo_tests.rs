use super::*;
use crate::{
    interpreter::PanicContext,
    storage::MemoryStorage,
    verification::Normal,
};
use fuel_tx::{
    Contract,
    GasCosts,
};

use alloc::vec;

const CONTRACT_LEN: usize = 512;
const INITIAL_GAS: Word = 1_000_000;

// Subset of SystemRegisters used during CROO tests
struct SystemRegisters {
    pc: Word,
    cgas: Word,
    ggas: Word,
}

fn initialize_system_registers() -> SystemRegisters {
    SystemRegisters {
        pc: 4,
        cgas: INITIAL_GAS,
        ggas: INITIAL_GAS,
    }
}

fn initialize_ownership_registers() -> OwnershipRegisters {
    OwnershipRegisters {
        sp: 1000,
        ssp: 1,
        hp: 2000,
        prev_hp: 3000,
    }
}

fn new_contract_id() -> ContractId {
    ContractId::new([3u8; ContractId::LEN])
}

#[test]
fn test_code_root() {
    // Given
    let contract_id = new_contract_id();

    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[0..ContractId::LEN].copy_from_slice(contract_id.as_slice());

    let data = alloc::vec![0xffu8; CONTRACT_LEN];
    let contract: Contract = data.into();
    let root = contract.root();
    storage
        .storage_contract_insert(&contract_id, contract.as_ref())
        .expect("Failed to insert contract");

    let gas_cost = GasCosts::default().croo();
    let ownership_registers = initialize_ownership_registers();
    let SystemRegisters {
        mut pc,
        mut cgas,
        mut ggas,
    } = initialize_system_registers();
    let croo_address = 0xFFusize;
    let croo_range = croo_address..croo_address + 32;

    let input_contracts = [contract_id];
    let mut panic_context = PanicContext::None;

    // When
    CodeRootCtx {
        memory: &mut memory,
        storage: &storage,
        gas_cost,
        input_contracts: &input_contracts.into_iter().collect(),
        panic_context: &mut panic_context,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        owner: ownership_registers,
        pc: RegMut::new(&mut pc),
        verifier: &mut Normal,
    }
    .code_root(croo_address as Word, 0)
    .unwrap();

    // Then
    assert_eq!(pc, 8);
    assert_eq!(memory[croo_range], *root.as_slice());
    assert_eq!(
        cgas,
        INITIAL_GAS - gas_cost.resolve_without_base(CONTRACT_LEN as Word)
    );
    assert_eq!(
        ggas,
        INITIAL_GAS - gas_cost.resolve_without_base(CONTRACT_LEN as Word)
    );
}

#[test]
fn test_code_root_contract_not_found() {
    // Given
    let contract_id = new_contract_id();

    let storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[0..ContractId::LEN].copy_from_slice(contract_id.as_slice());

    let gas_cost = GasCosts::default().croo();
    let ownership_registers = initialize_ownership_registers();
    let SystemRegisters {
        mut pc,
        mut cgas,
        mut ggas,
    } = initialize_system_registers();
    let croo_address = 0xFFusize;
    let croo_range = croo_address..croo_address + 32;

    let input_contracts = [contract_id];
    let mut panic_context = PanicContext::None;

    // When
    let _ = CodeRootCtx {
        memory: &mut memory,
        storage: &storage,
        gas_cost,
        input_contracts: &input_contracts.into_iter().collect(),
        panic_context: &mut panic_context,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        owner: ownership_registers,
        pc: RegMut::new(&mut pc),
        verifier: &mut Normal,
    }
    .code_root(croo_address as Word, 0)
    .expect_err("Contract is not found");

    // Then
    assert_eq!(pc, 4);
    assert_eq!(memory[croo_range], [1u8; 32]);
    assert_eq!(cgas, INITIAL_GAS);
    assert_eq!(ggas, INITIAL_GAS);
}

#[test]
fn test_code_root_contract_not_in_inputs() {
    // Given
    let contract_id = new_contract_id();

    let storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[0..ContractId::LEN].copy_from_slice(contract_id.as_slice());

    let gas_cost = GasCosts::default().croo();
    let ownership_registers = initialize_ownership_registers();
    let SystemRegisters {
        mut pc,
        mut cgas,
        mut ggas,
    } = initialize_system_registers();
    let croo_address = 0xFFusize;
    let croo_range = croo_address..croo_address + 32;

    let input_contracts = [];
    let mut panic_context = PanicContext::None;

    // When
    let _ = CodeRootCtx {
        memory: &mut memory,
        storage: &storage,
        gas_cost,
        input_contracts: &input_contracts.into_iter().collect(),
        panic_context: &mut panic_context,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        owner: ownership_registers,
        pc: RegMut::new(&mut pc),
        verifier: &mut Normal,
    }
    .code_root(croo_address as Word, 0)
    .expect_err("Contract is not in inputs");

    // Then
    assert_eq!(pc, 4);
    assert_eq!(memory[croo_range], [1u8; 32]);
    assert_eq!(cgas, INITIAL_GAS);
    assert_eq!(ggas, INITIAL_GAS);
}
