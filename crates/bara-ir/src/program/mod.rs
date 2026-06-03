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
