use super::{ExecutableTransaction, Interpreter};
use crate::call::{Call, CallFrame};
use crate::error::RuntimeError;
use crate::storage::InterpreterStorage;

use fuel_types::AssetId;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    pub(crate) fn call_frame(&self, call: Call, asset_id: AssetId) -> Result<CallFrame, RuntimeError> {
        let (to, a, b) = call.into_inner();

        let code = self.contract(&to)?.into_owned();
        let registers = self.registers;

        let frame = CallFrame::new(to, asset_id, registers, a, b, code);

        Ok(frame)
    }
}
