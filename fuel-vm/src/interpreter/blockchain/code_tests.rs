#![allow(clippy::cast_possible_truncation)]

use core::marker::PhantomData;

use alloc::vec;

use super::*;
use crate::{
    interpreter::{
        NotSupportedEcal,
        PanicContext,
    },
    storage::{
        MemoryStorage,
        MemoryStorageError,
    },
    verification::Panic,
};
use fuel_tx::{
    Contract,
    Script,
};

#[test]
fn test_load_contract_in_script() -> IoResult<(), MemoryStorageError> {
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let mut cgas = 1000;
    let mut ggas = 1000;
    let mut ssp = 1000;
    let mut sp = 1000;
    let hp = VM_MAX_RAM;
    let fp = 0;

    let contract_id = ContractId::from([4u8; 32]);

    let contract_id_mem_address: Word = 32;
    let offset = 20;
    let num_bytes = 40;
    const CONTRACT_SIZE: u64 = 400;

    memory[contract_id_mem_address as usize
        ..contract_id_mem_address as usize + ContractId::LEN]
        .copy_from_slice(contract_id.as_ref());
    storage
        .storage_contract_insert(
            &contract_id,
            &Contract::from(vec![5u8; CONTRACT_SIZE as usize]),
        )
        .unwrap();

    let mut panic_context = PanicContext::None;
    let input_contracts = [contract_id];
    let input_contracts = input_contracts.into_iter().collect();
    let input = LoadContractCodeCtx {
        contract_max_size: 100,
        storage: &storage,
        memory: &mut memory,
        context: &Context::Script {
            block_height: Default::default(),
        },
        input_contracts: &input_contracts,
        panic_context: &mut panic_context,
        gas_cost: DependentCost::from_units_per_gas(13, 1),
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        ssp: RegMut::new(&mut ssp),
        sp: RegMut::new(&mut sp),
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
        hp: Reg::new(&hp),
        verifier: &mut Panic,
        _phantom: PhantomData::<(MemoryInstance, Script, NotSupportedEcal)>,
    };
    input.load_contract_code(contract_id_mem_address, offset, num_bytes)?;
    assert_eq!(pc, 8);
    assert_eq!(cgas, 1000 - CONTRACT_SIZE /* price per byte */);
    assert_eq!(ggas, 1000 - CONTRACT_SIZE /* price per byte */);

    Ok(())
}
#[test]
fn test_load_contract_in_call() -> IoResult<(), MemoryStorageError> {
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let mut cgas = 1000;
    let mut ggas = 1000;
    let mut ssp = 1000;
    let mut sp = 1000;
    let fp = 32;
    let hp = VM_MAX_RAM;

    let contract_id = ContractId::from([4u8; 32]);

    let contract_id_mem_address: Word = 32;
    let offset = 20;
    let num_bytes = 40;
    const CONTRACT_SIZE: u64 = 400;

    memory[contract_id_mem_address as usize
        ..contract_id_mem_address as usize + ContractId::LEN]
        .copy_from_slice(contract_id.as_ref());
    storage
        .storage_contract_insert(
            &contract_id,
            &Contract::from(vec![5u8; CONTRACT_SIZE as usize]),
        )
        .unwrap();

    let mut panic_context = PanicContext::None;
    let input_contracts = [contract_id];
    let input_contracts = input_contracts.into_iter().collect();
    let input = LoadContractCodeCtx {
        contract_max_size: 100,
        storage: &storage,
        memory: &mut memory,
        context: &Context::Call {
            block_height: Default::default(),
        },
        input_contracts: &input_contracts,
        panic_context: &mut panic_context,
        gas_cost: DependentCost::from_units_per_gas(13, 1),
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        ssp: RegMut::new(&mut ssp),
        sp: RegMut::new(&mut sp),
        hp: Reg::new(&hp),
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
        verifier: &mut Panic,
        _phantom: PhantomData::<(MemoryInstance, Script, NotSupportedEcal)>,
    };
    input.load_contract_code(contract_id_mem_address, offset, num_bytes)?;
    assert_eq!(pc, 8);
    assert_eq!(cgas, 1000 - CONTRACT_SIZE /* price per byte */);
    assert_eq!(ggas, 1000 - CONTRACT_SIZE /* price per byte */);

    Ok(())
}

#[test]
fn test_code_copy() -> IoResult<(), MemoryStorageError> {
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut cgas = 1000;
    let mut ggas = 1000;
    let mut pc = 4;

    let contract_id = ContractId::from([4u8; 32]);

    let dest_mem_address = 2001;
    let contract_id_mem_address: Word = 32;
    let offset = 20;
    let num_bytes = 40;
    const CONTRACT_SIZE: u64 = 400;

    memory[contract_id_mem_address as usize
        ..contract_id_mem_address as usize + ContractId::LEN]
        .copy_from_slice(contract_id.as_ref());
    storage
        .storage_contract_insert(
            &contract_id,
            &Contract::from(vec![5u8; CONTRACT_SIZE as usize]),
        )
        .unwrap();

    let input_contracts = [contract_id];
    let input_contracts = input_contracts.into_iter().collect();
    let mut panic_context = PanicContext::None;
    let input = CodeCopyCtx {
        storage: &storage,
        memory: &mut memory,
        input_contracts: &input_contracts,
        panic_context: &mut panic_context,
        owner: OwnershipRegisters {
            sp: 1000,
            ssp: 1000,
            hp: 2000,
            prev_hp: VM_MAX_RAM - 1,
        },
        gas_cost: DependentCost::from_units_per_gas(13, 1),
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        pc: RegMut::new(&mut pc),
        verifier: &mut Panic,
        _phantom: PhantomData::<(MemoryInstance, Script, NotSupportedEcal)>,
    };
    input.code_copy(dest_mem_address, contract_id_mem_address, offset, num_bytes)?;
    assert_eq!(pc, 8);
    assert_eq!(cgas, 1000 - CONTRACT_SIZE /* price per byte */);
    assert_eq!(ggas, 1000 - CONTRACT_SIZE /* price per byte */);

    Ok(())
}
