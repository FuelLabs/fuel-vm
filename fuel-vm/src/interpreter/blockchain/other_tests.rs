use crate::storage::MemoryStorage;

use super::*;
use fuel_storage::StorageAsMut;
use fuel_types::Salt;
use test_case::test_case;

#[test_case(false, 0, None, 0 => Ok(()); "Burn nothing")]
#[test_case(false, 0, 0, 0 => Ok(()); "Burn is idempotent")]
#[test_case(false, 0, 100, 100 => Ok(()); "Burn all")]
#[test_case(false, 0, 100, 10 => Ok(()); "Burn some")]
#[test_case(true, 0, 100, 10 => Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)); "Can't burn from external context")]
#[test_case(false, 1, 100, 10 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Can't burn when contract id not in memory")]
#[test_case(false, 0, 100, 101 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Can't burn too much")]
#[test_case(false, 0, None, 1 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Can't burn when no balance")]
fn test_burn(external: bool, fp: Word, initialize: impl Into<Option<Word>>, amount: Word) -> Result<(), RuntimeError> {
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory = VmMemory::fully_allocated();
    let contract_id = ContractId::from([3u8; ContractId::LEN]);
    memory.force_write_bytes(0, &contract_id);
    let asset_id = AssetId::from([3u8; 32]);
    let initialize = initialize.into();
    if let Some(initialize) = initialize {
        storage
            .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, initialize)
            .unwrap();
    }
    let context = if external {
        Context::Script {
            block_height: Default::default(),
        }
    } else {
        Context::Call {
            block_height: Default::default(),
        }
    };
    let mut pc = 4;
    burn(
        &mut storage,
        &memory,
        &context,
        Reg::new(&fp),
        RegMut::new(&mut pc),
        amount,
    )?;
    assert_eq!(pc, 8);
    let result = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();
    assert_eq!(result, initialize.unwrap_or(0) - amount);
    Ok(())
}

#[test_case(false, 0, None, 0 => Ok(()); "Mint nothing")]
#[test_case(false, 0, 0, 0 => Ok(()); "Mint is idempotent")]
#[test_case(false, 0, 100, 0 => Ok(()); "Mint is idempotent any")]
#[test_case(false, 0, 100, 100 => Ok(()); "Mint Double")]
#[test_case(false, 0, 100, 10 => Ok(()); "Mint some")]
#[test_case(false, 0, None, 10 => Ok(()); "Mint some from nothing")]
#[test_case(false, 0, 0, 10 => Ok(()); "Mint some from zero")]
#[test_case(false, 0, None, Word::MAX => Ok(()); "Mint max from nothing")]
#[test_case(false, 0, 0, Word::MAX => Ok(()); "Mint max from zero")]
#[test_case(true, 0, 100, 10 => Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)); "Can't mint from external context")]
#[test_case(false, 0, 1, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::ArithmeticOverflow)); "Can't mint too much")]
fn test_mint(external: bool, fp: Word, initialize: impl Into<Option<Word>>, amount: Word) -> Result<(), RuntimeError> {
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory = VmMemory::fully_allocated();
    let contract_id = ContractId::from([3u8; ContractId::LEN]);
    memory.force_write_bytes(0, &contract_id);
    let asset_id = AssetId::from([3u8; 32]);
    let initialize = initialize.into();
    if let Some(initialize) = initialize {
        storage
            .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, initialize)
            .unwrap();
    }
    let context = if external {
        Context::Script {
            block_height: Default::default(),
        }
    } else {
        Context::Call {
            block_height: Default::default(),
        }
    };
    let mut pc = 4;
    mint(
        &mut storage,
        &memory,
        &context,
        Reg::new(&fp),
        RegMut::new(&mut pc),
        amount,
    )?;
    assert_eq!(pc, 8);
    let result = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();
    assert_eq!(result, initialize.unwrap_or(0) + amount);
    Ok(())
}

#[test]
fn test_block_hash() {
    let storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory = VmMemory::fully_allocated();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1,
        hp: 2000,
        prev_hp: 3000,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    let mut pc = 4;
    block_hash(&storage, &mut memory, owner, RegMut::new(&mut pc), 20, 40).unwrap();
    assert_eq!(pc, 8);
    let mem_bytes = Bytes32::from(memory.read_bytes(20).unwrap());
    assert_ne!(*mem_bytes, [1u8; 32]);
}

