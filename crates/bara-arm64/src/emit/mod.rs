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
    let mut last_rax_imm = None;

    for op in block.ops() {
        match op {
            IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(value),
            } => {
                last_rax_imm = Some(*value);
            }
            IrOp::Mov { .. } => {
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
            let value = last_rax_imm.ok_or(EmitError::UnsupportedShape)?;
            emit_mov_x0_u64(&mut code, value);
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

fn emit_u32_le(code: &mut Vec<u8>, instruction: u32) -> usize {
    code.extend_from_slice(&instruction.to_le_bytes());
    code.len()
}
