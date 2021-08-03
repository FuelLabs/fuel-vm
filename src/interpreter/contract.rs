use super::{ExecuteError, Interpreter};
use crate::consts::*;
use crate::crypto;
use crate::data::InterpreterStorage;

use fuel_asm::Word;
use fuel_tx::crypto as tx_crypto;
use fuel_tx::{Color, ContractId, Transaction, ValidationError};

use std::convert::TryFrom;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub struct Contract(Vec<u8>);

impl From<Vec<u8>> for Contract {
    fn from(c: Vec<u8>) -> Self {
        Self(c)
    }
}

impl From<&[u8]> for Contract {
    fn from(c: &[u8]) -> Self {
        Self(c.into())
    }
}

impl From<&mut [u8]> for Contract {
    fn from(c: &mut [u8]) -> Self {
        Self(c.into())
    }
}

impl From<Contract> for Vec<u8> {
    fn from(c: Contract) -> Vec<u8> {
        c.0
    }
}

impl AsRef<[u8]> for Contract {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for Contract {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl TryFrom<&Transaction> for Contract {
    type Error = ValidationError;

    fn try_from(tx: &Transaction) -> Result<Self, Self::Error> {
        match tx {
            Transaction::Create {
                bytecode_witness_index,
                witnesses,
                ..
            } => witnesses
                .get(*bytecode_witness_index as usize)
                .map(|c| c.as_ref().into())
                .ok_or(ValidationError::TransactionCreateBytecodeWitnessIndex),

            _ => Err(ValidationError::TransactionScriptOutputContractCreated { index: 0 }),
        }
    }
}

impl Contract {
    pub fn address(&self, salt: &[u8]) -> ContractId {
        let mut input = VM_CONTRACT_ID_BASE.to_vec();

        input.extend_from_slice(salt);
        input.extend_from_slice(crypto::merkle_root(self.0.as_slice()).as_ref());

        (*tx_crypto::hash(input.as_slice())).into()
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn contract(&self, address: &ContractId) -> Result<Option<Contract>, ExecuteError> {
        Ok(self.storage.get(address)?)
    }

    pub(crate) fn check_contract_exists(&self, address: &ContractId) -> Result<bool, ExecuteError> {
        Ok(self.storage.contains_key(address)?)
    }

    pub(crate) fn set_balance(
        &mut self,
        contract: ContractId,
        color: Color,
        balance: Word,
    ) -> Result<(), ExecuteError> {
        self.storage.insert((contract, color), balance)?;

        Ok(())
    }

    pub(crate) fn balance(&self, contract: &ContractId, color: &Color) -> Result<Word, ExecuteError> {
        Ok(self.storage.get(&(*contract, *color))?.unwrap_or(0))
    }

    pub(crate) fn balance_add(
        &mut self,
        contract: ContractId,
        color: Color,
        value: Word,
    ) -> Result<Word, ExecuteError> {
        let balance = self.balance(&contract, &color)?;
        let balance = balance.checked_add(value).ok_or(ExecuteError::NotEnoughBalance)?;

        self.set_balance(contract, color, balance)?;

        Ok(balance)
    }

    pub(crate) fn balance_sub(
        &mut self,
        contract: ContractId,
        color: Color,
        value: Word,
    ) -> Result<Word, ExecuteError> {
        let balance = self.balance(&contract, &color)?;
        let balance = balance.checked_sub(value).ok_or(ExecuteError::NotEnoughBalance)?;

        self.set_balance(contract, color, balance)?;

        Ok(balance)
    }
}

#[cfg(test)]
mod tests {
    use crate::consts::*;
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn mint_burn() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let mut balance = 1000;

        let storage = MemoryStorage::default();
        let mut vm = Interpreter::with_storage(storage);

        let gas_price = 0;
        let gas_limit = 1_000_000;
        let maturity = 0;

        let salt: Salt = rng.gen();
        let program: Witness = [
            Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
            Opcode::LW(0x10, 0x10, 0),
            Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
            Opcode::LW(0x11, 0x11, 0),
            Opcode::JNEI(0x10, REG_ZERO, 7),
            Opcode::MINT(0x11),
            Opcode::JI(8),
            Opcode::BURN(0x11),
            Opcode::RET(REG_ONE),
        ]
        .iter()
        .copied()
        .collect::<Vec<u8>>()
        .into();

        let contract = Contract::from(program.as_ref()).address(salt.as_ref());
        let color = Color::from(*contract);
        let output = Output::contract_created(contract);

        let bytecode_witness = 0;
        let tx = Transaction::create(
            gas_price,
            gas_limit,
            maturity,
            bytecode_witness,
            salt,
            vec![],
            vec![],
            vec![output],
            vec![program],
        );

        vm.transact(tx).expect("Failed to transact");

        let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract);
        let output = Output::contract(0, rng.gen(), rng.gen());

        let mut script_ops = vec![
            Opcode::ADDI(0x10, REG_ZERO, 0),
            Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
            Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
            Opcode::RET(REG_ONE),
        ];

        let script: Vec<u8> = script_ops.iter().copied().collect();
        let tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            script,
            vec![],
            vec![input.clone()],
            vec![output],
            vec![],
        );

        let script_data_offset = Interpreter::<()>::tx_mem_address() + tx.script_data_offset().unwrap();
        script_ops[0] = Opcode::ADDI(0x10, REG_ZERO, script_data_offset as Immediate12);

        let script: Vec<u8> = script_ops.iter().copied().collect();
        let script_data = Call::new(contract, 0, balance).to_bytes();
        let tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            script,
            script_data,
            vec![input.clone()],
            vec![output],
            vec![],
        );

        assert_eq!(0, vm.balance(&contract, &color).unwrap());
        vm.transact(tx).expect("Failed to transact");
        assert_eq!(balance as Word, vm.balance(&contract, &color).unwrap());

        // Try to burn more than the available balance
        let script: Vec<u8> = script_ops.iter().copied().collect();
        let script_data = Call::new(contract, 1, balance + 1).to_bytes();
        let tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            script,
            script_data,
            vec![input.clone()],
            vec![output],
            vec![],
        );

        assert!(vm.transact(tx).is_err());
        assert_eq!(balance as Word, vm.balance(&contract, &color).unwrap());

        // Burn some of the balance
        let burn = 100;

        let script: Vec<u8> = script_ops.iter().copied().collect();
        let script_data = Call::new(contract, 1, burn).to_bytes();
        let tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            script,
            script_data,
            vec![input.clone()],
            vec![output],
            vec![],
        );

        vm.transact(tx).expect("Failed to transact");
        balance -= burn;
        assert_eq!(balance as Word, vm.balance(&contract, &color).unwrap());

        // Burn the remainder balance
        let script: Vec<u8> = script_ops.iter().copied().collect();
        let script_data = Call::new(contract, 1, balance).to_bytes();
        let tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            script,
            script_data,
            vec![input.clone()],
            vec![output],
            vec![],
        );

        vm.transact(tx).expect("Failed to transact");
        assert_eq!(0, vm.balance(&contract, &color).unwrap());
    }
}
