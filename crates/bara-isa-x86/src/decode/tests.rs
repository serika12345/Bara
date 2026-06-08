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
    let bytes = X86Bytes::new(X86Va::new(0x1000), vec![0xc3]).expect("test bytes are non-empty");

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
fn decodes_mov_rax_rdi_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x48, 0x89, 0xf8, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1003),
                DecodedInstructionKind::MovRaxRdi
            ),
            DecodedInstruction::new(
                X86Va::new(0x1003),
                X86Va::new(0x1004),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_movzx_eax_byte_ptr_rdi_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x0f, 0xb6, 0x07, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1003),
                DecodedInstructionKind::MovzxEaxBytePtrRdi
            ),
            DecodedInstruction::new(
                X86Va::new(0x1003),
                X86Va::new(0x1004),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_bara_host_trap_sentinel_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x0f, 0x0b, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1002),
                DecodedInstructionKind::BaraHostTrapSentinel
            ),
            DecodedInstruction::new(
                X86Va::new(0x1002),
                X86Va::new(0x1003),
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
fn decodes_add_eax_imm32_between_mov_and_ret() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xb8, 0x2a, 0, 0, 0, 0x05, 0x03, 0, 0, 0, 0xc3],
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
                X86Va::new(10),
                DecodedInstructionKind::AddEaxImm32 {
                    imm: crate::decode::X86Imm32::new(3)
                }
            ),
            DecodedInstruction::new(X86Va::new(10), X86Va::new(11), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_sub_eax_imm8_between_mov_and_ret() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xb8, 0x2a, 0, 0, 0, 0x83, 0xe8, 0x03, 0xc3],
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
                DecodedInstructionKind::SubEaxImm8 {
                    imm: crate::X86Imm8::new(3)
                }
            ),
            DecodedInstruction::new(X86Va::new(8), X86Va::new(9), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_sub_eax_imm32_between_mov_and_ret() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xb8, 0x2a, 0, 0, 0, 0x2d, 0x03, 0, 0, 0, 0xc3],
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
                X86Va::new(10),
                DecodedInstructionKind::SubEaxImm32 {
                    imm: crate::decode::X86Imm32::new(3)
                }
            ),
            DecodedInstruction::new(X86Va::new(10), X86Va::new(11), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_cmp_eax_imm8_between_mov_and_ret() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xb8, 0x2a, 0, 0, 0, 0x83, 0xf8, 0x2a, 0xc3],
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
                DecodedInstructionKind::CmpEaxImm8 {
                    imm: crate::X86Imm8::new(42)
                }
            ),
            DecodedInstruction::new(X86Va::new(8), X86Va::new(9), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_cmp_eax_imm32_between_mov_and_ret() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xb8, 0x2a, 0, 0, 0, 0x3d, 0x2a, 0, 0, 0, 0xc3],
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
                X86Va::new(10),
                DecodedInstructionKind::CmpEaxImm32 {
                    imm: crate::decode::X86Imm32::new(42)
                }
            ),
            DecodedInstruction::new(X86Va::new(10), X86Va::new(11), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_test_eax_eax_between_mov_and_ret() {
    let input = X86Bytes::new(X86Va::new(0), vec![0xb8, 0x2a, 0, 0, 0, 0x85, 0xc0, 0xc3])
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
                X86Va::new(7),
                DecodedInstructionKind::TestEaxEax
            ),
            DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_xor_eax_eax_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x31, 0xc0, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1002),
                DecodedInstructionKind::XorEaxEax
            ),
            DecodedInstruction::new(
                X86Va::new(0x1002),
                X86Va::new(0x1003),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_call_rel32_with_positive_target() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0xe8, 0x10, 0, 0, 0])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x1000),
            X86Va::new(0x1005),
            DecodedInstructionKind::CallRel32 {
                target: X86Va::new(0x1015),
                return_to: X86Va::new(0x1005)
            }
        )]
    );
}

#[test]
fn decodes_call_rel32_with_negative_target() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0xe8, 0xf0, 0xff, 0xff, 0xff])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x1000),
            X86Va::new(0x1005),
            DecodedInstructionKind::CallRel32 {
                target: X86Va::new(0x0ff5),
                return_to: X86Va::new(0x1005)
            }
        )]
    );
}

#[test]
fn decodes_syscall_boundary() {
    let input =
        X86Bytes::new(X86Va::new(0x1000), vec![0x0f, 0x05]).expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x1000),
            X86Va::new(0x1002),
            DecodedInstructionKind::Syscall
        )]
    );
}

#[test]
fn truncated_mov_eax_imm32_is_reported() {
    let input = X86Bytes::new(X86Va::new(7), vec![0xb8, 0x2a]).expect("test bytes are non-empty");

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
