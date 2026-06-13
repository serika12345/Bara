use bara_ir::{
    BasicBlock, BasicBlockError, BlockId, BoundaryRequest, HostTrapKind, IrOp, MemoryReadWidth,
    Operand, Program, ProgramError, ProgramImageMetadata, SyscallAbi, SyscallRequest,
    SyscallRequestError, Terminator, UnsupportedReason, X86Reg,
};

use crate::{DecodeError, DecodedFunction, DecodedInstruction, DecodedInstructionKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiftError {
    EmptyDecodedFunction,
    BasicBlock(BasicBlockError),
    Program(ProgramError),
    Decode(DecodeError),
    SyscallRequest(SyscallRequestError),
}

pub fn lift_decoded_function(decoded: &DecodedFunction) -> Result<Program, LiftError> {
    lift_decoded_function_with_image_metadata(decoded, ProgramImageMetadata::empty())
}

pub fn lift_decoded_function_with_image_metadata(
    decoded: &DecodedFunction,
    image_metadata: ProgramImageMetadata,
) -> Result<Program, LiftError> {
    let mut blocks = Vec::new();
    let mut ops = Vec::new();
    let mut block_start = Some(decoded.entry());
    let mut block_end = decoded.entry();
    let instruction_starts = decoded
        .instructions()
        .iter()
        .map(DecodedInstruction::start)
        .collect::<Vec<_>>();

    for instruction in decoded.instructions() {
        let start = block_start.get_or_insert(instruction.start());
        match lift_instruction(instruction, &instruction_starts)? {
            LiftedInstruction::Op(op) => {
                block_end = instruction.end();
                ops.push(op);
            }
            LiftedInstruction::Terminator(terminator) => {
                let block = BasicBlock::new(
                    BlockId::new(blocks.len() as u32),
                    *start,
                    instruction.end(),
                    ops,
                    terminator,
                )
                .map_err(LiftError::BasicBlock)?;
                blocks.push(block);
                ops = Vec::new();
                block_start = None;
            }
        }
    }

    if !ops.is_empty() {
        let start = block_start.ok_or(LiftError::EmptyDecodedFunction)?;
        let block = BasicBlock::new(
            BlockId::new(blocks.len() as u32),
            start,
            block_end,
            ops,
            Terminator::Unsupported {
                reason: UnsupportedReason::MissingReturnTerminator { at: block_end },
            },
        )
        .map_err(LiftError::BasicBlock)?;
        blocks.push(block);
    }

    if blocks.is_empty() {
        return Err(LiftError::EmptyDecodedFunction);
    }

    Program::with_image_metadata(decoded.entry(), blocks, image_metadata)
        .map_err(LiftError::Program)
}

enum LiftedInstruction {
    Op(IrOp),
    Terminator(Terminator),
}

fn lift_instruction(
    instruction: &DecodedInstruction,
    instruction_starts: &[bara_ir::X86Va],
) -> Result<LiftedInstruction, LiftError> {
    match instruction.kind() {
        DecodedInstructionKind::MovEaxImm32 { imm } => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(u64::from(*imm)),
        })),
        DecodedInstructionKind::MovRaxRdi => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::Reg(X86Reg::Rdi),
        })),
        DecodedInstructionKind::MovRbxRax => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rbx),
            src: Operand::Reg(X86Reg::Rax),
        })),
        DecodedInstructionKind::MovRdxRax => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rdx),
            src: Operand::Reg(X86Reg::Rax),
        })),
        DecodedInstructionKind::MovRaxQwordPtrRipRelative { address, .. } => {
            Ok(LiftedInstruction::Op(IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::MemRipRelative {
                    address: *address,
                    width: MemoryReadWidth::Bits64,
                },
            }))
        }
        DecodedInstructionKind::MovRdiQwordPtrRipRelative { address, .. } => {
            Ok(LiftedInstruction::Op(IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdi),
                src: Operand::MemRipRelative {
                    address: *address,
                    width: MemoryReadWidth::Bits64,
                },
            }))
        }
        DecodedInstructionKind::MovRsiQwordPtrRipRelative { address, .. } => {
            Ok(LiftedInstruction::Op(IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rsi),
                src: Operand::MemRipRelative {
                    address: *address,
                    width: MemoryReadWidth::Bits64,
                },
            }))
        }
        DecodedInstructionKind::MovR14QwordPtrRipRelative { address, .. } => {
            Ok(LiftedInstruction::Op(IrOp::Mov {
                dst: Operand::Reg(X86Reg::R14),
                src: Operand::MemRipRelative {
                    address: *address,
                    width: MemoryReadWidth::Bits64,
                },
            }))
        }
        DecodedInstructionKind::MovR15QwordPtrRipRelative { address, .. } => {
            Ok(LiftedInstruction::Op(IrOp::Mov {
                dst: Operand::Reg(X86Reg::R15),
                src: Operand::MemRipRelative {
                    address: *address,
                    width: MemoryReadWidth::Bits64,
                },
            }))
        }
        DecodedInstructionKind::MovRdiQwordPtrR15 => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rdi),
            src: Operand::MemRegIndirect {
                base: X86Reg::R15,
                width: MemoryReadWidth::Bits64,
            },
        })),
        DecodedInstructionKind::MovRdxQwordPtrRax => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rdx),
            src: Operand::MemRegIndirect {
                base: X86Reg::Rax,
                width: MemoryReadWidth::Bits64,
            },
        })),
        DecodedInstructionKind::LeaRdiRipRelative { address, .. } => {
            Ok(LiftedInstruction::Op(IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdi),
                src: Operand::AddressRipRelative { address: *address },
            }))
        }
        DecodedInstructionKind::LeaRsiRipRelative { address, .. } => {
            Ok(LiftedInstruction::Op(IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rsi),
                src: Operand::AddressRipRelative { address: *address },
            }))
        }
        DecodedInstructionKind::MovRbpRsp => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rbp),
            src: Operand::Reg(X86Reg::Rsp),
        })),
        DecodedInstructionKind::MovzxEaxBytePtrRdi => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::Mem8 { base: X86Reg::Rdi },
        })),
        DecodedInstructionKind::AddEaxImm32 { imm } => Ok(LiftedInstruction::Op(IrOp::Add {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(imm.as_i64() as u64),
        })),
        DecodedInstructionKind::AddEaxImm8 { imm } => Ok(LiftedInstruction::Op(IrOp::Add {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(imm.as_i64() as u64),
        })),
        DecodedInstructionKind::SubEaxImm32 { imm } => Ok(LiftedInstruction::Op(IrOp::Sub {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(imm.as_i64() as u64),
        })),
        DecodedInstructionKind::SubEaxImm8 { imm } => Ok(LiftedInstruction::Op(IrOp::Sub {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(imm.as_i64() as u64),
        })),
        DecodedInstructionKind::CmpEaxImm32 { imm } => Ok(LiftedInstruction::Op(IrOp::Cmp {
            lhs: Operand::Reg(X86Reg::Rax),
            rhs: Operand::ImmU64(imm.as_i64() as u64),
        })),
        DecodedInstructionKind::CmpEaxImm8 { imm } => Ok(LiftedInstruction::Op(IrOp::Cmp {
            lhs: Operand::Reg(X86Reg::Rax),
            rhs: Operand::ImmU64(imm.as_i64() as u64),
        })),
        DecodedInstructionKind::TestEaxEax => Ok(LiftedInstruction::Op(IrOp::Test {
            lhs: Operand::Reg(X86Reg::Rax),
            rhs: Operand::Reg(X86Reg::Rax),
        })),
        DecodedInstructionKind::PushRax => Ok(LiftedInstruction::Op(IrOp::Push {
            src: Operand::Reg(X86Reg::Rax),
        })),
        DecodedInstructionKind::PushRbx => Ok(LiftedInstruction::Op(IrOp::Push {
            src: Operand::Reg(X86Reg::Rbx),
        })),
        DecodedInstructionKind::PushRbp => Ok(LiftedInstruction::Op(IrOp::Push {
            src: Operand::Reg(X86Reg::Rbp),
        })),
        DecodedInstructionKind::PushR14 => Ok(LiftedInstruction::Op(IrOp::Push {
            src: Operand::Reg(X86Reg::R14),
        })),
        DecodedInstructionKind::PushR15 => Ok(LiftedInstruction::Op(IrOp::Push {
            src: Operand::Reg(X86Reg::R15),
        })),
        DecodedInstructionKind::PopRax => Ok(LiftedInstruction::Op(IrOp::Pop {
            dst: Operand::Reg(X86Reg::Rax),
        })),
        DecodedInstructionKind::XorEaxEax => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(0),
        })),
        DecodedInstructionKind::XorEdxEdx => Ok(LiftedInstruction::Op(IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rdx),
            src: Operand::ImmU64(0),
        })),
        DecodedInstructionKind::BaraHostTrapSentinel => Ok(LiftedInstruction::Op(IrOp::HostTrap {
            kind: HostTrapKind::Stdout,
        })),
        DecodedInstructionKind::BaraAppKitGuiHelloWorldTrapSentinel => {
            Ok(LiftedInstruction::Op(IrOp::HostTrap {
                kind: HostTrapKind::AppKitGuiHelloWorld,
            }))
        }
        DecodedInstructionKind::CallRel32 { target, return_to }
            if instruction_starts.contains(target) =>
        {
            Ok(LiftedInstruction::Terminator(Terminator::DirectCall {
                target: *target,
                return_to: *return_to,
            }))
        }
        DecodedInstructionKind::CallRel32 { target, return_to } => {
            Ok(LiftedInstruction::Terminator(Terminator::Unsupported {
                reason: UnsupportedReason::DirectCallUnsupported {
                    target: *target,
                    return_to: *return_to,
                },
            }))
        }
        DecodedInstructionKind::CallR14 { return_to } => {
            Ok(LiftedInstruction::Terminator(Terminator::Unsupported {
                reason: UnsupportedReason::RegisterIndirectCallUnsupported {
                    target: X86Reg::R14,
                    call_site: instruction.start(),
                    return_to: *return_to,
                },
            }))
        }
        DecodedInstructionKind::JccRel8 {
            condition,
            taken,
            fallthrough,
        }
        | DecodedInstructionKind::JccRel32 {
            condition,
            taken,
            fallthrough,
        } => Ok(LiftedInstruction::Terminator(Terminator::CondJump {
            condition: *condition,
            taken: *taken,
            fallthrough: *fallthrough,
        })),
        DecodedInstructionKind::JmpRel8 { target } => {
            Ok(LiftedInstruction::Terminator(Terminator::DirectJump {
                target: *target,
            }))
        }
        DecodedInstructionKind::Syscall => {
            let request =
                SyscallRequest::new(SyscallAbi::X86_64, instruction.start(), instruction.end())
                    .map_err(LiftError::SyscallRequest)?;
            Ok(LiftedInstruction::Terminator(Terminator::BoundaryRequest {
                request: BoundaryRequest::Syscall(request),
            }))
        }
        DecodedInstructionKind::Ret => Ok(LiftedInstruction::Terminator(Terminator::Return)),
        DecodedInstructionKind::Unsupported { reason } => {
            Ok(LiftedInstruction::Terminator(Terminator::Unsupported {
                reason: reason.clone(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use bara_ir::{
        BlockId, BoundaryRequest, HostTrapKind, IrOp, MemoryReadWidth, Operand,
        ProgramImageImports, ProgramImageMetadata, ProgramImageRange, ProgramImageRelocations,
        ProgramImageSection, ProgramImageSectionKind, ProgramImageSections, ProgramImageSymbols,
        ProgramUnwindMetadata, SyscallAbi, SyscallRequest, Terminator, UnsupportedReason, X86Cond,
        X86Reg, X86Va,
    };

    use crate::{
        lift_decoded_function, lift_decoded_function_with_image_metadata, DecodedFunction,
        DecodedInstruction, DecodedInstructionKind,
    };

    #[test]
    fn lifts_mov_eax_imm32_and_ret_to_single_block_program() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(X86Va::new(5), X86Va::new(6), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded M1 function lifts");
        let block = &program.blocks()[0];

        assert_eq!(program.entry(), X86Va::new(0));
        assert_eq!(block.id(), BlockId::new(0));
        assert_eq!(block.start(), X86Va::new(0));
        assert_eq!(block.end(), X86Va::new(6));
        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42)
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_decoded_function_with_image_metadata() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(X86Va::new(5), X86Va::new(6), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");
        let range = ProgramImageRange::new(X86Va::new(0), X86Va::new(6))
            .expect("metadata range is non-empty");
        let metadata = ProgramImageMetadata::new(
            ProgramImageSections::from_items([ProgramImageSection::new(
                ProgramImageSectionKind::Code,
                range,
            )]),
            ProgramImageSymbols::empty(),
            ProgramImageRelocations::empty(),
            ProgramImageImports::empty(),
            ProgramUnwindMetadata::empty(),
        );

        let program = lift_decoded_function_with_image_metadata(&decoded, metadata.clone())
            .expect("decoded function lifts with metadata");

        assert_eq!(program.image_metadata(), &metadata);
    }

    #[test]
    fn splits_decoded_instruction_stream_at_return_terminators() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(X86Va::new(5), X86Va::new(6), DecodedInstructionKind::Ret),
                DecodedInstruction::new(
                    X86Va::new(6),
                    X86Va::new(11),
                    DecodedInstructionKind::MovEaxImm32 { imm: 7 },
                ),
                DecodedInstruction::new(
                    X86Va::new(11),
                    X86Va::new(12),
                    DecodedInstructionKind::Ret,
                ),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded function lifts");

        assert_eq!(program.blocks().len(), 2);
        assert_eq!(program.blocks()[0].id(), BlockId::new(0));
        assert_eq!(program.blocks()[0].start(), X86Va::new(0));
        assert_eq!(program.blocks()[0].end(), X86Va::new(6));
        assert_eq!(
            program.blocks()[0].ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42)
            }]
        );
        assert_eq!(program.blocks()[0].terminator(), &Terminator::Return);
        assert_eq!(program.blocks()[1].id(), BlockId::new(1));
        assert_eq!(program.blocks()[1].start(), X86Va::new(6));
        assert_eq!(program.blocks()[1].end(), X86Va::new(12));
        assert_eq!(
            program.blocks()[1].ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(7)
            }]
        );
        assert_eq!(program.blocks()[1].terminator(), &Terminator::Return);
    }

    #[test]
    fn does_not_infer_fallthrough_when_decoded_stream_ends_without_terminator() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![DecodedInstruction::new(
                X86Va::new(0),
                X86Va::new(5),
                DecodedInstructionKind::MovEaxImm32 { imm: 42 },
            )],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("unterminated stream lifts");

        assert_eq!(program.blocks().len(), 1);
        assert_eq!(program.blocks()[0].start(), X86Va::new(0));
        assert_eq!(program.blocks()[0].end(), X86Va::new(5));
        assert_eq!(
            program.blocks()[0].ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42)
            }]
        );
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::Unsupported {
                reason: UnsupportedReason::MissingReturnTerminator { at: X86Va::new(5) }
            }
        );
    }

    #[test]
    fn lifts_mov_rax_rdi_to_register_move() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(3),
                    DecodedInstructionKind::MovRaxRdi,
                ),
                DecodedInstruction::new(X86Va::new(3), X86Va::new(4), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded identity function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::Reg(X86Reg::Rdi)
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_mov_rdx_rax_to_register_move() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(3),
                    DecodedInstructionKind::MovRdxRax,
                ),
                DecodedInstruction::new(X86Va::new(3), X86Va::new(4), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded register move lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdx),
                src: Operand::Reg(X86Reg::Rax)
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_movzx_eax_byte_ptr_rdi_to_memory_load() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(3),
                    DecodedInstructionKind::MovzxEaxBytePtrRdi,
                ),
                DecodedInstruction::new(X86Va::new(3), X86Va::new(4), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded memory load lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::Mem8 { base: X86Reg::Rdi }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_bara_host_trap_sentinel_to_stdout_host_trap() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(2),
                    DecodedInstructionKind::BaraHostTrapSentinel,
                ),
                DecodedInstruction::new(
                    X86Va::new(2),
                    X86Va::new(4),
                    DecodedInstructionKind::XorEaxEax,
                ),
                DecodedInstruction::new(X86Va::new(4), X86Va::new(5), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded trap function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::HostTrap {
                    kind: HostTrapKind::Stdout
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_bara_appkit_gui_host_trap_sentinel_to_appkit_gui_host_trap() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(6),
                    DecodedInstructionKind::BaraAppKitGuiHelloWorldTrapSentinel,
                ),
                DecodedInstruction::new(
                    X86Va::new(6),
                    X86Va::new(8),
                    DecodedInstructionKind::XorEaxEax,
                ),
                DecodedInstruction::new(X86Va::new(8), X86Va::new(9), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded trap function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::HostTrap {
                    kind: HostTrapKind::AppKitGuiHelloWorld
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_add_eax_imm8_to_add_rax_immediate() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(5),
                    X86Va::new(8),
                    DecodedInstructionKind::AddEaxImm8 {
                        imm: crate::X86Imm8::new(3),
                    },
                ),
                DecodedInstruction::new(X86Va::new(8), X86Va::new(9), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded add function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42)
                },
                IrOp::Add {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_add_eax_imm32_to_add_rax_immediate() {
        let input = crate::X86Bytes::new(
            X86Va::new(0),
            vec![0xb8, 0x2a, 0, 0, 0, 0x05, 0x03, 0, 0, 0, 0xc3],
        )
        .expect("test bytes are non-empty");
        let decoded = crate::decode_function(&input).expect("test bytes decode");

        let program = lift_decoded_function(&decoded).expect("decoded add function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42)
                },
                IrOp::Add {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_sub_eax_imm8_to_sub_rax_immediate() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(5),
                    X86Va::new(8),
                    DecodedInstructionKind::SubEaxImm8 {
                        imm: crate::X86Imm8::new(3),
                    },
                ),
                DecodedInstruction::new(X86Va::new(8), X86Va::new(9), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded sub function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42)
                },
                IrOp::Sub {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_sub_eax_imm32_to_sub_rax_immediate() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(5),
                    X86Va::new(10),
                    DecodedInstructionKind::SubEaxImm32 {
                        imm: crate::decode::X86Imm32::new(3),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(10),
                    X86Va::new(11),
                    DecodedInstructionKind::Ret,
                ),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded sub function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42)
                },
                IrOp::Sub {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_cmp_eax_imm8_to_cmp_rax_immediate() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(5),
                    X86Va::new(8),
                    DecodedInstructionKind::CmpEaxImm8 {
                        imm: crate::X86Imm8::new(42),
                    },
                ),
                DecodedInstruction::new(X86Va::new(8), X86Va::new(9), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded cmp function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42)
                },
                IrOp::Cmp {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::ImmU64(42)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_cmp_eax_imm32_to_cmp_rax_immediate() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(5),
                    X86Va::new(10),
                    DecodedInstructionKind::CmpEaxImm32 {
                        imm: crate::decode::X86Imm32::new(42),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(10),
                    X86Va::new(11),
                    DecodedInstructionKind::Ret,
                ),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded cmp function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42)
                },
                IrOp::Cmp {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::ImmU64(42)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_test_eax_eax_to_test_rax_rax() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(5),
                    X86Va::new(7),
                    DecodedInstructionKind::TestEaxEax,
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded test function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42)
                },
                IrOp::Test {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::Reg(X86Reg::Rax)
                }
            ]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_push_pop_rax_to_stack_ops() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(1),
                    DecodedInstructionKind::PushRax,
                ),
                DecodedInstruction::new(
                    X86Va::new(1),
                    X86Va::new(2),
                    DecodedInstructionKind::PopRax,
                ),
                DecodedInstruction::new(X86Va::new(2), X86Va::new(3), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded push/pop function lifts");

        assert_eq!(
            program.blocks()[0].ops(),
            &[
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rax)
                },
                IrOp::Pop {
                    dst: Operand::Reg(X86Reg::Rax)
                }
            ]
        );
        assert_eq!(program.blocks()[0].terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_prologue_and_rip_relative_load_batch_before_next_unsupported_opcode() {
        let decoded = DecodedFunction::new(
            X86Va::new(0x1600),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0x1600),
                    X86Va::new(0x1601),
                    DecodedInstructionKind::PushRbp,
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1601),
                    X86Va::new(0x1604),
                    DecodedInstructionKind::MovRbpRsp,
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1604),
                    X86Va::new(0x1606),
                    DecodedInstructionKind::PushR15,
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1606),
                    X86Va::new(0x1608),
                    DecodedInstructionKind::PushR14,
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1608),
                    X86Va::new(0x1609),
                    DecodedInstructionKind::PushRbx,
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1609),
                    X86Va::new(0x160c),
                    DecodedInstructionKind::MovRbxRax,
                ),
                DecodedInstruction::new(
                    X86Va::new(0x160c),
                    X86Va::new(0x1613),
                    DecodedInstructionKind::MovRaxQwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x19ff),
                        address: X86Va::new(0x3012),
                    },
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
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(0x161d),
                    X86Va::new(0x1624),
                    DecodedInstructionKind::LeaRsiRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x10b6),
                        address: X86Va::new(0x26da),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1624),
                    X86Va::new(0x162b),
                    DecodedInstructionKind::MovRdiQwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x3b22),
                        address: X86Va::new(0x514d),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(0x162b),
                    X86Va::new(0x1632),
                    DecodedInstructionKind::MovRsiQwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x3aeb),
                        address: X86Va::new(0x511d),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1632),
                    X86Va::new(0x1639),
                    DecodedInstructionKind::MovR14QwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x1a14),
                        address: X86Va::new(0x304d),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(0x1639),
                    X86Va::new(0x163c),
                    DecodedInstructionKind::CallR14 {
                        return_to: X86Va::new(0x163c),
                    },
                ),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded push rbp function lifts");

        assert_eq!(
            program.blocks()[0].ops(),
            &[
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rbp)
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rbp),
                    src: Operand::Reg(X86Reg::Rsp)
                },
                IrOp::Push {
                    src: Operand::Reg(X86Reg::R15)
                },
                IrOp::Push {
                    src: Operand::Reg(X86Reg::R14)
                },
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rbx)
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rbx),
                    src: Operand::Reg(X86Reg::Rax)
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x3012),
                        width: MemoryReadWidth::Bits64,
                    }
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdx),
                    src: Operand::MemRegIndirect {
                        base: X86Reg::Rax,
                        width: MemoryReadWidth::Bits64,
                    }
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src: Operand::AddressRipRelative {
                        address: X86Va::new(0x26d0),
                    }
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rsi),
                    src: Operand::AddressRipRelative {
                        address: X86Va::new(0x26da),
                    }
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x514d),
                        width: MemoryReadWidth::Bits64,
                    }
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rsi),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x511d),
                        width: MemoryReadWidth::Bits64,
                    }
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::R14),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x304d),
                        width: MemoryReadWidth::Bits64,
                    }
                }
            ]
        );
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::Unsupported {
                reason: UnsupportedReason::RegisterIndirectCallUnsupported {
                    target: X86Reg::R14,
                    call_site: X86Va::new(0x1639),
                    return_to: X86Va::new(0x163c),
                }
            }
        );
    }

    #[test]
    fn lifts_lea_rdi_rip_relative_to_address_materialization() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(7),
                    DecodedInstructionKind::LeaRdiRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x10b3),
                        address: X86Va::new(0x10ba),
                    },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded RIP-relative LEA lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdi),
                src: Operand::AddressRipRelative {
                    address: X86Va::new(0x10ba),
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_lea_rsi_rip_relative_to_address_materialization() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(7),
                    DecodedInstructionKind::LeaRsiRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x10b6),
                        address: X86Va::new(0x10bd),
                    },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded RIP-relative RSI LEA lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rsi),
                src: Operand::AddressRipRelative {
                    address: X86Va::new(0x10bd),
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_mov_rdi_qword_ptr_rip_relative_to_memory_load() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(7),
                    DecodedInstructionKind::MovRdiQwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x3b22),
                        address: X86Va::new(0x3b29),
                    },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded RIP-relative RDI MOV lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdi),
                src: Operand::MemRipRelative {
                    address: X86Va::new(0x3b29),
                    width: MemoryReadWidth::Bits64,
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_mov_rsi_qword_ptr_rip_relative_to_memory_load() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(7),
                    DecodedInstructionKind::MovRsiQwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x3aeb),
                        address: X86Va::new(0x3af2),
                    },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded RIP-relative RSI MOV lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rsi),
                src: Operand::MemRipRelative {
                    address: X86Va::new(0x3af2),
                    width: MemoryReadWidth::Bits64,
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_mov_r14_qword_ptr_rip_relative_to_memory_load() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(7),
                    DecodedInstructionKind::MovR14QwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x1a14),
                        address: X86Va::new(0x1a1b),
                    },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded RIP-relative R14 MOV lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::R14),
                src: Operand::MemRipRelative {
                    address: X86Va::new(0x1a1b),
                    width: MemoryReadWidth::Bits64,
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_mov_r15_qword_ptr_rip_relative_to_memory_load() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(7),
                    DecodedInstructionKind::MovR15QwordPtrRipRelative {
                        displacement: crate::decode::X86Imm32::new(0x1a14),
                        address: X86Va::new(0x1a1b),
                    },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded RIP-relative R15 MOV lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::R15),
                src: Operand::MemRipRelative {
                    address: X86Va::new(0x1a1b),
                    width: MemoryReadWidth::Bits64,
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_mov_rdx_qword_ptr_rax_to_register_indirect_memory_load() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(3),
                    DecodedInstructionKind::MovRdxQwordPtrRax,
                ),
                DecodedInstruction::new(X86Va::new(3), X86Va::new(4), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded RAX-indirect load lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdx),
                src: Operand::MemRegIndirect {
                    base: X86Reg::Rax,
                    width: MemoryReadWidth::Bits64,
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_mov_rdi_qword_ptr_r15_to_register_indirect_memory_load() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(3),
                    DecodedInstructionKind::MovRdiQwordPtrR15,
                ),
                DecodedInstruction::new(X86Va::new(3), X86Va::new(4), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded R15-indirect load lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdi),
                src: Operand::MemRegIndirect {
                    base: X86Reg::R15,
                    width: MemoryReadWidth::Bits64,
                }
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_je_rel8_to_cond_jump_terminator() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(2),
                    DecodedInstructionKind::TestEaxEax,
                ),
                DecodedInstruction::new(
                    X86Va::new(2),
                    X86Va::new(4),
                    DecodedInstructionKind::JccRel8 {
                        condition: X86Cond::Equal,
                        taken: X86Va::new(8),
                        fallthrough: X86Va::new(4),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(4),
                    X86Va::new(9),
                    DecodedInstructionKind::MovEaxImm32 { imm: 0 },
                ),
                DecodedInstruction::new(X86Va::new(9), X86Va::new(10), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded je function lifts");

        assert_eq!(program.blocks().len(), 2);
        assert_eq!(program.blocks()[0].id(), BlockId::new(0));
        assert_eq!(program.blocks()[0].start(), X86Va::new(0));
        assert_eq!(program.blocks()[0].end(), X86Va::new(4));
        assert_eq!(
            program.blocks()[0].ops(),
            &[IrOp::Test {
                lhs: Operand::Reg(X86Reg::Rax),
                rhs: Operand::Reg(X86Reg::Rax)
            }]
        );
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::CondJump {
                condition: X86Cond::Equal,
                taken: X86Va::new(8),
                fallthrough: X86Va::new(4)
            }
        );
        assert_eq!(program.blocks()[1].id(), BlockId::new(1));
        assert_eq!(program.blocks()[1].start(), X86Va::new(4));
        assert_eq!(program.blocks()[1].end(), X86Va::new(10));
        assert_eq!(
            program.blocks()[1].ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(0)
            }]
        );
        assert_eq!(program.blocks()[1].terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_jne_rel8_to_not_equal_cond_jump_terminator() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(2),
                    DecodedInstructionKind::JccRel8 {
                        condition: X86Cond::NotEqual,
                        taken: X86Va::new(0),
                        fallthrough: X86Va::new(2),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(2),
                    X86Va::new(7),
                    DecodedInstructionKind::MovEaxImm32 { imm: 0 },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded jne function lifts");

        assert_eq!(program.blocks().len(), 2);
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::CondJump {
                condition: X86Cond::NotEqual,
                taken: X86Va::new(0),
                fallthrough: X86Va::new(2)
            }
        );
        assert_eq!(program.blocks()[1].start(), X86Va::new(2));
        assert_eq!(program.blocks()[1].terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_jcc_rel32_to_cond_jump_terminator() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(6),
                    DecodedInstructionKind::JccRel32 {
                        condition: X86Cond::Less,
                        taken: X86Va::new(7),
                        fallthrough: X86Va::new(6),
                    },
                ),
                DecodedInstruction::new(X86Va::new(6), X86Va::new(7), DecodedInstructionKind::Ret),
                DecodedInstruction::new(
                    X86Va::new(7),
                    X86Va::new(12),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(12),
                    X86Va::new(13),
                    DecodedInstructionKind::Ret,
                ),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded jcc rel32 function lifts");

        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::CondJump {
                condition: X86Cond::Less,
                taken: X86Va::new(7),
                fallthrough: X86Va::new(6)
            }
        );
    }

    #[test]
    fn lifts_jmp_rel8_to_direct_jump_terminator() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(2),
                    DecodedInstructionKind::JmpRel8 {
                        target: X86Va::new(8),
                    },
                ),
                DecodedInstruction::new(
                    X86Va::new(2),
                    X86Va::new(7),
                    DecodedInstructionKind::MovEaxImm32 { imm: 7 },
                ),
                DecodedInstruction::new(X86Va::new(7), X86Va::new(8), DecodedInstructionKind::Ret),
                DecodedInstruction::new(
                    X86Va::new(8),
                    X86Va::new(13),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(13),
                    X86Va::new(14),
                    DecodedInstructionKind::Ret,
                ),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded jmp function lifts");

        assert_eq!(program.blocks().len(), 3);
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::DirectJump {
                target: X86Va::new(8)
            }
        );
        assert_eq!(program.blocks()[1].start(), X86Va::new(2));
        assert_eq!(program.blocks()[1].terminator(), &Terminator::Return);
        assert_eq!(program.blocks()[2].start(), X86Va::new(8));
        assert_eq!(program.blocks()[2].terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_xor_eax_eax_to_mov_rax_zero() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(2),
                    DecodedInstructionKind::XorEaxEax,
                ),
                DecodedInstruction::new(X86Va::new(2), X86Va::new(3), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded xor function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(0)
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_xor_edx_edx_to_mov_rdx_zero() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(2),
                    DecodedInstructionKind::XorEdxEdx,
                ),
                DecodedInstruction::new(X86Va::new(2), X86Va::new(3), DecodedInstructionKind::Ret),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded xor function lifts");
        let block = &program.blocks()[0];

        assert_eq!(
            block.ops(),
            &[IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rdx),
                src: Operand::ImmU64(0)
            }]
        );
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn lifts_unsupported_instruction_to_unsupported_terminator() {
        let reason = UnsupportedReason::DecodeUnsupportedOpcode {
            opcode: 0x90,
            at: X86Va::new(0),
        };
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![DecodedInstruction::new(
                X86Va::new(0),
                X86Va::new(1),
                DecodedInstructionKind::Unsupported {
                    reason: reason.clone(),
                },
            )],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("unsupported decode lifts to IR");

        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::Unsupported { reason }
        );
    }

    #[test]
    fn lifts_direct_call_to_unsupported_terminator() {
        let target = X86Va::new(0x1015);
        let return_to = X86Va::new(0x1005);
        let decoded = DecodedFunction::new(
            X86Va::new(0x1000),
            vec![DecodedInstruction::new(
                X86Va::new(0x1000),
                return_to,
                DecodedInstructionKind::CallRel32 { target, return_to },
            )],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("direct call lifts to unsupported IR");

        assert_eq!(program.blocks()[0].ops(), &[]);
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::Unsupported {
                reason: UnsupportedReason::DirectCallUnsupported { target, return_to }
            }
        );
    }

    #[test]
    fn lifts_call_r14_to_register_indirect_call_unsupported_terminator() {
        let return_to = X86Va::new(0x1003);
        let decoded = DecodedFunction::new(
            X86Va::new(0x1000),
            vec![DecodedInstruction::new(
                X86Va::new(0x1000),
                return_to,
                DecodedInstructionKind::CallR14 { return_to },
            )],
        )
        .expect("decoded function has instructions");

        let program =
            lift_decoded_function(&decoded).expect("indirect call lifts to unsupported IR");

        assert_eq!(program.blocks()[0].ops(), &[]);
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::Unsupported {
                reason: UnsupportedReason::RegisterIndirectCallUnsupported {
                    target: X86Reg::R14,
                    call_site: X86Va::new(0x1000),
                    return_to,
                }
            }
        );
    }

    #[test]
    fn lifts_direct_call_to_direct_call_terminator_when_target_is_decoded() {
        let decoded = DecodedFunction::new(
            X86Va::new(0),
            vec![
                DecodedInstruction::new(
                    X86Va::new(0),
                    X86Va::new(5),
                    DecodedInstructionKind::CallRel32 {
                        target: X86Va::new(6),
                        return_to: X86Va::new(5),
                    },
                ),
                DecodedInstruction::new(X86Va::new(5), X86Va::new(6), DecodedInstructionKind::Ret),
                DecodedInstruction::new(
                    X86Va::new(6),
                    X86Va::new(11),
                    DecodedInstructionKind::MovEaxImm32 { imm: 42 },
                ),
                DecodedInstruction::new(
                    X86Va::new(11),
                    X86Va::new(12),
                    DecodedInstructionKind::Ret,
                ),
            ],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("decoded direct call lifts");

        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::DirectCall {
                target: X86Va::new(6),
                return_to: X86Va::new(5),
            }
        );
        assert_eq!(program.blocks()[1].start(), X86Va::new(5));
        assert_eq!(program.blocks()[2].start(), X86Va::new(6));
    }

    #[test]
    fn lifts_syscall_to_boundary_request_terminator() {
        let decoded = DecodedFunction::new(
            X86Va::new(0x1000),
            vec![DecodedInstruction::new(
                X86Va::new(0x1000),
                X86Va::new(0x1002),
                DecodedInstructionKind::Syscall,
            )],
        )
        .expect("decoded function has instructions");

        let program = lift_decoded_function(&decoded).expect("syscall lifts to IR request");
        let request =
            SyscallRequest::new(SyscallAbi::X86_64, X86Va::new(0x1000), X86Va::new(0x1002))
                .expect("test syscall range is valid");

        assert_eq!(program.blocks()[0].ops(), &[]);
        assert_eq!(
            program.blocks()[0].terminator(),
            &Terminator::BoundaryRequest {
                request: BoundaryRequest::Syscall(request)
            }
        );
    }
}
