use crate::boundary::{BoundaryRequest, SyscallRequest};
use crate::program::X86Va;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct BlockId(u32);

impl BlockId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasicBlock {
    id: BlockId,
    start: X86Va,
    end: X86Va,
    ops: Vec<IrOp>,
    terminator: Terminator,
}

impl BasicBlock {
    pub fn new(
        id: BlockId,
        start: X86Va,
        end: X86Va,
        ops: Vec<IrOp>,
        terminator: Terminator,
    ) -> Result<Self, BasicBlockError> {
        if start >= end {
            return Err(BasicBlockError::EmptyOrReversedRange { start, end });
        }

        Ok(Self {
            id,
            start,
            end,
            ops,
            terminator,
        })
    }

    pub const fn id(&self) -> BlockId {
        self.id
    }

    pub const fn start(&self) -> X86Va {
        self.start
    }

    pub const fn end(&self) -> X86Va {
        self.end
    }

    pub fn ops(&self) -> &[IrOp] {
        &self.ops
    }

    pub const fn terminator(&self) -> &Terminator {
        &self.terminator
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BasicBlockError {
    EmptyOrReversedRange { start: X86Va, end: X86Va },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrOp {
    Mov { dst: Operand, src: Operand },
    Add { dst: Operand, src: Operand },
    Sub { dst: Operand, src: Operand },
    HostTrap { kind: HostTrapKind },
    Unsupported { reason: UnsupportedReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Terminator {
    Return,
    BoundaryRequest { request: BoundaryRequest },
    Unsupported { reason: UnsupportedReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operand {
    Reg(X86Reg),
    ImmU64(u64),
    Mem8 { base: X86Reg },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Reg {
    Rax,
    Rdi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostTrapKind {
    Stdout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    DecodeUnsupportedOpcode { opcode: u8, at: X86Va },
    MissingReturnTerminator { at: X86Va },
    DirectCallUnsupported { target: X86Va, return_to: X86Va },
    SyscallUnsupported { request: SyscallRequest },
    EmitUnsupportedIr,
}

#[cfg(test)]
mod tests {
    use crate::{BasicBlock, BasicBlockError, BlockId, IrOp, Operand, Terminator, X86Reg, X86Va};

    #[test]
    fn block_id_exposes_value() {
        assert_eq!(BlockId::new(9).value(), 9);
    }

    #[test]
    fn basic_block_rejects_empty_range() {
        assert_eq!(
            BasicBlock::new(
                BlockId::new(0),
                X86Va::new(4),
                X86Va::new(4),
                Vec::new(),
                Terminator::Return
            ),
            Err(BasicBlockError::EmptyOrReversedRange {
                start: X86Va::new(4),
                end: X86Va::new(4)
            })
        );
    }

    #[test]
    fn basic_block_rejects_reversed_range() {
        assert_eq!(
            BasicBlock::new(
                BlockId::new(0),
                X86Va::new(5),
                X86Va::new(4),
                Vec::new(),
                Terminator::Return
            ),
            Err(BasicBlockError::EmptyOrReversedRange {
                start: X86Va::new(5),
                end: X86Va::new(4)
            })
        );
    }

    #[test]
    fn basic_block_exposes_fields() {
        let op = IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(42),
        };
        let block = BasicBlock::new(
            BlockId::new(1),
            X86Va::new(0),
            X86Va::new(6),
            vec![op.clone()],
            Terminator::Return,
        )
        .expect("test block range is valid");

        assert_eq!(block.id(), BlockId::new(1));
        assert_eq!(block.start(), X86Va::new(0));
        assert_eq!(block.end(), X86Va::new(6));
        assert_eq!(block.ops(), &[op]);
        assert_eq!(block.terminator(), &Terminator::Return);
    }
}
