use crate::block::{BasicBlock, BlockId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct X86Va(u64);

impl X86Va {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, byte_len: u64) -> Result<Self, ProgramError> {
        self.0
            .checked_add(byte_len)
            .map(Self)
            .ok_or(ProgramError::AddressOverflow {
                start: self,
                byte_len,
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    entry: X86Va,
    blocks: Vec<BasicBlock>,
}

impl Program {
    pub fn new(entry: X86Va, blocks: Vec<BasicBlock>) -> Result<Self, ProgramError> {
        let has_entry = blocks.iter().any(|block| block.start() == entry);
        if !has_entry {
            return Err(ProgramError::MissingEntryBlock { entry });
        }

        let mut seen = Vec::new();
        for block in &blocks {
            if seen.contains(&block.id()) {
                return Err(ProgramError::DuplicateBlockId { id: block.id() });
            }
            seen.push(block.id());
        }

        Ok(Self { entry, blocks })
    }

    pub const fn entry(&self) -> X86Va {
        self.entry
    }

    pub fn blocks(&self) -> &[BasicBlock] {
        &self.blocks
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgramError {
    AddressOverflow { start: X86Va, byte_len: u64 },
    MissingEntryBlock { entry: X86Va },
    DuplicateBlockId { id: BlockId },
}

#[cfg(test)]
mod tests {
    use crate::{BasicBlock, BlockId, Program, ProgramError, Terminator, X86Va};

    fn block(id: u32, start: u64, end: u64) -> BasicBlock {
        BasicBlock::new(
            BlockId::new(id),
            X86Va::new(start),
            X86Va::new(end),
            Vec::new(),
            Terminator::Return,
        )
        .expect("test block range is valid")
    }

    #[test]
    fn x86_va_checked_add_returns_typed_address() {
        assert_eq!(X86Va::new(0x1000).checked_add(5), Ok(X86Va::new(0x1005)));
    }

    #[test]
    fn x86_va_checked_add_reports_overflow() {
        assert_eq!(
            X86Va::new(u64::MAX).checked_add(1),
            Err(ProgramError::AddressOverflow {
                start: X86Va::new(u64::MAX),
                byte_len: 1
            })
        );
    }

    #[test]
    fn program_requires_entry_block() {
        assert_eq!(
            Program::new(X86Va::new(0), vec![block(0, 1, 2)]),
            Err(ProgramError::MissingEntryBlock {
                entry: X86Va::new(0)
            })
        );
    }

    #[test]
    fn program_rejects_duplicate_block_id() {
        assert_eq!(
            Program::new(X86Va::new(0), vec![block(0, 0, 1), block(0, 1, 2)]),
            Err(ProgramError::DuplicateBlockId {
                id: BlockId::new(0)
            })
        );
    }

    #[test]
    fn program_exposes_entry_and_blocks() {
        let program = Program::new(X86Va::new(0), vec![block(7, 0, 1)])
            .expect("program has entry block and unique block id");

        assert_eq!(program.entry(), X86Va::new(0));
        assert_eq!(program.blocks()[0].id(), BlockId::new(7));
    }
}
