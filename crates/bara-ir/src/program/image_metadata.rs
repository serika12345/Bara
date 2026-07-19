use crate::{
    boundary::{ExternalSymbolId, ExternalSymbolImport},
    program::X86Va,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramImageMetadata {
    sections: ProgramImageSections,
    mapped_bytes: ProgramImageMappedBytes,
    symbols: ProgramImageSymbols,
    relocations: ProgramImageRelocations,
    imports: ProgramImageImports,
    unwind: ProgramUnwindMetadata,
}

impl ProgramImageMetadata {
    pub fn empty() -> Self {
        Self::default()
    }

    pub const fn new(
        sections: ProgramImageSections,
        symbols: ProgramImageSymbols,
        relocations: ProgramImageRelocations,
        imports: ProgramImageImports,
        unwind: ProgramUnwindMetadata,
    ) -> Self {
        Self {
            sections,
            mapped_bytes: ProgramImageMappedBytes::empty(),
            symbols,
            relocations,
            imports,
            unwind,
        }
    }

    pub const fn new_with_mapped_bytes(
        sections: ProgramImageSections,
        mapped_bytes: ProgramImageMappedBytes,
        symbols: ProgramImageSymbols,
        relocations: ProgramImageRelocations,
        imports: ProgramImageImports,
        unwind: ProgramUnwindMetadata,
    ) -> Self {
        Self {
            sections,
            mapped_bytes,
            symbols,
            relocations,
            imports,
            unwind,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
            && self.mapped_bytes.is_empty()
            && self.symbols.is_empty()
            && self.relocations.is_empty()
            && self.imports.is_empty()
            && self.unwind.is_empty()
    }

    pub const fn sections(&self) -> &ProgramImageSections {
        &self.sections
    }

    pub const fn mapped_bytes(&self) -> &ProgramImageMappedBytes {
        &self.mapped_bytes
    }

    pub const fn symbols(&self) -> &ProgramImageSymbols {
        &self.symbols
    }

    pub const fn relocations(&self) -> &ProgramImageRelocations {
        &self.relocations
    }

    pub const fn imports(&self) -> &ProgramImageImports {
        &self.imports
    }

    pub const fn unwind(&self) -> &ProgramUnwindMetadata {
        &self.unwind
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramImageSections {
    items: Vec<ProgramImageSection>,
}

impl ProgramImageSections {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_items(items: impl IntoIterator<Item = ProgramImageSection>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[ProgramImageSection] {
        &self.items
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramImageSection {
    kind: ProgramImageSectionKind,
    range: ProgramImageRange,
}

impl ProgramImageSection {
    pub const fn new(kind: ProgramImageSectionKind, range: ProgramImageRange) -> Self {
        Self { kind, range }
    }

    pub const fn kind(self) -> ProgramImageSectionKind {
        self.kind
    }

    pub const fn range(self) -> ProgramImageRange {
        self.range
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramImageMappedBytes {
    segments: Vec<ProgramImageMappedByteSegment>,
}

impl ProgramImageMappedBytes {
    pub const fn empty() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn from_segments(
        segments: impl IntoIterator<Item = ProgramImageMappedByteSegment>,
    ) -> Self {
        Self {
            segments: segments.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn segment_for_range(
        &self,
        range: ProgramImageRange,
    ) -> Result<ProgramImageMappedByteSegment, ProgramImageMetadataError> {
        self.segments
            .iter()
            .find_map(|segment| segment.segment_for_range(range))
            .transpose()?
            .ok_or(ProgramImageMetadataError::MappedBytesRangeUnavailable)
    }

    pub fn read_u64_le(&self, address: X86Va) -> Option<u64> {
        self.segments
            .iter()
            .find_map(|segment| segment.read_u64_le(address))
    }

    pub fn read_nul_terminated_utf8(&self, address: X86Va) -> Option<&str> {
        self.segments
            .iter()
            .find_map(|segment| segment.read_nul_terminated_utf8(address))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramImageMappedByteSegment {
    range: ProgramImageRange,
    bytes: Vec<u8>,
}

impl ProgramImageMappedByteSegment {
    pub fn new(
        range: ProgramImageRange,
        bytes: Vec<u8>,
    ) -> Result<Self, ProgramImageMetadataError> {
        let byte_len =
            u64::try_from(bytes.len()).map_err(|_| ProgramImageMetadataError::AddressOverflow)?;
        if range.byte_len() != byte_len {
            return Err(ProgramImageMetadataError::MappedBytesLengthMismatch);
        }

        Ok(Self { range, bytes })
    }

    pub const fn range(&self) -> ProgramImageRange {
        self.range
    }

    fn segment_for_range(
        &self,
        range: ProgramImageRange,
    ) -> Option<Result<Self, ProgramImageMetadataError>> {
        if range.start() < self.range.start() || range.end() > self.range.end() {
            return None;
        }

        Some((|| {
            let start = usize::try_from(range.start().value() - self.range.start().value())
                .map_err(|_| ProgramImageMetadataError::AddressOverflow)?;
            let end = usize::try_from(range.end().value() - self.range.start().value())
                .map_err(|_| ProgramImageMetadataError::AddressOverflow)?;
            let bytes = self
                .bytes
                .get(start..end)
                .ok_or(ProgramImageMetadataError::MappedBytesRangeUnavailable)?
                .to_vec();
            Self::new(range, bytes)
        })())
    }

    fn read_u64_le(&self, address: X86Va) -> Option<u64> {
        let read_end = u128::from(address.value()).checked_add(8)?;
        if address.value() < self.range.start().value()
            || read_end > u128::from(self.range.end().value())
        {
            return None;
        }

        let offset = usize::try_from(address.value() - self.range.start().value()).ok()?;
        let bytes = self.bytes.get(offset..offset.checked_add(8)?)?;
        Some(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_nul_terminated_utf8(&self, address: X86Va) -> Option<&str> {
        if address.value() < self.range.start().value()
            || address.value() >= self.range.end().value()
        {
            return None;
        }

        let offset = usize::try_from(address.value() - self.range.start().value()).ok()?;
        let bytes = self.bytes.get(offset..)?;
        let nul_offset = bytes.iter().position(|byte| *byte == 0)?;
        std::str::from_utf8(bytes.get(..nul_offset)?).ok()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramImageSectionKind {
    Code,
    ConstData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramImageRange {
    start: X86Va,
    end: X86Va,
}

impl ProgramImageRange {
    pub fn new(start: X86Va, end: X86Va) -> Result<Self, ProgramImageMetadataError> {
        if start >= end {
            return Err(ProgramImageMetadataError::EmptyOrReversedRange);
        }

        Ok(Self { start, end })
    }

    pub const fn start(self) -> X86Va {
        self.start
    }

    pub const fn end(self) -> X86Va {
        self.end
    }

    const fn byte_len(self) -> u64 {
        self.end.value() - self.start.value()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramImageSymbols {
    items: Vec<ProgramImageSymbol>,
}

impl ProgramImageSymbols {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_items(items: impl IntoIterator<Item = ProgramImageSymbol>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[ProgramImageSymbol] {
        &self.items
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramImageSymbol {
    ExternalImport(ExternalSymbolImport),
}

impl ProgramImageSymbol {
    pub const fn external_import(import: ExternalSymbolImport) -> Self {
        Self::ExternalImport(import)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramImageRelocations {
    items: Vec<ProgramImageRelocation>,
}

impl ProgramImageRelocations {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_items(items: impl IntoIterator<Item = ProgramImageRelocation>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[ProgramImageRelocation] {
        &self.items
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramImageRelocation {
    at: X86Va,
    target: ProgramImageRelocationTarget,
}

impl ProgramImageRelocation {
    pub const fn new(at: X86Va, target: ProgramImageRelocationTarget) -> Self {
        Self { at, target }
    }

    pub const fn at(self) -> X86Va {
        self.at
    }

    pub const fn target(self) -> ProgramImageRelocationTarget {
        self.target
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramImageRelocationTarget {
    Address(X86Va),
    ExternalSymbol(ExternalSymbolId),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramImageImports {
    items: Vec<ProgramImageImport>,
}

impl ProgramImageImports {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_items(items: impl IntoIterator<Item = ProgramImageImport>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[ProgramImageImport] {
        &self.items
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramImageImport {
    import: ExternalSymbolImport,
}

impl ProgramImageImport {
    pub const fn new(import: ExternalSymbolImport) -> Self {
        Self { import }
    }

    pub const fn import(self) -> ExternalSymbolImport {
        self.import
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramUnwindMetadata {
    entries: Vec<ProgramUnwindEntry>,
}

impl ProgramUnwindMetadata {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: impl IntoIterator<Item = ProgramUnwindEntry>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[ProgramUnwindEntry] {
        &self.entries
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramUnwindEntry {
    range: ProgramImageRange,
}

impl ProgramUnwindEntry {
    pub const fn new(range: ProgramImageRange) -> Self {
        Self { range }
    }

    pub const fn range(self) -> ProgramImageRange {
        self.range
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramImageMetadataError {
    EmptyOrReversedRange,
    AddressOverflow,
    MappedBytesLengthMismatch,
    MappedBytesRangeUnavailable,
}
