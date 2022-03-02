use super::Interpreter;
use crate::call::{Call, CallFrame};
use crate::error::RuntimeError;
use crate::storage::InterpreterStorage;

use fuel_types::AssetId;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn call_frame(&self, call: Call, asset_id: AssetId) -> Result<CallFrame, RuntimeError> {
        let (to, a, b) = call.into_inner();

        let code = self.contract(&to)?.into_owned();
        let registers = self.registers;

        let frame = CallFrame::new(to, asset_id, registers, a, b, code);

        Ok(frame)
    }
}
