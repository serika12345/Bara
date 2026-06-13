use bara_ir::{UnsupportedReason, X86Cond, X86Va};

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
fn decodes_ret_then_trailing_block_bytes() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xb8, 1, 0, 0, 0, 0xc3, 0xb8, 2, 0, 0, 0, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0),
                X86Va::new(5),
                DecodedInstructionKind::MovEaxImm32 { imm: 1 }
            ),
            DecodedInstruction::new(X86Va::new(5), X86Va::new(6), DecodedInstructionKind::Ret),
            DecodedInstruction::new(
                X86Va::new(6),
                X86Va::new(11),
                DecodedInstructionKind::MovEaxImm32 { imm: 2 }
            ),
            DecodedInstruction::new(X86Va::new(11), X86Va::new(12), DecodedInstructionKind::Ret)
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
fn decodes_mov_rdx_rax_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x48, 0x89, 0xc2, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1003),
                DecodedInstructionKind::MovRdxRax
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
fn decodes_mov_rdi_rbx_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x48, 0x89, 0xdf, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1003),
                DecodedInstructionKind::MovRdiRbx
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
fn decodes_bara_appkit_gui_host_trap_sentinel_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x1000),
        vec![0x0f, 0x0b, b'B', b'8', b'G', b'1', 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1006),
                DecodedInstructionKind::BaraAppKitGuiHelloWorldTrapSentinel
            ),
            DecodedInstruction::new(
                X86Va::new(0x1006),
                X86Va::new(0x1007),
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
fn decodes_push_rax_pop_rax_between_mov_and_ret() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xb8, 0x2a, 0x00, 0x00, 0x00, 0x50, 0x58, 0xc3],
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
                X86Va::new(6),
                DecodedInstructionKind::PushRax
            ),
            DecodedInstruction::new(X86Va::new(6), X86Va::new(7), DecodedInstructionKind::PopRax),
            DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_add_rsp_imm8_before_next_unsupported_opcode() {
    let input = X86Bytes::new(X86Va::new(0x1700), vec![0x48, 0x83, 0xc4, 0x08, 0x41])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1700),
                X86Va::new(0x1704),
                DecodedInstructionKind::AddRspImm8 {
                    imm: crate::decode::X86Imm8::new(8)
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x1704),
                X86Va::new(0x1705),
                DecodedInstructionKind::Unsupported {
                    reason: UnsupportedReason::DecodeUnsupportedOpcode {
                        opcode: 0x41,
                        at: X86Va::new(0x1704)
                    }
                }
            )
        ]
    );
}

#[test]
fn decodes_pop_rbx_before_next_unsupported_rex_opcode() {
    let input = X86Bytes::new(X86Va::new(0x1800), vec![0x5b, 0x41, 0x40])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1800),
                X86Va::new(0x1801),
                DecodedInstructionKind::PopRbx
            ),
            DecodedInstruction::new(
                X86Va::new(0x1801),
                X86Va::new(0x1802),
                DecodedInstructionKind::Unsupported {
                    reason: UnsupportedReason::DecodeUnsupportedOpcode {
                        opcode: 0x41,
                        at: X86Va::new(0x1801)
                    }
                }
            )
        ]
    );
}

#[test]
fn decodes_pop_r14_before_next_unsupported_rex_opcode() {
    let input = X86Bytes::new(X86Va::new(0x1900), vec![0x41, 0x5e, 0x41, 0x5f])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1900),
                X86Va::new(0x1902),
                DecodedInstructionKind::PopR14
            ),
            DecodedInstruction::new(
                X86Va::new(0x1902),
                X86Va::new(0x1903),
                DecodedInstructionKind::Unsupported {
                    reason: UnsupportedReason::DecodeUnsupportedOpcode {
                        opcode: 0x41,
                        at: X86Va::new(0x1902)
                    }
                }
            )
        ]
    );
}

