use bara_ir::{UnsupportedReason, X86Va};

use super::{
    DecodeError, DecodedFunction, DecodedInstruction, DecodedInstructionKind, X86Bytes, X86Imm32,
    X86Imm8,
};

pub(super) fn parse_function(input: &X86Bytes) -> Result<DecodedFunction, DecodeError> {
    let mut offset = 0usize;
    let mut instructions = Vec::new();

    while offset < input.bytes().len() {
        let at = address_at(input, offset)?;
        let opcode = input.bytes()[offset];

        match opcode {
            0x05 => {
                let end_offset = offset + 5;
                let imm = read_i32(input, offset, end_offset, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 5)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::AddEaxImm32 {
                        imm: X86Imm32::new(imm),
                    },
                ));
                offset = end_offset;
            }
            0x2d => {
                let end_offset = offset + 5;
                let imm = read_i32(input, offset, end_offset, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 5)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::SubEaxImm32 {
                        imm: X86Imm32::new(imm),
                    },
                ));
                offset = end_offset;
            }
            0x31 => {
                let end_offset = offset + 2;
                let operand = read_u8(input, offset + 1, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 2)?;

                match operand {
                    0xc0 => {
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::XorEaxEax,
                        ));
                        offset = end_offset;
                    }
                    _ => {
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0x48 => {
                let end_offset = offset + 3;
                let opcode2 = read_u8(input, offset + 1, at, opcode)?;
                let operand = read_u8(input, offset + 2, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 3)?;

                match (opcode2, operand) {
                    (0x89, 0xf8) => {
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRaxRdi,
                        ));
                        offset = end_offset;
                    }
                    _ => {
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0xb8 => {
                let end_offset = offset + 5;
                let imm = read_u32(input, offset, end_offset, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 5)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::MovEaxImm32 { imm },
                ));
                offset = end_offset;
            }
            0x83 => {
                let end_offset = offset + 3;
                let operand = read_u8(input, offset + 1, at, opcode)?;
                let imm = read_u8(input, offset + 2, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 3)?;

                match operand {
                    0xc0 => {
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::AddEaxImm8 {
                                imm: X86Imm8::new(i8::from_le_bytes([imm])),
                            },
                        ));
                        offset = end_offset;
                    }
                    0xe8 => {
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::SubEaxImm8 {
                                imm: X86Imm8::new(i8::from_le_bytes([imm])),
                            },
                        ));
                        offset = end_offset;
                    }
                    _ => {
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0xc3 => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::Ret,
                ));
                return DecodedFunction::new(input.entry(), instructions);
            }
            unsupported => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(unsupported_instruction(at, end, unsupported));
                return DecodedFunction::new(input.entry(), instructions);
            }
        }
    }

    let at = address_at(input, input.bytes().len())?;
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

fn address_at(input: &X86Bytes, offset: usize) -> Result<X86Va, DecodeError> {
    input
        .entry()
        .checked_add(offset as u64)
        .map_err(|_| DecodeError::AddressOverflow {
            at: input.entry(),
            byte_len: offset as u64,
        })
}

fn instruction_end(
    input: &X86Bytes,
    at: X86Va,
    end_offset: usize,
    byte_len: u64,
) -> Result<X86Va, DecodeError> {
    input
        .entry()
        .checked_add(end_offset as u64)
        .map_err(|_| DecodeError::AddressOverflow { at, byte_len })
}

fn read_u8(input: &X86Bytes, offset: usize, at: X86Va, opcode: u8) -> Result<u8, DecodeError> {
    input
        .bytes()
        .get(offset)
        .copied()
        .ok_or(DecodeError::TruncatedInstruction { at, opcode })
}

fn read_i32(
    input: &X86Bytes,
    offset: usize,
    end_offset: usize,
    at: X86Va,
    opcode: u8,
) -> Result<i32, DecodeError> {
    let imm_bytes = input
        .bytes()
        .get((offset + 1)..end_offset)
        .ok_or(DecodeError::TruncatedInstruction { at, opcode })?;

    Ok(i32::from_le_bytes([
        imm_bytes[0],
        imm_bytes[1],
        imm_bytes[2],
        imm_bytes[3],
    ]))
}

fn read_u32(
    input: &X86Bytes,
    offset: usize,
    end_offset: usize,
    at: X86Va,
    opcode: u8,
) -> Result<u32, DecodeError> {
    let imm_bytes = input
        .bytes()
        .get((offset + 1)..end_offset)
        .ok_or(DecodeError::TruncatedInstruction { at, opcode })?;

    Ok(u32::from_le_bytes([
        imm_bytes[0],
        imm_bytes[1],
        imm_bytes[2],
        imm_bytes[3],
    ]))
}

fn unsupported_instruction(at: X86Va, end: X86Va, opcode: u8) -> DecodedInstruction {
    DecodedInstruction::new(
        at,
        end,
        DecodedInstructionKind::Unsupported {
            reason: UnsupportedReason::DecodeUnsupportedOpcode { opcode, at },
        },
    )
}
