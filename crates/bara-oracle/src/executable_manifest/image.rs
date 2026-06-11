use bara_ir::X86Va;
use bara_isa_x86::{DecodeError, X86Bytes};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutableEntry {
    offset: X86Va,
}

impl ExecutableEntry {
    pub const fn new(offset: X86Va) -> Self {
        Self { offset }
    }

    pub const fn offset(self) -> X86Va {
        self.offset
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeSegment {
    bytes: X86Bytes,
}

impl CodeSegment {
    pub const fn from_x86_bytes(bytes: X86Bytes) -> Self {
        Self { bytes }
    }

    pub const fn x86_bytes(&self) -> &X86Bytes {
        &self.bytes
    }

    fn contains_entry(&self, entry: ExecutableEntry) -> bool {
        self.entry_byte_offset(entry).is_ok()
    }

    fn bytes_from_entry(&self, entry: ExecutableEntry) -> Result<X86Bytes, ExecutableImageError> {
        let offset = self.entry_byte_offset(entry)?;
        let bytes = self
            .bytes
            .bytes()
            .get(offset..)
            .ok_or(ExecutableImageError::EntryOutOfCodeSegment)?
            .to_vec();

        X86Bytes::new(entry.offset(), bytes).map_err(ExecutableImageError::DecodeInput)
    }

    fn entry_byte_offset(&self, entry: ExecutableEntry) -> Result<usize, ExecutableImageError> {
        let relative_offset = entry
            .offset()
            .value()
            .checked_sub(self.bytes.entry().value())
            .ok_or(ExecutableImageError::EntryOutOfCodeSegment)?;
        let offset = usize::try_from(relative_offset)
            .map_err(|_| ExecutableImageError::EntryOutOfCodeSegment)?;

        if offset < self.bytes.bytes().len() {
            Ok(offset)
        } else {
            Err(ExecutableImageError::EntryOutOfCodeSegment)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableImage {
    code_segment: CodeSegment,
    entry: ExecutableEntry,
}

impl ExecutableImage {
    pub fn new(
        code_segment: CodeSegment,
        entry: ExecutableEntry,
    ) -> Result<Self, ExecutableImageError> {
        if !code_segment.contains_entry(entry) {
            return Err(ExecutableImageError::EntryOutOfCodeSegment);
        }

        Ok(Self {
            code_segment,
            entry,
        })
    }

    pub const fn code_segment(&self) -> &CodeSegment {
        &self.code_segment
    }

    pub const fn entry(&self) -> ExecutableEntry {
        self.entry
    }

    pub fn entry_function_bytes(&self) -> Result<X86Bytes, ExecutableImageError> {
        self.code_segment.bytes_from_entry(self.entry)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableImageError {
    EntryOutOfCodeSegment,
    DecodeInput(DecodeError),
}