#[test]
fn decodes_prologue_and_rip_relative_load_batch_before_next_unsupported_opcode() {
    let input = X86Bytes::new(
        X86Va::new(0x1600),
        vec![
            0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x53, 0x48, 0x89, 0xc3, 0x48, 0x8b,
            0x05, 0xff, 0x19, 0x00, 0x00, 0x48, 0x8b, 0x10, 0x48, 0x8d, 0x3d, 0xb3, 0x10, 0x00,
            0x00, 0x48, 0x8d, 0x35, 0xb6, 0x10, 0x00, 0x00, 0xe8, 0x79, 0x00, 0x00, 0x00, 0x48,
            0x8b, 0x3d, 0x22, 0x3b, 0x00, 0x00, 0x48, 0x8b, 0x35, 0xeb, 0x3a, 0x00, 0x00, 0x4c,
            0x8b, 0x35, 0x14, 0x1a, 0x00, 0x00, 0x41, 0xff, 0xd6,
        ],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1600),
                X86Va::new(0x1601),
                DecodedInstructionKind::PushRbp
            ),
            DecodedInstruction::new(
                X86Va::new(0x1601),
                X86Va::new(0x1604),
                DecodedInstructionKind::MovRbpRsp
            ),
            DecodedInstruction::new(
                X86Va::new(0x1604),
                X86Va::new(0x1606),
                DecodedInstructionKind::PushR15
            ),
            DecodedInstruction::new(
                X86Va::new(0x1606),
                X86Va::new(0x1608),
                DecodedInstructionKind::PushR14
            ),
            DecodedInstruction::new(
                X86Va::new(0x1608),
                X86Va::new(0x1609),
                DecodedInstructionKind::PushRbx
            ),
            DecodedInstruction::new(
                X86Va::new(0x1609),
                X86Va::new(0x160c),
                DecodedInstructionKind::MovRbxRax
            ),
            DecodedInstruction::new(
                X86Va::new(0x160c),
                X86Va::new(0x1613),
                DecodedInstructionKind::MovRaxQwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(0x19ff),
                    address: X86Va::new(0x3012),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x1613),
                X86Va::new(0x1616),
                DecodedInstructionKind::MovRdxQwordPtrRax,
            ),
            DecodedInstruction::new(
                X86Va::new(0x1616),
                X86Va::new(0x161d),
                DecodedInstructionKind::LeaRdiRipRelative {
                    displacement: crate::decode::X86Imm32::new(0x10b3),
                    address: X86Va::new(0x26d0),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x161d),
                X86Va::new(0x1624),
                DecodedInstructionKind::LeaRsiRipRelative {
                    displacement: crate::decode::X86Imm32::new(0x10b6),
                    address: X86Va::new(0x26da),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x1624),
                X86Va::new(0x1629),
                DecodedInstructionKind::CallRel32 {
                    target: X86Va::new(0x16a2),
                    return_to: X86Va::new(0x1629),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x1629),
                X86Va::new(0x1630),
                DecodedInstructionKind::MovRdiQwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(0x3b22),
                    address: X86Va::new(0x5152),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x1630),
                X86Va::new(0x1637),
                DecodedInstructionKind::MovRsiQwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(0x3aeb),
                    address: X86Va::new(0x5122),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x1637),
                X86Va::new(0x163e),
                DecodedInstructionKind::MovR14QwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(0x1a14),
                    address: X86Va::new(0x3052),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x163e),
                X86Va::new(0x1641),
                DecodedInstructionKind::CallR14 {
                    return_to: X86Va::new(0x1641),
                }
            )
        ]
    );
}

#[test]
fn decodes_lea_rdi_rip_relative_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x2000),
        vec![0x48, 0x8d, 0x3d, 0xf9, 0xff, 0xff, 0xff, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2007),
                DecodedInstructionKind::LeaRdiRipRelative {
                    displacement: crate::decode::X86Imm32::new(-7),
                    address: X86Va::new(0x2000),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x2007),
                X86Va::new(0x2008),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_lea_rsi_rip_relative_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x2000),
        vec![0x48, 0x8d, 0x35, 0xf9, 0xff, 0xff, 0xff, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2007),
                DecodedInstructionKind::LeaRsiRipRelative {
                    displacement: crate::decode::X86Imm32::new(-7),
                    address: X86Va::new(0x2000),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x2007),
                X86Va::new(0x2008),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn truncated_lea_rdi_rip_relative_is_reported() {
    let input = X86Bytes::new(X86Va::new(0x1616), vec![0x48, 0x8d, 0x3d])
        .expect("test bytes are non-empty");

    assert_eq!(
        decode_function(&input),
        Err(DecodeError::TruncatedInstruction {
            at: X86Va::new(0x1616),
            opcode: 0x48
        })
    );
}

