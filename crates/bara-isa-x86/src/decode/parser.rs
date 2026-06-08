use bara_ir::{UnsupportedReason, X86Cond, X86Va};

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
            0x0f => {
                let opcode2 = read_u8(input, offset + 1, at, opcode)?;

                match opcode2 {
                    0x05 => {
                        let end_offset = offset + 2;
                        let end = instruction_end(input, at, end_offset, 2)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::Syscall,
                        ));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                    0x0b => {
                        let end_offset = offset + 2;
                        let end = instruction_end(input, at, end_offset, 2)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::BaraHostTrapSentinel,
                        ));
                        offset = end_offset;
                    }
                    0xb6 => {
                        let end_offset = offset + 3;
                        let operand = read_u8(input, offset + 2, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 3)?;

                        match operand {
                            0x07 => {
                                instructions.push(DecodedInstruction::new(
                                    at,
                                    end,
                                    DecodedInstructionKind::MovzxEaxBytePtrRdi,
                                ));
                                offset = end_offset;
                            }
                            _ => {
                                instructions.push(unsupported_instruction(at, end, opcode));
                                return DecodedFunction::new(input.entry(), instructions);
                            }
                        }
                    }
                    _ => {
                        let end = instruction_end(input, at, offset + 2, 2)?;
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
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
            0x3d => {
                let end_offset = offset + 5;
                let imm = read_i32(input, offset, end_offset, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 5)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::CmpEaxImm32 {
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
            0x74 | 0x75 => {
                let end_offset = offset + 2;
                let displacement = read_u8(input, offset + 1, at, opcode)?;
                let fallthrough = instruction_end(input, at, end_offset, 2)?;
                let taken = relative_target(
                    fallthrough,
                    i32::from(i8::from_le_bytes([displacement])),
                    at,
                )?;
                instructions.push(DecodedInstruction::new(
                    at,
                    fallthrough,
                    DecodedInstructionKind::JccRel8 {
                        condition: short_jcc_condition(opcode),
                        taken,
                        fallthrough,
                    },
                ));
                offset = end_offset;
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
                    0xf8 => {
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::CmpEaxImm8 {
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
            0x85 => {
                let end_offset = offset + 2;
                let operand = read_u8(input, offset + 1, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 2)?;

                match operand {
                    0xc0 => {
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::TestEaxEax,
                        ));
                        offset = end_offset;
                    }
                    _ => {
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0xe8 => {
                let end_offset = offset + 5;
                let displacement = read_i32(input, offset, end_offset, at, opcode)?;
                let return_to = instruction_end(input, at, end_offset, 5)?;
                let target = relative_target(return_to, displacement, at)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    return_to,
                    DecodedInstructionKind::CallRel32 { target, return_to },
                ));
                offset = end_offset;
            }
            0xc3 => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::Ret,
                ));
                offset += 1;
            }
            unsupported => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(unsupported_instruction(at, end, unsupported));
                return DecodedFunction::new(input.entry(), instructions);
            }
        }
    }

    if decoded_stream_ends_with_terminator(&instructions) {
        return DecodedFunction::new(input.entry(), instructions);
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

fn decoded_stream_ends_with_terminator(instructions: &[DecodedInstruction]) -> bool {
    let Some(instruction) = instructions.last() else {
        return false;
    };

    matches!(
        instruction.kind(),
        DecodedInstructionKind::CallRel32 { .. }
            | DecodedInstructionKind::JccRel8 { .. }
            | DecodedInstructionKind::Ret
            | DecodedInstructionKind::Syscall
    )
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

fn relative_target(return_to: X86Va, displacement: i32, at: X86Va) -> Result<X86Va, DecodeError> {
    let target = i128::from(return_to.value()) + i128::from(displacement);
    if target < 0 || target > i128::from(u64::MAX) {
        return Err(DecodeError::AddressOverflow {
            at,
            byte_len: u64::from(displacement.unsigned_abs()),
        });
    }

    Ok(X86Va::new(target as u64))
}

fn short_jcc_condition(opcode: u8) -> X86Cond {
    match opcode {
        0x74 => X86Cond::Equal,
        0x75 => X86Cond::NotEqual,
        _ => unreachable!("caller restricts short jcc opcodes"),
    }
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
