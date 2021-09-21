use super::Interpreter;
use crate::contract::Contract;
use crate::data::InterpreterStorage;
use crate::error::InterpreterError;

use fuel_data::{Color, ContractId, Word};

use std::borrow::Cow;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn contract(&self, contract: &ContractId) -> Result<Cow<'_, Contract>, InterpreterError> {
        self.storage
            .storage_contract(contract)
            .transpose()
            .ok_or(InterpreterError::ContractNotFound)?
    }

    pub(crate) fn check_contract_exists(&self, contract: &ContractId) -> Result<bool, InterpreterError> {
        self.storage.storage_contract_exists(contract)
    }

    pub(crate) fn balance(&self, contract: &ContractId, color: &Color) -> Result<Word, InterpreterError> {
        Ok(self
            .storage
            .merkle_contract_color_balance(contract, color)?
            .unwrap_or_default())
    }
}

#[cfg(all(test, feature = "random"))]
mod tests {
    use crate::consts::*;
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn mint_burn() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let mut balance = 1000;

        let mut vm = Interpreter::in_memory();

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

        let contract = Contract::from(program.as_ref());
        let contract_root = contract.root();
        let contract = contract.id(&salt, &contract_root);

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

        let script_data_offset = VM_TX_MEMORY + tx.script_data_offset().unwrap();
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
