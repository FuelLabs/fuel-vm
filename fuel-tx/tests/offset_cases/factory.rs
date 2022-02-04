use fuel_tx::consts::MAX_GAS_PER_TX;
use fuel_tx::*;
use fuel_types::bytes::Deserializable;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub struct TransactionFactory {
    rng: StdRng,
    input_sampler: Uniform<usize>,
    output_sampler: Uniform<usize>,
    tx_sampler: Uniform<usize>,
}

impl TransactionFactory {
    pub fn from_seed(seed: u64) -> Self {
        StdRng::seed_from_u64(seed).into()
    }

    pub fn input(&mut self) -> Input {
        let variant = self.input_sampler.sample(&mut self.rng);

        match variant {
            0 => Input::coin(
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen::<Witness>().into_inner(),
                self.rng.gen::<Witness>().into_inner(),
            ),

            1 => Input::contract(
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
            ),

            _ => unreachable!(),
        }
    }

    pub fn output(&mut self) -> Output {
        let variant = self.output_sampler.sample(&mut self.rng);

        match variant {
            0 => Output::coin(self.rng.gen(), self.rng.gen(), self.rng.gen()),
            1 => Output::contract(self.rng.gen(), self.rng.gen(), self.rng.gen()),
            2 => Output::withdrawal(self.rng.gen(), self.rng.gen(), self.rng.gen()),
            3 => Output::change(self.rng.gen(), self.rng.gen(), self.rng.gen()),
            4 => Output::variable(self.rng.gen(), self.rng.gen(), self.rng.gen()),
            5 => Output::contract_created(self.rng.gen(), self.rng.gen()),

            _ => unreachable!(),
        }
    }

    pub fn transaction(&mut self) -> Transaction {
        let variant = self.tx_sampler.sample(&mut self.rng);
        let contracts = self.rng.gen_range(0..10);
        let inputs = self.rng.gen_range(0..10);
        let outputs = self.rng.gen_range(0..10);
        let witnesses = self.rng.gen_range(0..10);
        let storage = self.rng.gen_range(0..10);

        match variant {
            0 => Transaction::script(
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen::<Witness>().into_inner(),
                self.rng.gen::<Witness>().into_inner(),
                (0..inputs).map(|_| self.input()).collect(),
                (0..outputs).map(|_| self.output()).collect(),
                (0..witnesses).map(|_| self.rng.gen()).collect(),
            ),

            1 => Transaction::create(
                self.rng.gen(),
                MAX_GAS_PER_TX,
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                (0..contracts).map(|_| self.rng.gen()).collect(),
                (0..storage).map(|_| self.rng.gen()).collect(),
                (0..inputs).map(|_| self.input()).collect(),
                (0..outputs).map(|_| self.output()).collect(),
                (0..witnesses).map(|_| self.rng.gen()).collect(),
            ),

            _ => unreachable!(),
        }
    }
}

impl Iterator for TransactionFactory {
    type Item = Transaction;

    fn next(&mut self) -> Option<Transaction> {
        Some(self.transaction())
    }
}

impl From<StdRng> for TransactionFactory {
    fn from(rng: StdRng) -> Self {
        // Trick to enforce coverage of all variants in compile-time
        let input_sampler = Uniform::from(0..2);
        Input::from_bytes(&[])
            .map(|i| match i {
                Input::Coin { .. } => (),
                Input::Contract { .. } => (),
            })
            .unwrap_or(());

        let output_sampler = Uniform::from(0..6);
        Output::from_bytes(&[])
            .map(|o| match o {
                Output::Coin { .. } => (),
                Output::Contract { .. } => (),
                Output::Withdrawal { .. } => (),
                Output::Change { .. } => (),
                Output::Variable { .. } => (),
                Output::ContractCreated { .. } => (),
            })
            .unwrap_or(());

        let tx_sampler = Uniform::from(0..2);
        Transaction::from_bytes(&[])
            .map(|t| match t {
                Transaction::Script { .. } => (),
                Transaction::Create { .. } => (),
            })
            .unwrap_or(());

        Self {
            rng,
            input_sampler,
            output_sampler,
            tx_sampler,
        }
    }
}
