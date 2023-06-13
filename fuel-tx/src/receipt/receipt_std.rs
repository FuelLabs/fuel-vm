use super::{Receipt, ReceiptRepr};

use fuel_asm::PanicInstruction;
use fuel_types::bytes::{self, SizedBytes, WORD_SIZE};
use fuel_types::{MemLayout, MemLocType, Word};

use crate::receipt::script_result::ScriptExecutionResult;
use crate::receipt::sizes::CallSizes;
use std::io::{self, Write};

use crate::receipt::sizes::*;

impl io::Read for Receipt {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.serialized_size();

        if buf.len() < len {
            return Err(bytes::eof());
        }

        match self {
            Self::Call {
                id,
                to,
                amount,
                asset_id,
                gas,
                param1,
                param2,
                pc,
                is,
            } => {
                type S = CallSizes;
                const LEN: usize = CallSizes::LEN;
                let buf: &mut [_; LEN] = buf
                    .get_mut(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::Call as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_at(buf, S::layout(S::LAYOUT.to), to);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_at(buf, S::layout(S::LAYOUT.asset_id), asset_id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.gas), *gas);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.param1), *param1);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.param2), *param2);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);
            }

            Self::Return { id, val, pc, is } => {
                type S = ReturnSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::Return as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.val), *val);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);
            }

            Self::ReturnData {
                id,
                ptr,
                len,
                digest,
                data,
                pc,
                is,
            } => {
                let full_buf = buf;
                type S = ReturnDataSizes;
                let buf: &mut [_; S::LEN] = full_buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::ReturnData as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.ptr), *ptr);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.len), *len);
                bytes::store_at(buf, S::layout(S::LAYOUT.digest), digest);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);

                bytes::store_bytes(full_buf.get_mut(S::LEN..).ok_or(bytes::eof())?, data)?;
            }

            Self::Panic { id, reason, pc, is, .. } => {
                type S = PanicSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::Panic as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.reason), Word::from(*reason));
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);
            }

            Self::Revert { id, ra, pc, is } => {
                type S = RevertSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::Revert as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.ra), *ra);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);
            }

            Self::Log {
                id,
                ra,
                rb,
                rc,
                rd,
                pc,
                is,
            } => {
                type S = LogSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::Log as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.ra), *ra);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.rb), *rb);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.rc), *rc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.rd), *rd);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);
            }

            Self::LogData {
                id,
                ra,
                rb,
                ptr,
                len,
                digest,
                data,
                pc,
                is,
            } => {
                let full_buf = buf;
                type S = LogDataSizes;
                let buf: &mut [_; S::LEN] = full_buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::LogData as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.ra), *ra);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.rb), *rb);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.ptr), *ptr);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.len), *len);
                bytes::store_at(buf, S::layout(S::LAYOUT.digest), digest);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);

                bytes::store_bytes(full_buf.get_mut(S::LEN..).ok_or(bytes::eof())?, data)?;
            }

            Self::Transfer {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            } => {
                type S = TransferSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::Transfer as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_at(buf, S::layout(S::LAYOUT.to), to);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_at(buf, S::layout(S::LAYOUT.asset_id), asset_id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);
            }

            Self::TransferOut {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            } => {
                type S = TransferOutSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::TransferOut as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.id), id);
                bytes::store_at(buf, S::layout(S::LAYOUT.to), to);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_at(buf, S::layout(S::LAYOUT.asset_id), asset_id);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.pc), *pc);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.is), *is);
            }

            Self::ScriptResult { result, gas_used } => {
                type S = ScriptResultSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::ScriptResult as u8);

                let result = Word::from(*result);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.result), result);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.gas_used), *gas_used);
            }

            Self::MessageOut {
                sender,
                recipient,
                amount,
                nonce,
                len,
                digest,
                data,
            } => {
                let full_buf = buf;
                type S = MessageOutSizes;
                let buf: &mut [_; S::LEN] = full_buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), ReceiptRepr::MessageOut as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.sender), sender);
                bytes::store_at(buf, S::layout(S::LAYOUT.recipient), recipient);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_at(buf, S::layout(S::LAYOUT.nonce), nonce);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.len), *len);
                bytes::store_at(buf, S::layout(S::LAYOUT.digest), digest);

                bytes::store_bytes(full_buf.get_mut(S::LEN..).ok_or(bytes::eof())?, data)?;
            }
        }

        Ok(len)
    }
}