#[test]
fn test_block_height() {
    let context = Context::Script {
        block_height: 20.into(),
    };
    let mut pc = 4;
    let mut result = 0;
    block_height(&context, RegMut::new(&mut pc), &mut result).unwrap();
    assert_eq!(pc, 8);
    assert_eq!(result, 20);
}

#[test]
fn test_coinbase() {
    let storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory = VmMemory::fully_allocated();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1,
        hp: 2000,
        prev_hp: 3000,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    let mut pc = 4;
    coinbase(&storage, &mut memory, owner, RegMut::new(&mut pc), 20).unwrap();
    assert_eq!(pc, 8);
    let mem_bytes = Bytes32::from(memory.read_bytes(20).unwrap());
    assert_eq!(*mem_bytes, [0u8; 32]);
}

#[test]
fn test_code_root() {
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory = VmMemory::fully_allocated();
    let contract_id = ContractId::from([3u8; ContractId::LEN]);
    memory.force_write_bytes(0, &contract_id);
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1,
        hp: 2000,
        prev_hp: 3000,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    let mut pc = 4;
    let _ = code_root(&storage, &mut memory, owner, RegMut::new(&mut pc), 20, 0).expect_err("Contract is not found");
    assert_eq!(pc, 4);

    storage
        .storage_contract_root_insert(
            &ContractId::from([3u8; 32]),
            &Salt::from([5u8; 32]),
            &Bytes32::from([6u8; 32]),
        )
        .unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1,
        hp: 2000,
        prev_hp: 3000,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    code_root(&storage, &mut memory, owner, RegMut::new(&mut pc), 20, 0).unwrap();
    assert_eq!(pc, 8);
    let mem_bytes = Bytes32::from(memory.read_bytes(20).unwrap());
    assert_eq!(*mem_bytes, [6u8; 32]);
}

#[test]
fn test_code_size() {
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory = VmMemory::fully_allocated();
    let contract_id = ContractId::from([3u8; ContractId::LEN]);
    memory.force_write_bytes(0, &contract_id);

    StorageAsMut::storage::<ContractsRawCode>(&mut storage)
        .write(&ContractId::from([3u8; 32]), vec![1u8; 100])
        .unwrap();
    let mut pc = 4;
    let is = 0;
    let mut cgas = 0;
    let mut ggas = 0;
    let input = CodeSizeCtx {
        storage: &mut storage,
        memory: &mut memory,
        gas_cost: DependentCost {
            base: 0,
            dep_per_unit: 0,
        },
        profiler: &mut Profiler::default(),
        current_contract: None,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        pc: RegMut::new(&mut pc),
        is: Reg::new(&is),
    };
    let mut result = 0;
    let _ = input.code_size(&mut result, 1).expect_err("Contract is not found");
    assert_eq!(pc, 4);

    let input = CodeSizeCtx {
        storage: &mut storage,
        memory: &mut memory,
        gas_cost: DependentCost {
            base: 0,
            dep_per_unit: 0,
        },
        profiler: &mut Profiler::default(),
        current_contract: None,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        pc: RegMut::new(&mut pc),
        is: Reg::new(&is),
    };
    let mut result = 0;
    input.code_size(&mut result, 0).unwrap();
    assert_eq!(pc, 8);
    assert_eq!(result, 100);
}

#[test]
fn test_timestamp() {
    let storage = MemoryStorage::new(Default::default(), Default::default());
    let mut pc = 4;
    let mut result = 0;
    let _ = timestamp(&storage, Default::default(), RegMut::new(&mut pc), &mut result, 1)
        .expect_err("Height is greater then current block height");
    let _ = timestamp(
        &storage,
        u32::MAX.into(),
        RegMut::new(&mut pc),
        &mut result,
        u32::MAX as Word + 1,
    )
    .expect_err("Height doesn't fit into a u32");
    assert_eq!(pc, 4);

    timestamp(&storage, Default::default(), RegMut::new(&mut pc), &mut result, 0).unwrap();
    assert_eq!(pc, 8);

    timestamp(&storage, 20.into(), RegMut::new(&mut pc), &mut result, 19).unwrap();
    assert_eq!(pc, 12);
}
