use super::Interpreter;
use crate::call::{Call, CallFrame};
use crate::data::InterpreterStorage;
use crate::error::InterpreterError;

use fuel_types::Color;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn call_frame(&self, call: Call, color: Color) -> Result<CallFrame, InterpreterError> {
        let (to, a, b) = call.into_inner();

        let code = self.contract(&to)?.into_owned();
        let registers = self.registers;

        let frame = CallFrame::new(to, color, registers, a, b, code);

        Ok(frame)
    }
}
