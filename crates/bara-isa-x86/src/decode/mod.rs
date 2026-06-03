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
    AddEaxImm8 { imm: X86Imm8 },
    Ret,
    Unsupported { reason: UnsupportedReason },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86Imm8(i8);

impl X86Imm8 {
    pub const fn new(value: i8) -> Self {
        Self(value)
    }

    pub(crate) fn as_i64(self) -> i64 {
        i64::from(self.0)
    }
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
            0x83 => {
                let end_offset = offset + 3;
                let operand = input
                    .bytes()
                    .get(offset + 1)
                    .ok_or(DecodeError::TruncatedInstruction { at, opcode })?;
                let imm = input
                    .bytes()
                    .get(offset + 2)
                    .ok_or(DecodeError::TruncatedInstruction { at, opcode })?;
                let end = input
                    .entry()
                    .checked_add(end_offset as u64)
                    .map_err(|_| DecodeError::AddressOverflow { at, byte_len: 3 })?;

                if *operand == 0xc0 {
                    instructions.push(DecodedInstruction::new(
                        at,
                        end,
                        DecodedInstructionKind::AddEaxImm8 {
                            imm: X86Imm8::new(i8::from_le_bytes([*imm])),
                        },
                    ));
                    offset = end_offset;
                } else {
                    instructions.push(DecodedInstruction::new(
                        at,
                        end,
                        DecodedInstructionKind::Unsupported {
                            reason: UnsupportedReason::DecodeUnsupportedOpcode { opcode, at },
                        },
                    ));
                    return DecodedFunction::new(input.entry(), instructions);
                }
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

#[cfg(test)]
mod tests {
    use bara_ir::{UnsupportedReason, X86Va};

    use crate::{
        decode_function, DecodeError, DecodedFunction, DecodedInstruction, DecodedInstructionKind,
        X86Bytes,
    };

    #[test]
    fn x86_bytes_reject_empty_function() {
        assert_eq!(
            X86Bytes::new(X86Va::new(0x1000), Vec::new()),
            Err(DecodeError::EmptyFunction {
                entry: X86Va::new(0x1000)
            })
        );
    }

    #[test]
    fn x86_bytes_exposes_entry_and_bytes() {
        let bytes =
            X86Bytes::new(X86Va::new(0x1000), vec![0xc3]).expect("test bytes are non-empty");

        assert_eq!(bytes.entry(), X86Va::new(0x1000));
        assert_eq!(bytes.bytes(), &[0xc3]);
    }

    #[test]
    fn decoded_function_rejects_empty_instruction_list() {
        assert_eq!(
            DecodedFunction::new(X86Va::new(0), Vec::new()),
            Err(DecodeError::EmptyFunction {
                entry: X86Va::new(0)
            })
        );
    }

    #[test]
    fn decoded_instruction_exposes_fields() {
        let instruction = DecodedInstruction::new(
            X86Va::new(0),
            X86Va::new(5),
            DecodedInstructionKind::MovEaxImm32 { imm: 42 },
        );

        assert_eq!(instruction.start(), X86Va::new(0));
        assert_eq!(instruction.end(), X86Va::new(5));
        assert_eq!(
            instruction.kind(),
            &DecodedInstructionKind::MovEaxImm32 { imm: 42 }
        );
    }

    #[test]
    fn decodes_mov_eax_imm32_then_ret() {
        let input = X86Bytes::new(X86Va::new(0x1000), vec![0xb8, 0x2a, 0, 0, 0, 0xc3])
            .expect("test bytes are non-empty");

        let decoded = decode_function(&input).expect("test bytes decode");

        assert_eq!(decoded.entry(), X86Va::new(0x1000));
        assert_eq!(
            decoded.instructions(),
            &[
                DecodedInstruction::new(
                    X86Va::new(0x1000),
                    X86Va::new(0x1005),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 }
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1005),
                    X86Va::new(0x1006),
                    DecodedInstructionKind::Ret
                )
            ]
        );
    }

    #[test]
    fn decodes_add_eax_imm8_between_mov_and_ret() {
        let input = X86Bytes::new(
            X86Va::new(0),
            vec![0xb8, 0x2a, 0, 0, 0, 0x83, 0xc0, 0x03, 0xc3],
        )
        .expect("test bytes are non-empty");

        let decoded = decode_function(&input).expect("test bytes decode");

        assert_eq!(
            decoded.instructions(),
            &[
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 }
                ),
                DecodedInstruction::new(
                    X86Va::new(5),
                    X86Va::new(8),
                    DecodedInstructionKind::AddEaxImm8 {
                        imm: crate::X86Imm8::new(3)
                    }
                ),
                DecodedInstruction::new(X86Va::new(8), X86Va::new(9), DecodedInstructionKind::Ret)
            ]
        );
    }

    #[test]
    fn truncated_mov_eax_imm32_is_reported() {
        let input =
            X86Bytes::new(X86Va::new(7), vec![0xb8, 0x2a]).expect("test bytes are non-empty");

        assert_eq!(
            decode_function(&input),
            Err(DecodeError::TruncatedInstruction {
                at: X86Va::new(7),
                opcode: 0xb8
            })
        );
    }

    #[test]
    fn unsupported_opcode_stops_decode_with_reason() {
        let input = X86Bytes::new(X86Va::new(0x20), vec![0x90]).expect("test bytes are non-empty");

        let decoded = decode_function(&input).expect("unsupported opcode decodes as instruction");

        assert_eq!(
            decoded.instructions(),
            &[DecodedInstruction::new(
                X86Va::new(0x20),
                X86Va::new(0x21),
                DecodedInstructionKind::Unsupported {
                    reason: UnsupportedReason::DecodeUnsupportedOpcode {
                        opcode: 0x90,
                        at: X86Va::new(0x20)
                    }
                }
            )]
        );
    }

    #[test]
    fn missing_ret_becomes_unsupported_instruction() {
        let input =
            X86Bytes::new(X86Va::new(0), vec![0xb8, 1, 0, 0, 0]).expect("test bytes are non-empty");

        let decoded = decode_function(&input).expect("missing ret is represented in decode");

        assert_eq!(
            decoded.instructions().last(),
            Some(&DecodedInstruction::new(
                X86Va::new(5),
                X86Va::new(6),
                DecodedInstructionKind::Unsupported {
                    reason: UnsupportedReason::MissingReturnTerminator { at: X86Va::new(5) }
                }
            ))
        );
    }
}
