use crate::storage::MemoryStorage;

use super::*;
use test_case::test_case;

#[test_case(0, 0 => Ok(()); "Can increase balance by zero")]
#[test_case(None, 0 => Ok(()); "Can initialize balance to zero")]
#[test_case(None, Word::MAX => Ok(()); "Can initialize balance to max")]
#[test_case(0, Word::MAX => Ok(()); "Can add max to zero")]
#[test_case(Word::MAX, 0 => Ok(()); "Can add zero to max")]
#[test_case(1, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::ArithmeticOverflow)); "Overflowing add")]
#[test_case(Word::MAX, 1 => Err(RuntimeError::Recoverable(PanicReason::ArithmeticOverflow)); "Overflowing 1 add")]
fn test_balance_increase(initial: impl Into<Option<Word>>, amount: Word) -> Result<(), RuntimeError> {
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let initial = initial.into();
    if let Some(initial) = initial {
        storage
            .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, initial)
            .unwrap();
    }

    let result = balance_increase(&mut storage, &contract_id, &asset_id, amount)?;
    let initial = initial.unwrap_or(0);

    assert_eq!(result, initial + amount);

    let result = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();

    assert_eq!(result, initial + amount);

    Ok(())
}

#[test_case(0, 0 => Ok(()); "Can increase balance by zero")]
#[test_case(None, 0 => Ok(()); "Can initialize balance to zero")]
#[test_case(Word::MAX, 0 => Ok(()); "Can initialize balance to max")]
#[test_case(10, 10 => Ok(()); "Can subtract to zero")]
#[test_case(1, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Overflowing subtract")]
#[test_case(1, 2 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Overflowing 1 subtract")]
fn test_balance_decrease(initial: impl Into<Option<Word>>, amount: Word) -> Result<(), RuntimeError> {
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let initial = initial.into();
    if let Some(initial) = initial {
        storage
            .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, initial)
            .unwrap();
    }

    let result = balance_decrease(&mut storage, &contract_id, &asset_id, amount)?;
    let initial = initial.unwrap_or(0);

    assert_eq!(result, initial - amount);

    let result = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();

    assert_eq!(result, initial - amount);

    Ok(())
}