#[test]
fn truncated_lea_rsi_rip_relative_is_reported() {
    let input = X86Bytes::new(X86Va::new(0x161d), vec![0x48, 0x8d, 0x35])
        .expect("test bytes are non-empty");

    assert_eq!(
        decode_function(&input),
        Err(DecodeError::TruncatedInstruction {
            at: X86Va::new(0x161d),
            opcode: 0x48
        })
    );
}

#[test]
fn decodes_rex_lea_unsupported_when_destination_operand_does_not_match() {
    let input = X86Bytes::new(X86Va::new(0x161d), vec![0x48, 0x8d, 0x34])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("unsupported opcode decodes as instruction");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x161d),
            X86Va::new(0x1620),
            DecodedInstructionKind::Unsupported {
                reason: UnsupportedReason::DecodeUnsupportedOpcode {
                    opcode: 0x48,
                    at: X86Va::new(0x161d),
                }
            }
        )]
    );
}

#[test]
fn decodes_mov_rdx_qword_ptr_rax_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x2000), vec![0x48, 0x8b, 0x10, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2003),
                DecodedInstructionKind::MovRdxQwordPtrRax
            ),
            DecodedInstruction::new(
                X86Va::new(0x2003),
                X86Va::new(0x2004),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_mov_rax_qword_ptr_rip_relative_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x2000),
        vec![0x48, 0x8b, 0x05, 0xf9, 0xff, 0xff, 0xff, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2007),
                DecodedInstructionKind::MovRaxQwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(-7),
                    address: X86Va::new(0x2000),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x2007),
                X86Va::new(0x2008),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_mov_rdi_qword_ptr_rip_relative_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x2000),
        vec![0x48, 0x8b, 0x3d, 0xf9, 0xff, 0xff, 0xff, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2007),
                DecodedInstructionKind::MovRdiQwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(-7),
                    address: X86Va::new(0x2000),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x2007),
                X86Va::new(0x2008),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_mov_rsi_qword_ptr_rip_relative_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x2000),
        vec![0x48, 0x8b, 0x35, 0xf9, 0xff, 0xff, 0xff, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2007),
                DecodedInstructionKind::MovRsiQwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(-7),
                    address: X86Va::new(0x2000),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x2007),
                X86Va::new(0x2008),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_mov_r14_qword_ptr_rip_relative_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x2000),
        vec![0x4c, 0x8b, 0x35, 0xf9, 0xff, 0xff, 0xff, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2007),
                DecodedInstructionKind::MovR14QwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(-7),
                    address: X86Va::new(0x2000),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x2007),
                X86Va::new(0x2008),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_mov_r15_qword_ptr_rip_relative_then_ret() {
    let input = X86Bytes::new(
        X86Va::new(0x2000),
        vec![0x4c, 0x8b, 0x3d, 0xf9, 0xff, 0xff, 0xff, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2007),
                DecodedInstructionKind::MovR15QwordPtrRipRelative {
                    displacement: crate::decode::X86Imm32::new(-7),
                    address: X86Va::new(0x2000),
                }
            ),
            DecodedInstruction::new(
                X86Va::new(0x2007),
                X86Va::new(0x2008),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_mov_rdi_qword_ptr_r15_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x2000), vec![0x49, 0x8b, 0x3f, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x2000),
                X86Va::new(0x2003),
                DecodedInstructionKind::MovRdiQwordPtrR15
            ),
            DecodedInstruction::new(
                X86Va::new(0x2003),
                X86Va::new(0x2004),
                DecodedInstructionKind::Ret
            )
        ]
    );
}

