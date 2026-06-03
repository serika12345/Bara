use bara_ir::{UnsupportedReason, X86Va};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedFunction {
    entry: X86Va,
    instructions: Vec<DecodedInstruction>,
}

impl DecodedFunction {
    pub fn new(entry: X86Va, instructions: Vec<DecodedInstruction>) -> Result<Self, DecodeError> {
        if instructions.is_empty() {
            return Err(DecodeError::EmptyFunction { entry });
        }

        Ok(Self {
            entry,
            instructions,
        })
    }

    pub const fn entry(&self) -> X86Va {
        self.entry
    }

    pub fn instructions(&self) -> &[DecodedInstruction] {
        &self.instructions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedInstruction {
    start: X86Va,
    end: X86Va,
    kind: DecodedInstructionKind,
}

impl DecodedInstruction {
    pub const fn new(start: X86Va, end: X86Va, kind: DecodedInstructionKind) -> Self {
        Self { start, end, kind }
    }

    pub const fn start(&self) -> X86Va {
        self.start
    }

    pub const fn end(&self) -> X86Va {
        self.end
    }

    pub const fn kind(&self) -> &DecodedInstructionKind {
        &self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedInstructionKind {
    MovEaxImm32 { imm: u32 },
    Ret,
    Unsupported { reason: UnsupportedReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeError {
    EmptyFunction { entry: X86Va },
    AddressOverflow { at: X86Va, byte_len: u64 },
    TruncatedInstruction { at: X86Va, opcode: u8 },
}

pub fn decode_function(input: &X86Bytes) -> Result<DecodedFunction, DecodeError> {
    let mut offset = 0usize;
    let mut instructions = Vec::new();

    while offset < input.bytes().len() {
        let at =
            input
                .entry()
                .checked_add(offset as u64)
                .map_err(|_| DecodeError::AddressOverflow {
                    at: input.entry(),
                    byte_len: offset as u64,
                })?;
        let opcode = input.bytes()[offset];

        match opcode {
            0xb8 => {
                let end_offset = offset + 5;
                let imm_bytes = input
                    .bytes()
                    .get((offset + 1)..end_offset)
                    .ok_or(DecodeError::TruncatedInstruction { at, opcode })?;
                let imm =
                    u32::from_le_bytes([imm_bytes[0], imm_bytes[1], imm_bytes[2], imm_bytes[3]]);
                let end = input
                    .entry()
                    .checked_add(end_offset as u64)
                    .map_err(|_| DecodeError::AddressOverflow { at, byte_len: 5 })?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::MovEaxImm32 { imm },
                ));
                offset = end_offset;
            }
            0xc3 => {
                let end = input
                    .entry()
                    .checked_add((offset + 1) as u64)
                    .map_err(|_| DecodeError::AddressOverflow { at, byte_len: 1 })?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::Ret,
                ));
                return DecodedFunction::new(input.entry(), instructions);
            }
            unsupported => {
                let end = input
                    .entry()
                    .checked_add((offset + 1) as u64)
                    .map_err(|_| DecodeError::AddressOverflow { at, byte_len: 1 })?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::Unsupported {
                        reason: UnsupportedReason::DecodeUnsupportedOpcode {
                            opcode: unsupported,
                            at,
                        },
                    },
                ));
                return DecodedFunction::new(input.entry(), instructions);
            }
        }
    }

    let at = input
        .entry()
        .checked_add(input.bytes().len() as u64)
        .map_err(|_| DecodeError::AddressOverflow {
            at: input.entry(),
            byte_len: input.bytes().len() as u64,
        })?;
    let end = at
        .checked_add(1)
        .map_err(|_| DecodeError::AddressOverflow { at, byte_len: 1 })?;
    instructions.push(DecodedInstruction::new(
        at,
        end,
        DecodedInstructionKind::Unsupported {
            reason: UnsupportedReason::MissingReturnTerminator { at },
        },
    ));
    DecodedFunction::new(input.entry(), instructions)
}
