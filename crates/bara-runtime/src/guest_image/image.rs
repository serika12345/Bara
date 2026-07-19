use bara_ir::{
    ProgramImageImports, ProgramImageMappedBytes, ProgramImageMetadataError, ProgramImageRange,
    ProgramImageRelocations, ProgramImageSections, ProgramImageSymbols, ProgramUnwindMetadata,
    X86Va,
};

use super::{GuestImageMappedBytesSource, GuestImageMetadata};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImage {
    format: GuestImageFormat,
    entry_point: GuestImageEntryPoint,
    segments: GuestImageSegments,
    metadata: GuestImageMetadata,
}

impl GuestImage {
    pub fn mach_o_executable(
        entry_point: GuestImageEntryPoint,
        code_segment: GuestImageSegment,
        metadata: GuestImageMetadata,
    ) -> Result<Self, GuestImageError> {
        Self::new(
            GuestImageFormat::MachO,
            entry_point,
            GuestImageSegments::from_items([code_segment]),
            metadata,
        )
    }

    pub fn new(
        format: GuestImageFormat,
        entry_point: GuestImageEntryPoint,
        segments: GuestImageSegments,
        metadata: GuestImageMetadata,
    ) -> Result<Self, GuestImageError> {
        if segments.code_segment().is_none() {
            return Err(GuestImageError::MissingCodeSegment);
        }

        if !segments.contains_entry(entry_point) {
            return Err(GuestImageError::EntryOutsideMappedSegments);
        }

        Ok(Self {
            format,
            entry_point,
            segments,
            metadata,
        })
    }

    pub const fn format(&self) -> GuestImageFormat {
        self.format
    }

    pub const fn entry_point(&self) -> GuestImageEntryPoint {
        self.entry_point
    }

    pub fn segments(&self) -> &GuestImageSegments {
        &self.segments
    }

    pub fn code_segment(&self) -> Option<GuestImageSegment> {
        self.segments.code_segment()
    }

    pub const fn metadata(&self) -> &GuestImageMetadata {
        &self.metadata
    }

    pub const fn sections(&self) -> &ProgramImageSections {
        self.metadata.sections()
    }

    pub const fn mapped_bytes_source(&self) -> GuestImageMappedBytesSource {
        self.metadata.mapped_bytes_source()
    }

    pub const fn mapped_bytes(&self) -> &ProgramImageMappedBytes {
        self.metadata.mapped_bytes()
    }

    pub const fn symbols(&self) -> &ProgramImageSymbols {
        self.metadata.symbols()
    }

    pub const fn imports(&self) -> &ProgramImageImports {
        self.metadata.imports()
    }

    pub const fn relocations(&self) -> &ProgramImageRelocations {
        self.metadata.relocations()
    }

    pub const fn unwind(&self) -> &ProgramUnwindMetadata {
        self.metadata.unwind()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageFormat {
    MachO,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestImageEntryPoint {
    address: X86Va,
}

impl GuestImageEntryPoint {
    pub const fn new(address: X86Va) -> Self {
        Self { address }
    }

    pub const fn address(self) -> X86Va {
        self.address
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GuestImageSegments {
    items: Vec<GuestImageSegment>,
}

impl GuestImageSegments {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_items(items: impl IntoIterator<Item = GuestImageSegment>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    pub fn items(&self) -> &[GuestImageSegment] {
        &self.items
    }

    pub fn code_segment(&self) -> Option<GuestImageSegment> {
        self.items
            .iter()
            .copied()
            .find(|segment| segment.kind() == GuestImageSegmentKind::Code)
    }

    fn contains_entry(&self, entry_point: GuestImageEntryPoint) -> bool {
        self.items
            .iter()
            .any(|segment| segment.contains_entry(entry_point))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestImageSegment {
    kind: GuestImageSegmentKind,
    range: ProgramImageRange,
    source: GuestImageSegmentSource,
    address_space: GuestImageAddressSpace,
}

impl GuestImageSegment {
    pub const fn new(
        kind: GuestImageSegmentKind,
        range: ProgramImageRange,
        source: GuestImageSegmentSource,
        address_space: GuestImageAddressSpace,
    ) -> Self {
        Self {
            kind,
            range,
            source,
            address_space,
        }
    }

    pub const fn kind(self) -> GuestImageSegmentKind {
        self.kind
    }

    pub const fn range(self) -> ProgramImageRange {
        self.range
    }

    pub const fn source(self) -> GuestImageSegmentSource {
        self.source
    }

    pub const fn address_space(self) -> GuestImageAddressSpace {
        self.address_space
    }

    fn contains_entry(self, entry_point: GuestImageEntryPoint) -> bool {
        let address = entry_point.address();
        self.range.start() <= address && address < self.range.end()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageSegmentKind {
    Code,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageSegmentSource {
    LcSegment64FileRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageAddressSpace {
    MachOVirtualAddress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageError {
    MissingCodeSegment,
    MissingMachOExecutableCodeSection,
    AmbiguousMachOExecutableCodeSections,
    MachOExecutableCodeByteLenOverflow,
    MachOExecutableCodeBytesUnavailable(ProgramImageMetadataError),
    EntryOutsideMappedSegments,
}
