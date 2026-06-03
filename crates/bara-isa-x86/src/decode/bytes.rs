use bara_ir::X86Va;

use super::DecodeError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct X86Bytes {
    entry: X86Va,
    bytes: Vec<u8>,
}

impl X86Bytes {
    pub fn new(entry: X86Va, bytes: Vec<u8>) -> Result<Self, DecodeError> {
        if bytes.is_empty() {
            return Err(DecodeError::EmptyFunction { entry });
        }

        Ok(Self { entry, bytes })
    }

    pub const fn entry(&self) -> X86Va {
        self.entry
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
