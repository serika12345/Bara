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
                    0x80..=0x8f => {
                        let end_offset = offset + 6;
                        let displacement = read_i32_at(input, offset + 2, at, opcode)?;
                        let fallthrough = instruction_end(input, at, end_offset, 6)?;
                        let taken = relative_target(fallthrough, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            fallthrough,
                            DecodedInstructionKind::JccRel32 {
                                condition: jcc_condition(opcode2),
                                taken,
                                fallthrough,
                            },
                        ));
                        offset = end_offset;
                    }
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
                        let (end_offset, instruction_len, kind) =
                            if has_b8_g1_host_trap_tag(input.bytes(), offset) {
                                (
                                    offset + 6,
                                    6,
                                    DecodedInstructionKind::BaraAppKitGuiHelloWorldTrapSentinel,
                                )
                            } else {
                                (offset + 2, 2, DecodedInstructionKind::BaraHostTrapSentinel)
                            };
                        let end = instruction_end(input, at, end_offset, instruction_len)?;
                        instructions.push(DecodedInstruction::new(at, end, kind));
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
            0x41 => {
                let Some(opcode2) = input.bytes().get(offset + 1).copied() else {
                    let end = instruction_end(input, at, offset + 1, 1)?;
                    instructions.push(unsupported_instruction(at, end, opcode));
                    return DecodedFunction::new(input.entry(), instructions);
                };

                match opcode2 {
                    0x56 => {
                        let end_offset = offset + 2;
                        let end = instruction_end(input, at, end_offset, 2)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::PushR14,
                        ));
                        offset = end_offset;
                    }
                    0x57 => {
                        let end_offset = offset + 2;
                        let end = instruction_end(input, at, end_offset, 2)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::PushR15,
                        ));
                        offset = end_offset;
                    }
                    0xff => {
                        let operand = read_u8(input, offset + 2, at, opcode)?;
                        let end_offset = offset + 3;
                        let end = instruction_end(input, at, end_offset, 3)?;

                        match operand {
                            0xd6 => {
                                instructions.push(DecodedInstruction::new(
                                    at,
                                    end,
                                    DecodedInstructionKind::CallR14 { return_to: end },
                                ));
                                return DecodedFunction::new(input.entry(), instructions);
                            }
                            _ => {
                                instructions.push(unsupported_instruction(at, end, opcode));
                                return DecodedFunction::new(input.entry(), instructions);
                            }
                        }
                    }
                    _ => {
                        let end = instruction_end(input, at, offset + 1, 1)?;
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0x48 => {
                let opcode2 = read_u8(input, offset + 1, at, opcode)?;
                let operand = read_u8(input, offset + 2, at, opcode)?;

                match (opcode2, operand) {
                    (0x8b, 0x10) => {
                        let end_offset = offset + 3;
                        let end = instruction_end(input, at, end_offset, 3)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRdxQwordPtrRax,
                        ));
                        offset = end_offset;
                    }
                    (0x8b, 0x05) => {
                        let end_offset = offset + 7;
                        let displacement = read_i32_at(input, offset + 3, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 7)?;
                        let address = relative_target(end, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRaxQwordPtrRipRelative {
                                displacement: X86Imm32::new(displacement),
                                address,
                            },
                        ));
                        offset = end_offset;
                    }
                    (0x8b, 0x3d) => {
                        let end_offset = offset + 7;
                        let displacement = read_i32_at(input, offset + 3, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 7)?;
                        let address = relative_target(end, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRdiQwordPtrRipRelative {
                                displacement: X86Imm32::new(displacement),
                                address,
                            },
                        ));
                        offset = end_offset;
                    }
                    (0x8b, 0x35) => {
                        let end_offset = offset + 7;
                        let displacement = read_i32_at(input, offset + 3, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 7)?;
                        let address = relative_target(end, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRsiQwordPtrRipRelative {
                                displacement: X86Imm32::new(displacement),
                                address,
                            },
                        ));
                        offset = end_offset;
                    }
                    (0x8d, 0x3d) => {
                        let end_offset = offset + 7;
                        let displacement = read_i32_at(input, offset + 3, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 7)?;
                        let address = relative_target(end, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::LeaRdiRipRelative {
                                displacement: X86Imm32::new(displacement),
                                address,
                            },
                        ));
                        offset = end_offset;
                    }
                    (0x8d, 0x35) => {
                        let end_offset = offset + 7;
                        let displacement = read_i32_at(input, offset + 3, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 7)?;
                        let address = relative_target(end, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::LeaRsiRipRelative {
                                displacement: X86Imm32::new(displacement),
                                address,
                            },
                        ));
                        offset = end_offset;
                    }
                    (0x89, 0xf8) => {
                        let end_offset = offset + 3;
                        let end = instruction_end(input, at, end_offset, 3)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRaxRdi,
                        ));
                        offset = end_offset;
                    }
                    (0x89, 0xc3) => {
                        let end_offset = offset + 3;
                        let end = instruction_end(input, at, end_offset, 3)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRbxRax,
                        ));
                        offset = end_offset;
                    }
                    (0x89, 0xe5) => {
                        let end_offset = offset + 3;
                        let end = instruction_end(input, at, end_offset, 3)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRbpRsp,
                        ));
                        offset = end_offset;
                    }
                    _ => {
                        let end = instruction_end(input, at, offset + 3, 3)?;
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0x49 => {
                let opcode2 = read_u8(input, offset + 1, at, opcode)?;
                let operand = read_u8(input, offset + 2, at, opcode)?;

                match (opcode2, operand) {
                    (0x8b, 0x3f) => {
                        let end_offset = offset + 3;
                        let end = instruction_end(input, at, end_offset, 3)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovRdiQwordPtrR15,
                        ));
                        offset = end_offset;
                    }
                    _ => {
                        let end = instruction_end(input, at, offset + 3, 3)?;
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0x4c => {
                let opcode2 = read_u8(input, offset + 1, at, opcode)?;
                let operand = read_u8(input, offset + 2, at, opcode)?;

                match (opcode2, operand) {
                    (0x8b, 0x35) => {
                        let end_offset = offset + 7;
                        let displacement = read_i32_at(input, offset + 3, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 7)?;
                        let address = relative_target(end, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovR14QwordPtrRipRelative {
                                displacement: X86Imm32::new(displacement),
                                address,
                            },
                        ));
                        offset = end_offset;
                    }
                    (0x8b, 0x3d) => {
                        let end_offset = offset + 7;
                        let displacement = read_i32_at(input, offset + 3, at, opcode)?;
                        let end = instruction_end(input, at, end_offset, 7)?;
                        let address = relative_target(end, displacement, at)?;
                        instructions.push(DecodedInstruction::new(
                            at,
                            end,
                            DecodedInstructionKind::MovR15QwordPtrRipRelative {
                                displacement: X86Imm32::new(displacement),
                                address,
                            },
                        ));
                        offset = end_offset;
                    }
                    _ => {
                        let end = instruction_end(input, at, offset + 3, 3)?;
                        instructions.push(unsupported_instruction(at, end, opcode));
                        return DecodedFunction::new(input.entry(), instructions);
                    }
                }
            }
            0x50 => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::PushRax,
                ));
                offset += 1;
            }
            0x53 => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::PushRbx,
                ));
                offset += 1;
            }
            0x55 => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::PushRbp,
                ));
                offset += 1;
            }
            0x58 => {
                let end = instruction_end(input, at, offset + 1, 1)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::PopRax,
                ));
                offset += 1;
            }
            0x70..=0x7f => {
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
                        condition: jcc_condition(opcode),
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
            0xeb => {
                let end_offset = offset + 2;
                let displacement = read_u8(input, offset + 1, at, opcode)?;
                let end = instruction_end(input, at, end_offset, 2)?;
                let target =
                    relative_target(end, i32::from(i8::from_le_bytes([displacement])), at)?;
                instructions.push(DecodedInstruction::new(
                    at,
                    end,
                    DecodedInstructionKind::JmpRel8 { target },
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

fn has_b8_g1_host_trap_tag(bytes: &[u8], offset: usize) -> bool {
    bytes
        .get(offset + 2..offset + 6)
        .is_some_and(|tag| tag == b"B8G1")
}

fn decoded_stream_ends_with_terminator(instructions: &[DecodedInstruction]) -> bool {
    let Some(instruction) = instructions.last() else {
        return false;
    };

    matches!(
        instruction.kind(),
        DecodedInstructionKind::CallRel32 { .. }
            | DecodedInstructionKind::CallR14 { .. }
            | DecodedInstructionKind::JccRel8 { .. }
            | DecodedInstructionKind::JccRel32 { .. }
            | DecodedInstructionKind::JmpRel8 { .. }
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

fn read_i32_at(
    input: &X86Bytes,
    start_offset: usize,
    at: X86Va,
    opcode: u8,
) -> Result<i32, DecodeError> {
    let end_offset = start_offset
        .checked_add(4)
        .ok_or(DecodeError::TruncatedInstruction { at, opcode })?;
    let imm_bytes = input
        .bytes()
        .get(start_offset..end_offset)
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

fn jcc_condition(opcode: u8) -> X86Cond {
    match opcode & 0x0f {
        0x0 => X86Cond::Overflow,
        0x1 => X86Cond::NotOverflow,
        0x2 => X86Cond::Below,
        0x3 => X86Cond::AboveOrEqual,
        0x4 => X86Cond::Equal,
        0x5 => X86Cond::NotEqual,
        0x6 => X86Cond::BelowOrEqual,
        0x7 => X86Cond::Above,
        0x8 => X86Cond::Sign,
        0x9 => X86Cond::NotSign,
        0xa => X86Cond::Parity,
        0xb => X86Cond::NotParity,
        0xc => X86Cond::Less,
        0xd => X86Cond::GreaterOrEqual,
        0xe => X86Cond::LessOrEqual,
        0xf => X86Cond::Greater,
        _ => unreachable!("low nibble is restricted to jcc conditions"),
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