impl io::Write for Receipt {
    fn write(&mut self, full_buf: &[u8]) -> io::Result<usize> {
        let identifier: &[_; WORD_SIZE] = full_buf
            .get(..WORD_SIZE)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        // Safety: buf len is checked
        let identifier = bytes::restore_word(bytes::from_array(identifier));
        let identifier = ReceiptRepr::try_from(identifier)?;

        match identifier {
            ReceiptRepr::Call => {
                type S = CallSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let to = bytes::restore_at(buf, S::layout(S::LAYOUT.to));
                let amount = bytes::restore_word_at(buf, S::layout(S::LAYOUT.amount));
                let asset_id = bytes::restore_at(buf, S::layout(S::LAYOUT.asset_id));
                let gas = bytes::restore_word_at(buf, S::layout(S::LAYOUT.gas));
                let param1 = bytes::restore_word_at(buf, S::layout(S::LAYOUT.param1));
                let param2 = bytes::restore_word_at(buf, S::layout(S::LAYOUT.param2));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let id = id.into();
                let to = to.into();
                let asset_id = asset_id.into();

                *self = Self::call(id, to, amount, asset_id, gas, param1, param2, pc, is);
            }

            ReceiptRepr::Return => {
                type S = ReturnSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let val = bytes::restore_word_at(buf, S::layout(S::LAYOUT.val));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let id = id.into();

                *self = Self::ret(id, val, pc, is);
            }

            ReceiptRepr::ReturnData => {
                type S = ReturnDataSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let ptr = bytes::restore_word_at(buf, S::layout(S::LAYOUT.ptr));
                let len = bytes::restore_word_at(buf, S::layout(S::LAYOUT.len));
                let digest = bytes::restore_at(buf, S::layout(S::LAYOUT.digest));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let (_, data, _) = bytes::restore_bytes(full_buf.get(S::LEN..).ok_or(bytes::eof())?)?;

                let id = id.into();
                let digest = digest.into();

                *self = Self::return_data_with_len(id, ptr, len, digest, data, pc, is);
            }

            ReceiptRepr::Panic => {
                type S = PanicSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let reason = bytes::restore_word_at(buf, S::layout(S::LAYOUT.reason));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let id = id.into();

                *self = Self::panic(id, PanicInstruction::from(reason), pc, is);
            }

            ReceiptRepr::Revert => {
                type S = RevertSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let ra = bytes::restore_word_at(buf, S::layout(S::LAYOUT.ra));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let id = id.into();

                *self = Self::revert(id, ra, pc, is);
            }

            ReceiptRepr::Log => {
                type S = LogSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let ra = bytes::restore_word_at(buf, S::layout(S::LAYOUT.ra));
                let rb = bytes::restore_word_at(buf, S::layout(S::LAYOUT.rb));
                let rc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.rc));
                let rd = bytes::restore_word_at(buf, S::layout(S::LAYOUT.rd));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let id = id.into();

                *self = Self::log(id, ra, rb, rc, rd, pc, is);
            }

            ReceiptRepr::LogData => {
                type S = LogDataSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let ra = bytes::restore_word_at(buf, S::layout(S::LAYOUT.ra));
                let rb = bytes::restore_word_at(buf, S::layout(S::LAYOUT.rb));
                let ptr = bytes::restore_word_at(buf, S::layout(S::LAYOUT.ptr));
                let len = bytes::restore_word_at(buf, S::layout(S::LAYOUT.len));
                let digest = bytes::restore_at(buf, S::layout(S::LAYOUT.digest));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let (_, data, _) = bytes::restore_bytes(full_buf.get(S::LEN..).ok_or(bytes::eof())?)?;

                let id = id.into();
                let digest = digest.into();

                *self = Self::log_data_with_len(id, ra, rb, ptr, len, digest, data, pc, is);
            }