#[test]
fn decodes_call_r14_as_indirect_call_terminator() {
    let input = X86Bytes::new(X86Va::new(0x2000), vec![0x41, 0xff, 0xd6, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x2000),
            X86Va::new(0x2003),
            DecodedInstructionKind::CallR14 {
                return_to: X86Va::new(0x2003),
            }
        )]
    );
}

#[test]
fn truncated_mov_rax_qword_ptr_rip_relative_is_reported() {
    let input = X86Bytes::new(X86Va::new(0x160c), vec![0x48, 0x8b, 0x05])
        .expect("test bytes are non-empty");

    assert_eq!(
        decode_function(&input),
        Err(DecodeError::TruncatedInstruction {
            at: X86Va::new(0x160c),
            opcode: 0x48
        })
    );
}

#[test]
fn truncated_mov_rdi_qword_ptr_rip_relative_is_reported() {
    let input = X86Bytes::new(X86Va::new(0x162f), vec![0x48, 0x8b, 0x3d])
        .expect("test bytes are non-empty");

    assert_eq!(
        decode_function(&input),
        Err(DecodeError::TruncatedInstruction {
            at: X86Va::new(0x162f),
            opcode: 0x48
        })
    );
}

#[test]
fn truncated_mov_rsi_qword_ptr_rip_relative_is_reported() {
    let input = X86Bytes::new(X86Va::new(0x1630), vec![0x48, 0x8b, 0x35])
        .expect("test bytes are non-empty");

    assert_eq!(
        decode_function(&input),
        Err(DecodeError::TruncatedInstruction {
            at: X86Va::new(0x1630),
            opcode: 0x48
        })
    );
}

#[test]
fn truncated_mov_r14_qword_ptr_rip_relative_is_reported() {
    let input = X86Bytes::new(X86Va::new(0x1637), vec![0x4c, 0x8b, 0x35])
        .expect("test bytes are non-empty");

    assert_eq!(
        decode_function(&input),
        Err(DecodeError::TruncatedInstruction {
            at: X86Va::new(0x1637),
            opcode: 0x4c
        })
    );
}

#[test]
fn truncated_call_r14_is_reported() {
    let input =
        X86Bytes::new(X86Va::new(0x163e), vec![0x41, 0xff]).expect("test bytes are non-empty");

    assert_eq!(
        decode_function(&input),
        Err(DecodeError::TruncatedInstruction {
            at: X86Va::new(0x163e),
            opcode: 0x41
        })
    );
}

#[test]
fn decodes_rex_mov_unsupported_when_rip_relative_operand_does_not_match() {
    let input = X86Bytes::new(X86Va::new(0x160c), vec![0x48, 0x8b, 0x11, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("unsupported opcode decodes as instruction");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x160c),
            X86Va::new(0x160f),
            DecodedInstructionKind::Unsupported {
                reason: UnsupportedReason::DecodeUnsupportedOpcode {
                    opcode: 0x48,
                    at: X86Va::new(0x160c),
                }
            }
        )]
    );
}

#[test]
fn decodes_rex_r_mov_unsupported_when_rip_relative_operand_does_not_match() {
    let input = X86Bytes::new(X86Va::new(0x1637), vec![0x4c, 0x8b, 0x34])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("unsupported opcode decodes as instruction");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x1637),
            X86Va::new(0x163a),
            DecodedInstructionKind::Unsupported {
                reason: UnsupportedReason::DecodeUnsupportedOpcode {
                    opcode: 0x4c,
                    at: X86Va::new(0x1637),
                }
            }
        )]
    );
}

#[test]
fn decodes_rex_b_call_unsupported_when_indirect_target_does_not_match() {
    let input = X86Bytes::new(X86Va::new(0x163e), vec![0x41, 0xff, 0xd7])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("unsupported opcode decodes as instruction");

    assert_eq!(
        decoded.instructions(),
        &[DecodedInstruction::new(
            X86Va::new(0x163e),
            X86Va::new(0x1641),
            DecodedInstructionKind::Unsupported {
                reason: UnsupportedReason::DecodeUnsupportedOpcode {
                    opcode: 0x41,
                    at: X86Va::new(0x163e),
                }
            }
        )]
    );
}

