use super::{Receipt, ReceiptRepr};

use fuel_asm::InstructionResult;
use fuel_types::bytes::{self, SizedBytes, WORD_SIZE};
use fuel_types::Word;

use crate::receipt::script_result::ScriptExecutionResult;
use std::io::{self, Write};

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
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Call as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_array_unchecked(buf, to);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, asset_id);
                let buf = bytes::store_number_unchecked(buf, *gas);
                let buf = bytes::store_number_unchecked(buf, *param1);
                let buf = bytes::store_number_unchecked(buf, *param2);
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
            }

            Self::Return { id, val, pc, is } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Return as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *val);
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
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
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::ReturnData as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ptr);
                let buf = bytes::store_number_unchecked(buf, *len);
                let buf = bytes::store_array_unchecked(buf, digest);
                let (_, buf) = bytes::store_bytes(buf, data)?;
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
            }

            Self::Panic {
                id, reason, pc, is, ..
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Panic as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *reason);
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
            }

            Self::Revert { id, ra, pc, is } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Revert as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ra);
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
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
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Log as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ra);
                let buf = bytes::store_number_unchecked(buf, *rb);
                let buf = bytes::store_number_unchecked(buf, *rc);
                let buf = bytes::store_number_unchecked(buf, *rd);
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
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
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::LogData as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ra);
                let buf = bytes::store_number_unchecked(buf, *rb);
                let buf = bytes::store_number_unchecked(buf, *ptr);
                let buf = bytes::store_number_unchecked(buf, *len);
                let buf = bytes::store_array_unchecked(buf, digest);
                let (_, buf) = bytes::store_bytes(buf, data)?;
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
            }

            Self::Transfer {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Transfer as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_array_unchecked(buf, to);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, asset_id);
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
            }

            Self::TransferOut {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::TransferOut as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_array_unchecked(buf, to);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, asset_id);
                let buf = bytes::store_number_unchecked(buf, *pc);

                bytes::store_number_unchecked(buf, *is);
            }

            Self::ScriptResult { result, gas_used } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::ScriptResult as Word);

                let result = Word::from(*result);
                let buf = bytes::store_number_unchecked(buf, result);

                bytes::store_number_unchecked(buf, *gas_used);
            }

            Self::MessageOut {
                message_id,
                sender,
                recipient,
                amount,
                nonce,
                len,
                digest,
                data,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::MessageOut as Word);

                let buf = bytes::store_array_unchecked(buf, message_id);
                let buf = bytes::store_array_unchecked(buf, sender);
                let buf = bytes::store_array_unchecked(buf, recipient);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, nonce);
                let buf = bytes::store_number_unchecked(buf, *len);
                let buf = bytes::store_array_unchecked(buf, digest);

                bytes::store_bytes(buf, data)?;
            }
        }

        Ok(len)
    }
}

impl io::Write for Receipt {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() < WORD_SIZE {
            return Err(bytes::eof());
        }

        // Safety: buffer size is checked
        let (identifier, buf) = unsafe { bytes::restore_word_unchecked(buf) };
        let identifier = ReceiptRepr::try_from(identifier)?;

        let orig_buf_len = buf.len();
        let mut used_len = Self::variant_len_without_data(identifier);

        if orig_buf_len < used_len {
            return Err(bytes::eof());
        }

        // Safety: buf len is checked
        match identifier {
            ReceiptRepr::Call => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (asset_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (gas, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (param1, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (param2, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let to = to.into();
                let asset_id = asset_id.into();

                *self = Self::call(id, to, amount, asset_id, gas, param1, param2, pc, is);
            }

            ReceiptRepr::Return => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (val, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::ret(id, val, pc, is);
            }

            ReceiptRepr::ReturnData => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ptr, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (len, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (digest, buf) = unsafe { bytes::restore_array_unchecked(buf) };

                let (count, data, buf) = bytes::restore_bytes(buf)?;

                used_len += count;
                if orig_buf_len < used_len {
                    return Err(bytes::eof());
                }

                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let digest = digest.into();

                *self = Self::return_data_with_len(id, ptr, len, digest, data, pc, is);
            }

            ReceiptRepr::Panic => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (reason, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::panic(id, InstructionResult::from(reason), pc, is);
            }

            ReceiptRepr::Revert => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ra, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::revert(id, ra, pc, is);
            }

            ReceiptRepr::Log => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ra, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rb, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rd, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::log(id, ra, rb, rc, rd, pc, is);
            }

            ReceiptRepr::LogData => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ra, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rb, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (ptr, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (len, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (digest, buf) = unsafe { bytes::restore_array_unchecked(buf) };

                let (count, data, buf) = bytes::restore_bytes(buf)?;

                used_len += count;
                if orig_buf_len < used_len {
                    return Err(bytes::eof());
                }

                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let digest = digest.into();

                *self = Self::log_data_with_len(id, ra, rb, ptr, len, digest, data, pc, is);
            }

            ReceiptRepr::Transfer => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (asset_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let to = to.into();
                let asset_id = asset_id.into();

                *self = Self::transfer(id, to, amount, asset_id, pc, is);
            }

            ReceiptRepr::TransferOut => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (asset_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let to = to.into();
                let asset_id = asset_id.into();

                *self = Self::transfer_out(id, to, amount, asset_id, pc, is);
            }

            ReceiptRepr::ScriptResult => {
                let (result, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (gas_used, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let result = ScriptExecutionResult::from(result);

                *self = Self::script_result(result, gas_used);
            }

            ReceiptRepr::MessageOut => {
                let (message_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (sender, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (recipient, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (nonce, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (len, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (digest, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (count, data, _) = bytes::restore_bytes(buf)?;

                used_len += count;
                if orig_buf_len < used_len {
                    return Err(bytes::eof());
                }

                let message_id = message_id.into();
                let sender = sender.into();
                let recipient = recipient.into();
                let nonce = nonce.into();
                let digest = digest.into();

                *self = Self::message_out_with_len(
                    message_id, sender, recipient, amount, nonce, len, digest, data,
                );
            }
        }

        Ok(used_len + WORD_SIZE)
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
