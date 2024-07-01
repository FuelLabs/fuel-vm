#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::vec;

use crate::storage::MemoryStorage;
use core::convert::Infallible;

use super::*;
use crate::interpreter::PanicContext;
use fuel_storage::StorageAsMut;
use test_case::test_case;

#[test_case(false, 0, None, 0, [0; 32] => Ok(()); "Burn nothing")]
#[test_case(false, 0, 0, 0, [0; 32] => Ok(()); "Burn is idempotent")]
#[test_case(false, 0, 100, 100, [0; 32] => Ok(()); "Burn all")]
#[test_case(false, 0, 100, 100, [15; 32] => Ok(()); "Burn all for another sub id")]
#[test_case(false, 0, 100, 10, [0; 32] => Ok(()); "Burn some")]
#[test_case(true, 0, 100, 10, [0; 32] => Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)); "Can't burn from external context")]
#[test_case(false, 1, 100, 10, [0; 32] => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Can't burn when contract id not in memory")]
#[test_case(false, 0, 100, 101, [0; 32] => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Can't burn too much")]
#[test_case(false, 0, None, 1, [0; 32] => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Can't burn when no balance")]
fn test_burn(
    external: bool,
    fp: Word,
    initialize: impl Into<Option<Word>>,
    amount: Word,
    sub_id: [u8; 32],
) -> IoResult<(), Infallible> {
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let contract_id = ContractId::from([3u8; 32]);
    memory[0..ContractId::LEN].copy_from_slice(contract_id.as_slice());
    memory[ContractId::LEN..ContractId::LEN + Bytes32::LEN]
        .copy_from_slice(sub_id.as_slice());
    let sub_id = Bytes32::from(sub_id);
    let asset_id = contract_id.asset_id(&sub_id);
    let initialize = initialize.into();
    if let Some(initialize) = initialize {
        let old_balance = storage
            .contract_asset_id_balance_replace(&contract_id, &asset_id, initialize)
            .unwrap();
        assert!(old_balance.is_none());
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
    let mut receipts = Default::default();

    let is = 0;
    const ORIGINAL_PC: Word = 4;
    let mut pc = ORIGINAL_PC;
    BurnCtx {
        storage: &mut storage,
        context: &context,
        receipts: &mut receipts,
        memory: &mut memory,
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
        is: Reg::new(&is),
    }
    .burn(amount, ContractId::LEN as Word)?;
    assert_eq!(pc, 8);
    let result = storage
        .contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();
    assert_eq!(result, initialize.unwrap_or(0) - amount);
    assert_eq!(receipts.len(), 1);
    assert_eq!(
        receipts[0],
        Receipt::Burn {
            sub_id,
            contract_id,
            val: amount,
            pc: ORIGINAL_PC,
            is
        }
    );
    Ok(())
}

#[test_case(false, 0, None, 0, [0; 32] => Ok(()); "Mint nothing")]
#[test_case(false, 0, 0, 0, [0; 32] => Ok(()); "Mint is idempotent")]
#[test_case(false, 0, 100, 0, [0; 32] => Ok(()); "Mint is idempotent any")]
#[test_case(false, 0, 100, 100, [0; 32] => Ok(()); "Mint Double")]
#[test_case(false, 0, 100, 100, [15; 32] => Ok(()); "Mint Double for another sub id")]
#[test_case(false, 0, 100, 10, [0; 32] => Ok(()); "Mint some")]
#[test_case(false, 0, None, 10, [0; 32] => Ok(()); "Mint some from nothing")]
#[test_case(false, 0, 0, 10, [0; 32] => Ok(()); "Mint some from zero")]
#[test_case(false, 0, None, Word::MAX, [0; 32] => Ok(()); "Mint max from nothing")]
#[test_case(false, 0, 0, Word::MAX, [0; 32] => Ok(()); "Mint max from zero")]
#[test_case(true, 0, 100, 10, [0; 32] => Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)); "Can't mint from external context")]
#[test_case(false, 0, 1, Word::MAX, [0; 32] => Err(RuntimeError::Recoverable(PanicReason::BalanceOverflow)); "Can't mint too much")]
fn test_mint(
    external: bool,
    fp: Word,
    initialize: impl Into<Option<Word>>,
    amount: Word,
    sub_id: [u8; 32],
) -> IoResult<(), Infallible> {
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let contract_id = ContractId::from([3u8; 32]);
    memory[0..ContractId::LEN].copy_from_slice(contract_id.as_slice());
    memory[ContractId::LEN..ContractId::LEN + Bytes32::LEN]
        .copy_from_slice(sub_id.as_slice());
    let sub_id = Bytes32::from(sub_id);
    let asset_id = contract_id.asset_id(&sub_id);
    let initialize = initialize.into();
    if let Some(initialize) = initialize {
        let old_balance = storage
            .contract_asset_id_balance_replace(&contract_id, &asset_id, initialize)
            .unwrap();
        assert!(old_balance.is_none());
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

    let mut receipts = Default::default();

    let is = 0;
    const ORIGINAL_PC: Word = 4;
    let mut pc = ORIGINAL_PC;
    let mut cgas = 10_000;
    let mut ggas = 10_000;
    MintCtx {
        storage: &mut storage,
        context: &context,
        receipts: &mut receipts,
        memory: &mut memory,
        profiler: &mut Profiler::default(),
        new_storage_gas_per_byte: 1,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
        is: Reg::new(&is),
    }
    .mint(amount, ContractId::LEN as Word)?;
    assert_eq!(pc, 8);
    let result = storage
        .contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();
    assert_eq!(result, initialize.unwrap_or(0) + amount);
    assert_eq!(receipts.len(), 1);
    assert_eq!(
        receipts[0],
        Receipt::Mint {
            sub_id,
            contract_id,
            val: amount,
            pc: ORIGINAL_PC,
            is
        }
    );
    Ok(())
}

#[test]
fn test_block_hash() {
    let storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1,
        hp: 2000,
        prev_hp: 3000,
    };
    let mut pc = 4;
    block_hash(&storage, &mut memory, owner, RegMut::new(&mut pc), 20, 40).unwrap();
    assert_eq!(pc, 8);
    assert_ne!(memory[20..20 + 32], [1u8; 32]);
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
    let storage = MemoryStorage::new(Default::default(), ContractId::zeroed());
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1,
        hp: 2000,
        prev_hp: 3000,
    };
    let mut pc = 4;
    coinbase(&storage, &mut memory, owner, RegMut::new(&mut pc), 20).unwrap();
    assert_eq!(pc, 8);
    assert_eq!(memory[20..20 + 32], [0u8; 32]);
}

