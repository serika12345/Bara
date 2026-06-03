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
    Unsupported { reason: UnsupportedReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Terminator {
    Return,
    Unsupported { reason: UnsupportedReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operand {
    Reg(X86Reg),
    ImmU64(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Reg {
    Rax,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    DecodeUnsupportedOpcode { opcode: u8, at: X86Va },
    MissingReturnTerminator { at: X86Va },
    EmitUnsupportedIr,
}