            ReceiptRepr::Transfer => {
                type S = TransferSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let to = bytes::restore_at(buf, S::layout(S::LAYOUT.to));
                let amount = bytes::restore_word_at(buf, S::layout(S::LAYOUT.amount));
                let asset_id = bytes::restore_at(buf, S::layout(S::LAYOUT.asset_id));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let id = id.into();
                let to = to.into();
                let asset_id = asset_id.into();

                *self = Self::transfer(id, to, amount, asset_id, pc, is);
            }

            ReceiptRepr::TransferOut => {
                type S = TransferOutSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let id = bytes::restore_at(buf, S::layout(S::LAYOUT.id));
                let to = bytes::restore_at(buf, S::layout(S::LAYOUT.to));
                let amount = bytes::restore_word_at(buf, S::layout(S::LAYOUT.amount));
                let asset_id = bytes::restore_at(buf, S::layout(S::LAYOUT.asset_id));
                let pc = bytes::restore_word_at(buf, S::layout(S::LAYOUT.pc));
                let is = bytes::restore_word_at(buf, S::layout(S::LAYOUT.is));

                let id = id.into();
                let to = to.into();
                let asset_id = asset_id.into();

                *self = Self::transfer_out(id, to, amount, asset_id, pc, is);
            }

            ReceiptRepr::ScriptResult => {
                type S = ScriptResultSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;
                let result = bytes::restore_word_at(buf, S::layout(S::LAYOUT.result));
                let gas_used = bytes::restore_word_at(buf, S::layout(S::LAYOUT.gas_used));

                let result = ScriptExecutionResult::from(result);

                *self = Self::script_result(result, gas_used);
            }

            ReceiptRepr::MessageOut => {
                type S = MessageOutSizes;
                let buf: &[_; S::LEN] = full_buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let sender = bytes::restore_at(buf, S::layout(S::LAYOUT.sender));
                let recipient = bytes::restore_at(buf, S::layout(S::LAYOUT.recipient));
                let amount = bytes::restore_word_at(buf, S::layout(S::LAYOUT.amount));
                let nonce = bytes::restore_at(buf, S::layout(S::LAYOUT.nonce));
                let len = bytes::restore_word_at(buf, S::layout(S::LAYOUT.len));
                let digest = bytes::restore_at(buf, S::layout(S::LAYOUT.digest));

                let (_, data, _) = bytes::restore_bytes(full_buf.get(S::LEN..).ok_or(bytes::eof())?)?;

                let sender = sender.into();
                let recipient = recipient.into();
                let nonce = nonce.into();
                let digest = digest.into();

                *self = Self::message_out_with_len(sender, recipient, amount, nonce, len, digest, data);
            }
        }

        let n = self.serialized_size();
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl bytes::Deserializable for Receipt {
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut instance = Self::ret(Default::default(), 0, 0, 0);

        // We are sure that all needed bytes are written or error would happen.
        // unused let is here to silence clippy warning for this check.
        let _ = instance.write(bytes)?;

        Ok(instance)
    }
}

impl TryFrom<Word> for ReceiptRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Call),
            0x01 => Ok(Self::Return),
            0x02 => Ok(Self::ReturnData),
            0x03 => Ok(Self::Panic),
            0x04 => Ok(Self::Revert),
            0x05 => Ok(Self::Log),
            0x06 => Ok(Self::LogData),
            0x07 => Ok(Self::Transfer),
            0x08 => Ok(Self::TransferOut),
            0x09 => Ok(Self::ScriptResult),
            0x0A => Ok(Self::MessageOut),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}
