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