#[test]
fn test_code_size() {
    let contract_id = ContractId::new([3u8; ContractId::LEN]);
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[0..ContractId::LEN].copy_from_slice(contract_id.as_slice());
    StorageAsMut::storage::<ContractsRawCode>(&mut storage)
        .write_bytes(&ContractId::from([3u8; 32]), &[1u8; 100])
        .unwrap();
    let mut pc = 4;
    let is = 0;
    let mut cgas = 0;
    let mut ggas = 0;
    let input_contracts = [contract_id];
    let input_contracts = input_contracts.into_iter().collect();
    let mut panic_context = PanicContext::None;
    let input = CodeSizeCtx {
        storage: &mut storage,
        memory: &mut memory,
        gas_cost: DependentCost::free(),
        profiler: &mut Profiler::default(),
        input_contracts: InputContracts::new(&input_contracts, &mut panic_context),
        current_contract: None,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        pc: RegMut::new(&mut pc),
        is: Reg::new(&is),
    };
    let mut result = 0;
    let _ = input
        .code_size(&mut result, 1)
        .expect_err("Contract is not found");
    assert_eq!(pc, 4);

    let input = CodeSizeCtx {
        storage: &mut storage,
        memory: &mut memory,
        gas_cost: DependentCost::free(),
        input_contracts: InputContracts::new(&input_contracts, &mut panic_context),
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

    let input_contracts = Default::default();
    let input = CodeSizeCtx {
        storage: &mut storage,
        memory: &mut memory,
        gas_cost: DependentCost::free(),
        input_contracts: InputContracts::new(&input_contracts, &mut panic_context),
        profiler: &mut Profiler::default(),
        current_contract: None,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        pc: RegMut::new(&mut pc),
        is: Reg::new(&is),
    };
    let mut result = 0;
    let _ = input
        .code_size(&mut result, 0)
        .expect_err("The contract is not in the input");
}

#[test]
fn test_timestamp() {
    let storage = MemoryStorage::default();
    let mut pc = 4;
    let mut result = 0;
    let _ = timestamp(
        &storage,
        Default::default(),
        RegMut::new(&mut pc),
        &mut result,
        1,
    )
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

    timestamp(
        &storage,
        Default::default(),
        RegMut::new(&mut pc),
        &mut result,
        0,
    )
    .unwrap();
    assert_eq!(pc, 8);

    timestamp(&storage, 20.into(), RegMut::new(&mut pc), &mut result, 19).unwrap();
    assert_eq!(pc, 12);
}
