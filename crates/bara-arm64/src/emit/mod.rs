use bara_ir::{validate_program, IrOp, Operand, Program, Terminator, UnsupportedReason, X86Reg};

use crate::{ArmPc, PcMapEntry};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arm64MachineCode {
    bytes: Vec<u8>,
}

impl Arm64MachineCode {
    pub fn new(bytes: Vec<u8>) -> Result<Self, EmitError> {
        if bytes.is_empty() {
            return Err(EmitError::EmptyCode);
        }

        Ok(Self { bytes })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmittedFunction {
    code: Arm64MachineCode,
    pc_map: Vec<PcMapEntry>,
}

impl EmittedFunction {
    pub fn new(code: Arm64MachineCode, pc_map: Vec<PcMapEntry>) -> Self {
        Self { code, pc_map }
    }

    pub const fn code(&self) -> &Arm64MachineCode {
        &self.code
    }

    pub fn pc_map(&self) -> &[PcMapEntry] {
        &self.pc_map
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmitError {
    InvalidProgram,
    EmptyCode,
    UnsupportedIr { reason: UnsupportedReason },
    UnsupportedShape,
}

pub fn emit_program(program: &Program) -> Result<EmittedFunction, EmitError> {
    if !validate_program(program).is_valid() {
        return Err(EmitError::InvalidProgram);
    }

    let [block] = program.blocks() else {
        return Err(EmitError::UnsupportedShape);
    };

    let mut code = Vec::new();
    let mut has_rax_value = false;

    for op in block.ops() {
        match op {
            IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(value),
            } => {
                emit_mov_x0_u64(&mut code, *value);
                has_rax_value = true;
            }
            IrOp::Mov { .. } => {
                return Err(EmitError::UnsupportedIr {
                    reason: UnsupportedReason::EmitUnsupportedIr,
                });
            }
            IrOp::Add {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(value),
            } => {
                if !has_rax_value {
                    return Err(EmitError::UnsupportedShape);
                }
                emit_add_x0_imm12(&mut code, *value)?;
            }
            IrOp::Add { .. } => {
                return Err(EmitError::UnsupportedIr {
                    reason: UnsupportedReason::EmitUnsupportedIr,
                });
            }
            IrOp::Sub {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(value),
            } => {
                if !has_rax_value {
                    return Err(EmitError::UnsupportedShape);
                }
                emit_sub_x0_imm12(&mut code, *value)?;
            }
            IrOp::Sub { .. } => {
                return Err(EmitError::UnsupportedIr {
                    reason: UnsupportedReason::EmitUnsupportedIr,
                });
            }
            IrOp::Unsupported { reason } => {
                return Err(EmitError::UnsupportedIr {
                    reason: reason.clone(),
                });
            }
        }
    }

    match block.terminator() {
        Terminator::Return => {
            if !has_rax_value {
                return Err(EmitError::UnsupportedShape);
            }
            emit_u32_le(&mut code, 0xd65f_03c0);
        }
        Terminator::Unsupported { reason } => {
            return Err(EmitError::UnsupportedIr {
                reason: reason.clone(),
            });
        }
    }

    let machine_code = Arm64MachineCode::new(code)?;
    let pc_map = vec![PcMapEntry::new(block.start(), ArmPc::new(0))];
    Ok(EmittedFunction::new(machine_code, pc_map))
}

fn emit_mov_x0_u64(code: &mut Vec<u8>, value: u64) -> usize {
    let mut emitted = 0usize;
    let mut wrote_first = false;

    for hw in 0..4u32 {
        let imm16 = ((value >> (hw * 16)) & 0xffff) as u32;
        if imm16 != 0 || !wrote_first {
            let opcode = if wrote_first {
                0xf280_0000
            } else {
                0xd280_0000
            };
            emit_u32_le(code, opcode | (hw << 21) | (imm16 << 5));
            wrote_first = true;
            emitted += 1;
        }
    }

    emitted
}

fn emit_add_x0_imm12(code: &mut Vec<u8>, value: u64) -> Result<usize, EmitError> {
    let Ok(imm12) = u32::try_from(value) else {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    };

    if imm12 > 0xfff {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    }

    Ok(emit_u32_le(code, 0x9100_0000 | (imm12 << 10)))
}

fn emit_sub_x0_imm12(code: &mut Vec<u8>, value: u64) -> Result<usize, EmitError> {
    let Ok(imm12) = u32::try_from(value) else {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    };

    if imm12 > 0xfff {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    }

    Ok(emit_u32_le(code, 0xd100_0000 | (imm12 << 10)))
}

fn emit_u32_le(code: &mut Vec<u8>, instruction: u32) -> usize {
    code.extend_from_slice(&instruction.to_le_bytes());
    code.len()
}

#[cfg(test)]
mod tests {
    use bara_ir::{
        BasicBlock, BlockId, IrOp, Operand, Program, Terminator, UnsupportedReason, X86Reg, X86Va,
    };

    use crate::{emit_program, Arm64MachineCode, ArmPc, EmitError};

    fn program_with_ops(ops: Vec<IrOp>, terminator: Terminator) -> Program {
        let block = BasicBlock::new(
            BlockId::new(0),
            X86Va::new(0),
            X86Va::new(1),
            ops,
            terminator,
        )
        .expect("test block range is valid");
        Program::new(X86Va::new(0), vec![block]).expect("program has entry block")
    }

    #[test]
    fn machine_code_rejects_empty_bytes() {
        assert_eq!(Arm64MachineCode::new(Vec::new()), Err(EmitError::EmptyCode));
    }

    #[test]
    fn emits_mov_x0_imm_and_ret_for_rax_immediate_return() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42),
            }],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("M1 IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
        assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
        assert_eq!(emitted.pc_map()[0].target(), ArmPc::new(0));
    }

    #[test]
    fn emits_add_x0_immediate_for_rax_add_immediate() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Add {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("add immediate IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_sub_x0_immediate_for_rax_sub_immediate() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Sub {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("sub immediate IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0xd1, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_mov_x0_with_multiple_u16_chunks() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(0x0001_0000_0000_0002),
            }],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("multi-chunk immediate emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x00, 0x80, 0xd2, 0x20, 0x00, 0xe0, 0xf2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn return_without_rax_immediate_is_unsupported_shape() {
        let program = program_with_ops(Vec::new(), Terminator::Return);

        assert_eq!(emit_program(&program), Err(EmitError::UnsupportedShape));
    }

    #[test]
    fn unsupported_terminator_is_invalid_program() {
        let program = program_with_ops(
            Vec::new(),
            Terminator::Unsupported {
                reason: UnsupportedReason::MissingReturnTerminator { at: X86Va::new(1) },
            },
        );

        assert_eq!(emit_program(&program), Err(EmitError::InvalidProgram));
    }
}
