use bara_ir::{
    BasicBlock, BasicBlockError, BlockId, IrOp, Operand, Program, ProgramError, Terminator, X86Reg,
};

use crate::{DecodeError, DecodedFunction, DecodedInstructionKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiftError {
    EmptyDecodedFunction,
    BasicBlock(BasicBlockError),
    Program(ProgramError),
    Decode(DecodeError),
}

pub fn lift_decoded_function(decoded: &DecodedFunction) -> Result<Program, LiftError> {
    let mut ops = Vec::new();
    let mut terminator = None;
    let mut block_end = decoded.entry();

    for instruction in decoded.instructions() {
        block_end = instruction.end();
        match instruction.kind() {
            DecodedInstructionKind::MovEaxImm32 { imm } => {
                ops.push(IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(u64::from(*imm)),
                });
            }
            DecodedInstructionKind::MovRaxRdi => {
                ops.push(IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::Reg(X86Reg::Rdi),
                });
            }
            DecodedInstructionKind::MovzxEaxBytePtrRdi => {
                ops.push(IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::Mem8 { base: X86Reg::Rdi },
                });
            }
            DecodedInstructionKind::AddEaxImm32 { imm } => {
                ops.push(IrOp::Add {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(imm.as_i64() as u64),
                });
            }
            DecodedInstructionKind::AddEaxImm8 { imm } => {
                ops.push(IrOp::Add {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(imm.as_i64() as u64),
                });
            }
            DecodedInstructionKind::SubEaxImm32 { imm } => {
                ops.push(IrOp::Sub {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(imm.as_i64() as u64),
                });
            }
            DecodedInstructionKind::SubEaxImm8 { imm } => {
                ops.push(IrOp::Sub {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(imm.as_i64() as u64),
                });
            }
            DecodedInstructionKind::XorEaxEax => {
                ops.push(IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0),
                });
            }
            DecodedInstructionKind::Ret => {
                terminator = Some(Terminator::Return);
                break;
            }
            DecodedInstructionKind::Unsupported { reason } => {
                terminator = Some(Terminator::Unsupported {
                    reason: reason.clone(),
                });
                break;
            }
        }
    }

    let terminator = terminator.ok_or(LiftError::EmptyDecodedFunction)?;
    let block = BasicBlock::new(BlockId::new(0), decoded.entry(), block_end, ops, terminator)
        .map_err(LiftError::BasicBlock)?;

    Program::new(decoded.entry(), vec![block]).map_err(LiftError::Program)
}

#[cfg(test)]
mod tests {
    use bara_ir::{BlockId, IrOp, Operand, Terminator, UnsupportedReason, X86Reg, X86Va};

    use crate::{
        lift_decoded_function, DecodedFunction, DecodedInstruction, DecodedInstructionKind,
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
}
