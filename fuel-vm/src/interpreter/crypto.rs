use super::internal::{clear_err, set_err};
use super::{ExecutableTransaction, Interpreter};
use crate::constraints::reg_key::*;
use crate::error::RuntimeError;

use fuel_crypto::{Hasher, Message, PublicKey, Signature};
use fuel_types::Word;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn ecrecover(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let signature = Signature::from_bytes(self.mem_read_bytes(b)?);
        let message = Message::from_bytes(self.mem_read_bytes(c)?);

        match signature.recover(&message) {
            Ok(pub_key) => {
                self.mem_write_slice(a, pub_key.as_ref())?;
                clear_err(self.registers.err_mut());
            }
            Err(_) => {
                self.mem_write(a, PublicKey::LEN)?.fill(0);
                set_err(self.registers.err_mut());
            }
        }

        Ok(())
    }

    pub(crate) fn keccak256(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        use sha3::{Digest, Keccak256};

        let data = self.mem_read(b, c)?;
        let mut h = Keccak256::default();
        h.update(data);
        self.mem_write_slice(a, h.finalize().as_slice())
    }

    pub(crate) fn sha256(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let data = self.mem_read(b, c)?;
        let h = Hasher::hash(data);
        self.mem_write_slice(a, &*h)
    }
}