#[test]
fn decodes_je_rel8_and_continues_with_fallthrough() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x74, 0x02, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1002),
                DecodedInstructionKind::JccRel8 {
                    condition: X86Cond::Equal,
                    taken: X86Va::new(0x1004),
                    fallthrough: X86Va::new(0x1002)
                }
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
fn decodes_jne_rel8_with_negative_target() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x75, 0xfe, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1002),
                DecodedInstructionKind::JccRel8 {
                    condition: X86Cond::NotEqual,
                    taken: X86Va::new(0x1000),
                    fallthrough: X86Va::new(0x1002)
                }
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
fn decodes_jo_rel8_to_overflow_condition() {
    let input =
        X86Bytes::new(X86Va::new(0), vec![0x70, 0x01, 0xc3]).expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions()[0].kind(),
        &DecodedInstructionKind::JccRel8 {
            condition: X86Cond::Overflow,
            taken: X86Va::new(3),
            fallthrough: X86Va::new(2)
        }
    );
}

#[test]
fn decodes_jl_rel32_and_continues_with_target_block() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![
            0x0f, 0x8c, 0x01, 0x00, 0x00, 0x00, 0xc3, 0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3,
        ],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0),
                X86Va::new(6),
                DecodedInstructionKind::JccRel32 {
                    condition: X86Cond::Less,
                    taken: X86Va::new(7),
                    fallthrough: X86Va::new(6)
                }
            ),
            DecodedInstruction::new(X86Va::new(6), X86Va::new(7), DecodedInstructionKind::Ret),
            DecodedInstruction::new(
                X86Va::new(7),
                X86Va::new(12),
                DecodedInstructionKind::MovEaxImm32 { imm: 42 }
            ),
            DecodedInstruction::new(X86Va::new(12), X86Va::new(13), DecodedInstructionKind::Ret)
        ]
    );
}

#[test]
fn decodes_jmp_rel8_and_continues_with_target_block() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![
            0xeb, 0x06, 0xb8, 0x07, 0x00, 0x00, 0x00, 0xc3, 0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3,
        ],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0),
                X86Va::new(2),
                DecodedInstructionKind::JmpRel8 {
                    target: X86Va::new(8)
                }
            ),
            DecodedInstruction::new(
                X86Va::new(2),
                X86Va::new(7),
                DecodedInstructionKind::MovEaxImm32 { imm: 7 }
            ),
            DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            DecodedInstruction::new(
                X86Va::new(8),
                X86Va::new(13),
                DecodedInstructionKind::MovEaxImm32 { imm: 42 }
            ),
            DecodedInstruction::new(X86Va::new(13), X86Va::new(14), DecodedInstructionKind::Ret)
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
fn decodes_xor_edx_edx_then_ret() {
    let input = X86Bytes::new(X86Va::new(0x1000), vec![0x31, 0xd2, 0xc3])
        .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1002),
                DecodedInstructionKind::XorEdxEdx
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
fn decodes_call_rel32_then_fallthrough_instruction() {
    let input = X86Bytes::new(
        X86Va::new(0),
        vec![0xe8, 1, 0, 0, 0, 0xb8, 2, 0, 0, 0, 0xc3],
    )
    .expect("test bytes are non-empty");

    let decoded = decode_function(&input).expect("test bytes decode");

    assert_eq!(
        decoded.instructions(),
        &[
            DecodedInstruction::new(
                X86Va::new(0),
                X86Va::new(5),
                DecodedInstructionKind::CallRel32 {
                    target: X86Va::new(6),
                    return_to: X86Va::new(5)
                }
            ),
            DecodedInstruction::new(
                X86Va::new(5),
                X86Va::new(10),
                DecodedInstructionKind::MovEaxImm32 { imm: 2 }
            ),
            DecodedInstruction::new(X86Va::new(10), X86Va::new(11), DecodedInstructionKind::Ret)
        ]
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
