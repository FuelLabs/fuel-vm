use crate::data::InterpreterStorage;
use crate::interpreter::{ExecuteError, Interpreter};

use fuel_asm::Word;
use fuel_tx::consts::*;
use fuel_tx::{Bytes32, Color};

use std::convert::TryFrom;
use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn external_color_balance_sub(&mut self, color: &Color, value: Word) -> Result<(), ExecuteError> {
        if value == 0 {
            return Ok(());
        }

        const LEN: usize = Color::size_of() + WORD_SIZE;

        let balance_memory = self.memory[Bytes32::size_of()..Bytes32::size_of() + MAX_INPUTS as usize * LEN]
            .chunks_mut(LEN)
            .find(|chunk| &chunk[..Color::size_of()] == color.as_ref())
            .map(|chunk| &mut chunk[Color::size_of()..])
            .ok_or(ExecuteError::ExternalColorNotFound)?;

        let balance = <[u8; WORD_SIZE]>::try_from(&*balance_memory).expect("Sized chunk expected to fit!");
        let balance = Word::from_be_bytes(balance);
        let balance = balance.checked_sub(value).ok_or(ExecuteError::NotEnoughBalance)?;
        let balance = balance.to_be_bytes();

        balance_memory.copy_from_slice(&balance);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn external_balance() {
        let mut rng = StdRng::seed_from_u64(2322u64);

        let storage = MemoryStorage::default();
        let mut vm = Interpreter::with_storage(storage);

        let gas_price = 0;
        let gas_limit = 1_000_000;
        let maturity = 0;

        let script = vec![Opcode::RET(0x01)].iter().copied().collect();
        let balances = vec![(rng.gen(), 100), (rng.gen(), 500)];

        let inputs = balances
            .iter()
            .map(|(color, amount)| Input::coin(rng.gen(), rng.gen(), *amount, *color, 0, maturity, vec![], vec![]))
            .collect();

        let tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            script,
            vec![],
            inputs,
            vec![],
            vec![vec![].into()],
        );

        vm.init(tx).expect("Failed to init VM!");

        for (color, amount) in balances {
            assert!(vm.external_color_balance_sub(&color, amount + 1).is_err());
            vm.external_color_balance_sub(&color, amount - 10).unwrap();
            assert!(vm.external_color_balance_sub(&color, 11).is_err());
            vm.external_color_balance_sub(&color, 10).unwrap();
            assert!(vm.external_color_balance_sub(&color, 1).is_err());
        }
    }
}
