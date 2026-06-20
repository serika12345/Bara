use bara_ir::{ProgramImageImports, ProgramImageMappedBytes, ProgramImageRange, X86Va};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImage {
    format: GuestImageFormat,
    entry_point: GuestImageEntryPoint,
    segments: GuestImageSegments,
    mapped_bytes_source: GuestImageMappedBytesSource,
    mapped_bytes: ProgramImageMappedBytes,
    imports: ProgramImageImports,
}

impl GuestImage {
    pub fn mach_o_executable(
        entry_point: GuestImageEntryPoint,
        code_segment: GuestImageSegment,
        mapped_bytes_source: GuestImageMappedBytesSource,
        mapped_bytes: ProgramImageMappedBytes,
        imports: ProgramImageImports,
    ) -> Result<Self, GuestImageError> {
        Self::new(
            GuestImageFormat::MachO,
            entry_point,
            GuestImageSegments::from_items([code_segment]),
            mapped_bytes_source,
            mapped_bytes,
            imports,
        )
    }

    pub fn new(
        format: GuestImageFormat,
        entry_point: GuestImageEntryPoint,
        segments: GuestImageSegments,
        mapped_bytes_source: GuestImageMappedBytesSource,
        mapped_bytes: ProgramImageMappedBytes,
        imports: ProgramImageImports,
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
            mapped_bytes_source,
            mapped_bytes,
            imports,
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

    pub const fn mapped_bytes_source(&self) -> GuestImageMappedBytesSource {
        self.mapped_bytes_source
    }

    pub const fn mapped_bytes(&self) -> &ProgramImageMappedBytes {
        &self.mapped_bytes
    }

    pub const fn imports(&self) -> &ProgramImageImports {
        &self.imports
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
pub enum GuestImageMappedBytesSource {
    ProgramImageMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageError {
    MissingCodeSegment,
    EntryOutsideMappedSegments,
}

#[cfg(test)]
mod tests {
    use super::{
        GuestImage, GuestImageAddressSpace, GuestImageEntryPoint, GuestImageError,
        GuestImageMappedBytesSource, GuestImageSegment, GuestImageSegmentKind,
        GuestImageSegmentSource, GuestImageSegments,
    };
    use bara_ir::{
        ExternalSymbolId, ExternalSymbolImport, ProgramImageImport, ProgramImageImports,
        ProgramImageMappedByteSegment, ProgramImageMappedBytes, ProgramImageRange, X86Va,
    };

    fn image_range(start: u64, end: u64) -> ProgramImageRange {
        ProgramImageRange::new(X86Va::new(start), X86Va::new(end))
            .expect("test image range is valid")
    }

    fn code_segment() -> GuestImageSegment {
        GuestImageSegment::new(
            GuestImageSegmentKind::Code,
            image_range(0x1_0000_0000, 0x1_0000_1000),
            GuestImageSegmentSource::LcSegment64FileRange,
            GuestImageAddressSpace::MachOVirtualAddress,
        )
    }

    fn mapped_bytes() -> ProgramImageMappedBytes {
        let bytes = vec![42, 0, 0, 0, 0, 0, 0, 0];
        let segment =
            ProgramImageMappedByteSegment::new(image_range(0x1_0000_0000, 0x1_0000_0008), bytes)
                .expect("test mapped byte segment is valid");
        ProgramImageMappedBytes::from_segments([segment])
    }

    fn imports() -> ProgramImageImports {
        let import = ExternalSymbolImport::unresolved(ExternalSymbolId::new(7));
        ProgramImageImports::from_items([ProgramImageImport::new(import)])
    }

    #[test]
    fn mach_o_guest_image_exposes_runtime_facing_mapping_shell() {
        let image = GuestImage::mach_o_executable(
            GuestImageEntryPoint::new(X86Va::new(0x1_0000_0010)),
            code_segment(),
            GuestImageMappedBytesSource::ProgramImageMetadata,
            mapped_bytes(),
            imports(),
        )
        .expect("entry is inside code segment");

        assert_eq!(image.entry_point().address(), X86Va::new(0x1_0000_0010));
        assert_eq!(image.code_segment(), Some(code_segment()));
        assert_eq!(
            image.mapped_bytes_source(),
            GuestImageMappedBytesSource::ProgramImageMetadata
        );
        assert_eq!(
            image.mapped_bytes().read_u64_le(X86Va::new(0x1_0000_0000)),
            Some(42)
        );
        assert_eq!(image.imports().items().len(), 1);
        assert_eq!(
            image.imports().items()[0].import(),
            ExternalSymbolImport::unresolved(ExternalSymbolId::new(7))
        );
    }

    #[test]
    fn guest_image_rejects_entry_outside_mapped_segments() {
        assert_eq!(
            GuestImage::mach_o_executable(
                GuestImageEntryPoint::new(X86Va::new(0x1_0000_1000)),
                code_segment(),
                GuestImageMappedBytesSource::ProgramImageMetadata,
                mapped_bytes(),
                imports(),
            ),
            Err(GuestImageError::EntryOutsideMappedSegments)
        );
    }

    #[test]
    fn guest_image_rejects_missing_code_segment() {
        assert_eq!(
            GuestImage::new(
                super::GuestImageFormat::MachO,
                GuestImageEntryPoint::new(X86Va::new(0x1_0000_0010)),
                GuestImageSegments::empty(),
                GuestImageMappedBytesSource::ProgramImageMetadata,
                mapped_bytes(),
                imports(),
            ),
            Err(GuestImageError::MissingCodeSegment)
        );
    }
}
