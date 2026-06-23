use bara_ir::{
    ProgramImageImports, ProgramImageMappedBytes, ProgramImageMetadata, ProgramImageRange,
    ProgramImageRelocations, ProgramImageSectionKind, ProgramImageSections, ProgramImageSymbols,
    ProgramUnwindMetadata, X86Va,
};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOImage {
    guest_image: GuestImage,
    code_segment: MachOExecutableCodeSegment,
}

impl MachOImage {
    pub fn executable(
        entry_point: MachOExecutableEntryPoint,
        code_segment: MachOExecutableCodeSegment,
        metadata: GuestImageMetadata,
    ) -> Result<Self, GuestImageError> {
        Ok(Self {
            guest_image: GuestImage::mach_o_executable(
                entry_point.guest_image_entry_point(),
                code_segment.guest_image_segment(),
                metadata,
            )?,
            code_segment,
        })
    }

    pub fn executable_from_code_range(
        entry_point: MachOExecutableEntryPoint,
        code_range: MachOExecutableCodeRange,
        metadata: GuestImageMetadata,
    ) -> Result<Self, GuestImageError> {
        Self::executable(
            entry_point,
            MachOExecutableCodeSegment::new(code_range),
            metadata,
        )
    }

    pub fn executable_from_program_image_metadata(
        entry_point: MachOExecutableEntryPoint,
        metadata: &ProgramImageMetadata,
    ) -> Result<Self, GuestImageError> {
        let code_range = MachOExecutableCodeRange::from_program_image_metadata(metadata)?;
        Self::executable_from_code_range(
            entry_point,
            code_range,
            GuestImageMetadata::from_program_image_metadata(metadata),
        )
    }

    pub const fn guest_image(&self) -> &GuestImage {
        &self.guest_image
    }

    pub const fn entry_point(&self) -> MachOExecutableEntryPoint {
        MachOExecutableEntryPoint::new(self.guest_image.entry_point().address())
    }

    pub const fn code_segment(&self) -> MachOExecutableCodeSegment {
        self.code_segment
    }

    pub const fn metadata(&self) -> &GuestImageMetadata {
        self.guest_image.metadata()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOExecutableEntryPoint {
    address: X86Va,
}

impl MachOExecutableEntryPoint {
    pub const fn new(address: X86Va) -> Self {
        Self { address }
    }

    pub const fn address(self) -> X86Va {
        self.address
    }

    const fn guest_image_entry_point(self) -> GuestImageEntryPoint {
        GuestImageEntryPoint::new(self.address)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOExecutableCodeRange {
    range: ProgramImageRange,
}

impl MachOExecutableCodeRange {
    pub const fn new(range: ProgramImageRange) -> Self {
        Self { range }
    }

    pub fn from_program_image_metadata(
        metadata: &ProgramImageMetadata,
    ) -> Result<Self, GuestImageError> {
        let mut code_sections = metadata
            .sections()
            .items()
            .iter()
            .filter(|section| section.kind() == ProgramImageSectionKind::Code);
        let code_section = code_sections
            .next()
            .ok_or(GuestImageError::MissingMachOExecutableCodeSection)?;
        if code_sections.next().is_some() {
            return Err(GuestImageError::AmbiguousMachOExecutableCodeSections);
        }

        Ok(Self::new(code_section.range()))
    }

    pub const fn range(self) -> ProgramImageRange {
        self.range
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOExecutableCodeSegment {
    segment: GuestImageSegment,
}

impl MachOExecutableCodeSegment {
    pub const fn new(range: MachOExecutableCodeRange) -> Self {
        Self {
            segment: GuestImageSegment::mach_o_executable_code(range),
        }
    }

    pub const fn range(self) -> MachOExecutableCodeRange {
        MachOExecutableCodeRange::new(self.segment.range())
    }

    pub const fn source(self) -> GuestImageSegmentSource {
        self.segment.source()
    }

    pub const fn address_space(self) -> GuestImageAddressSpace {
        self.segment.address_space()
    }

    const fn guest_image_segment(self) -> GuestImageSegment {
        self.segment
    }
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
    pub const fn mach_o_executable_code(range: MachOExecutableCodeRange) -> Self {
        Self::new(
            GuestImageSegmentKind::Code,
            range.range(),
            GuestImageSegmentSource::LcSegment64FileRange,
            GuestImageAddressSpace::MachOVirtualAddress,
        )
    }

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageMappedBytes {
    source: GuestImageMappedBytesSource,
    payload: ProgramImageMappedBytes,
}

impl GuestImageMappedBytes {
    pub const fn new(
        source: GuestImageMappedBytesSource,
        payload: ProgramImageMappedBytes,
    ) -> Self {
        Self { source, payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(
            GuestImageMappedBytesSource::ProgramImageMetadata,
            metadata.mapped_bytes().clone(),
        )
    }

    pub const fn source(&self) -> GuestImageMappedBytesSource {
        self.source
    }

    pub const fn payload(&self) -> &ProgramImageMappedBytes {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageMetadata {
    mapped_bytes: GuestImageMappedBytes,
    sections: GuestImageSections,
    symbols: ProgramImageSymbols,
    relocations: ProgramImageRelocations,
    imports: ProgramImageImports,
    unwind: ProgramUnwindMetadata,
}

impl GuestImageMetadata {
    pub const fn new(
        mapped_bytes: GuestImageMappedBytes,
        sections: GuestImageSections,
        symbols: ProgramImageSymbols,
        relocations: ProgramImageRelocations,
        imports: ProgramImageImports,
        unwind: ProgramUnwindMetadata,
    ) -> Self {
        Self {
            mapped_bytes,
            sections,
            symbols,
            relocations,
            imports,
            unwind,
        }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(
            GuestImageMappedBytes::from_program_image_metadata(metadata),
            GuestImageSections::from_program_image_metadata(metadata),
            metadata.symbols().clone(),
            metadata.relocations().clone(),
            metadata.imports().clone(),
            metadata.unwind().clone(),
        )
    }

    pub const fn mapped_bytes_source(&self) -> GuestImageMappedBytesSource {
        self.mapped_bytes.source()
    }

    pub const fn sections(&self) -> &ProgramImageSections {
        self.sections.payload()
    }

    pub const fn mapped_bytes(&self) -> &ProgramImageMappedBytes {
        self.mapped_bytes.payload()
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageSections {
    payload: ProgramImageSections,
}

impl GuestImageSections {
    pub const fn new(payload: ProgramImageSections) -> Self {
        Self { payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(metadata.sections().clone())
    }

    pub const fn payload(&self) -> &ProgramImageSections {
        &self.payload
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageError {
    MissingCodeSegment,
    MissingMachOExecutableCodeSection,
    AmbiguousMachOExecutableCodeSections,
    EntryOutsideMappedSegments,
}

#[cfg(test)]
mod tests {
    use super::{
        GuestImage, GuestImageAddressSpace, GuestImageEntryPoint, GuestImageError,
        GuestImageFormat, GuestImageMappedBytes, GuestImageMappedBytesSource, GuestImageMetadata,
        GuestImageSections, GuestImageSegment, GuestImageSegmentKind, GuestImageSegmentSource,
        GuestImageSegments, MachOExecutableCodeRange, MachOExecutableCodeSegment,
        MachOExecutableEntryPoint, MachOImage,
    };
    use bara_ir::{
        ExternalSymbolId, ExternalSymbolImport, ProgramImageImport, ProgramImageImports,
        ProgramImageMappedByteSegment, ProgramImageMappedBytes, ProgramImageMetadata,
        ProgramImageRange, ProgramImageRelocation, ProgramImageRelocationTarget,
        ProgramImageRelocations, ProgramImageSection, ProgramImageSectionKind,
        ProgramImageSections, ProgramImageSymbol, ProgramImageSymbols, ProgramUnwindEntry,
        ProgramUnwindMetadata, X86Va,
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

    fn mach_o_code_segment() -> MachOExecutableCodeSegment {
        MachOExecutableCodeSegment::new(MachOExecutableCodeRange::new(image_range(
            0x1_0000_0000,
            0x1_0000_1000,
        )))
    }

    fn mapped_bytes() -> ProgramImageMappedBytes {
        let bytes = vec![42, 0, 0, 0, 0, 0, 0, 0];
        let segment =
            ProgramImageMappedByteSegment::new(image_range(0x1_0000_0000, 0x1_0000_0008), bytes)
                .expect("test mapped byte segment is valid");
        ProgramImageMappedBytes::from_segments([segment])
    }

    fn guest_image_mapped_bytes() -> GuestImageMappedBytes {
        GuestImageMappedBytes::new(
            GuestImageMappedBytesSource::ProgramImageMetadata,
            mapped_bytes(),
        )
    }

    fn imports() -> ProgramImageImports {
        let import = ExternalSymbolImport::unresolved(ExternalSymbolId::new(7));
        ProgramImageImports::from_items([ProgramImageImport::new(import)])
    }

    fn relocations() -> ProgramImageRelocations {
        ProgramImageRelocations::from_items([ProgramImageRelocation::new(
            X86Va::new(0x1_0000_0020),
            ProgramImageRelocationTarget::ExternalSymbol(ExternalSymbolId::new(7)),
        )])
    }

    fn sections() -> ProgramImageSections {
        ProgramImageSections::from_items([
            ProgramImageSection::new(
                ProgramImageSectionKind::Code,
                image_range(0x1_0000_0000, 0x1_0000_1000),
            ),
            ProgramImageSection::new(
                ProgramImageSectionKind::ConstData,
                image_range(0x1_0000_2000, 0x1_0000_2010),
            ),
        ])
    }

    fn guest_image_sections() -> GuestImageSections {
        GuestImageSections::new(sections())
    }

    fn symbols() -> ProgramImageSymbols {
        let import = ExternalSymbolImport::unresolved(ExternalSymbolId::new(7));
        ProgramImageSymbols::from_items([ProgramImageSymbol::external_import(import)])
    }

    fn unwind() -> ProgramUnwindMetadata {
        ProgramUnwindMetadata::from_entries([ProgramUnwindEntry::new(image_range(
            0x1_0000_0000,
            0x1_0000_0040,
        ))])
    }

    fn program_image_metadata() -> ProgramImageMetadata {
        ProgramImageMetadata::new_with_mapped_bytes(
            sections(),
            mapped_bytes(),
            symbols(),
            relocations(),
            imports(),
            unwind(),
        )
    }

    fn metadata() -> GuestImageMetadata {
        GuestImageMetadata::new(
            guest_image_mapped_bytes(),
            guest_image_sections(),
            symbols(),
            relocations(),
            imports(),
            unwind(),
        )
    }

    #[test]
    fn mach_o_guest_image_exposes_runtime_facing_mapping_shell() {
        let image = GuestImage::mach_o_executable(
            GuestImageEntryPoint::new(X86Va::new(0x1_0000_0010)),
            code_segment(),
            metadata(),
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
        assert_eq!(image.metadata(), &metadata());
        assert_eq!(image.sections().items().len(), 2);
        assert_eq!(
            image.sections().items()[1].kind(),
            ProgramImageSectionKind::ConstData
        );
        assert_eq!(image.imports().items().len(), 1);
        assert_eq!(
            image.imports().items()[0].import(),
            ExternalSymbolImport::unresolved(ExternalSymbolId::new(7))
        );
        assert_eq!(image.symbols().items().len(), 1);
        assert_eq!(
            image.symbols().items()[0],
            ProgramImageSymbol::external_import(ExternalSymbolImport::unresolved(
                ExternalSymbolId::new(7)
            ))
        );
        assert_eq!(image.relocations().items().len(), 1);
        assert_eq!(
            image.relocations().items()[0].target(),
            ProgramImageRelocationTarget::ExternalSymbol(ExternalSymbolId::new(7))
        );
        assert_eq!(image.unwind().entries().len(), 1);
        assert_eq!(
            image.unwind().entries()[0].range(),
            image_range(0x1_0000_0000, 0x1_0000_0040)
        );
    }

    #[test]
    fn mach_o_image_wraps_runtime_guest_image_shell() {
        let image = MachOImage::executable(
            MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010)),
            mach_o_code_segment(),
            metadata(),
        )
        .expect("entry is inside code segment");

        assert_eq!(image.guest_image().format(), GuestImageFormat::MachO);
        assert_eq!(image.entry_point().address(), X86Va::new(0x1_0000_0010));
        assert_eq!(image.code_segment(), mach_o_code_segment());
        assert_eq!(image.metadata(), &metadata());
        assert_eq!(image.guest_image().metadata(), &metadata());
    }

    #[test]
    fn mach_o_image_builds_executable_code_segment_from_range() {
        let image = MachOImage::executable_from_code_range(
            MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010)),
            MachOExecutableCodeRange::new(image_range(0x1_0000_0000, 0x1_0000_1000)),
            metadata(),
        )
        .expect("entry is inside code segment");

        assert_eq!(image.guest_image().format(), GuestImageFormat::MachO);
        assert_eq!(image.entry_point().address(), X86Va::new(0x1_0000_0010));
        assert_eq!(image.code_segment(), mach_o_code_segment());
        assert_eq!(image.metadata(), &metadata());
    }

    #[test]
    fn mach_o_executable_code_segment_exposes_mapping_identity() {
        let segment = mach_o_code_segment();

        assert_eq!(
            segment.range(),
            MachOExecutableCodeRange::new(image_range(0x1_0000_0000, 0x1_0000_1000))
        );
        assert_eq!(
            segment.source(),
            GuestImageSegmentSource::LcSegment64FileRange
        );
        assert_eq!(
            segment.address_space(),
            GuestImageAddressSpace::MachOVirtualAddress
        );
        assert_eq!(segment.guest_image_segment(), code_segment());
    }

    #[test]
    fn guest_image_mapped_bytes_exposes_source_and_payload() {
        let mapped_bytes = guest_image_mapped_bytes();

        assert_eq!(
            mapped_bytes.source(),
            GuestImageMappedBytesSource::ProgramImageMetadata
        );
        assert_eq!(
            mapped_bytes
                .payload()
                .read_u64_le(X86Va::new(0x1_0000_0000)),
            Some(42)
        );
    }

    #[test]
    fn guest_image_sections_exposes_payload() {
        let sections = guest_image_sections();

        assert_eq!(sections.payload().items().len(), 2);
        assert_eq!(
            sections.payload().items()[0].kind(),
            ProgramImageSectionKind::Code
        );
        assert_eq!(
            sections.payload().items()[1].range(),
            image_range(0x1_0000_2000, 0x1_0000_2010)
        );
    }

    #[test]
    fn mach_o_executable_code_range_exposes_program_image_range() {
        let range = image_range(0x1_0000_0000, 0x1_0000_1000);
        assert_eq!(MachOExecutableCodeRange::new(range).range(), range);
    }

    #[test]
    fn mach_o_executable_entry_point_exposes_typed_address() {
        let entry_point = MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010));

        assert_eq!(entry_point.address(), X86Va::new(0x1_0000_0010));
        assert_eq!(
            entry_point.guest_image_entry_point(),
            GuestImageEntryPoint::new(X86Va::new(0x1_0000_0010))
        );
    }

    #[test]
    fn mach_o_executable_code_range_uses_single_code_section_from_metadata() {
        assert_eq!(
            MachOExecutableCodeRange::from_program_image_metadata(&program_image_metadata()),
            Ok(MachOExecutableCodeRange::new(image_range(
                0x1_0000_0000,
                0x1_0000_1000
            )))
        );
    }

    #[test]
    fn mach_o_executable_code_range_rejects_missing_code_section() {
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            ProgramImageSections::from_items([ProgramImageSection::new(
                ProgramImageSectionKind::ConstData,
                image_range(0x1_0000_2000, 0x1_0000_2010),
            )]),
            mapped_bytes(),
            symbols(),
            relocations(),
            imports(),
            unwind(),
        );

        assert_eq!(
            MachOExecutableCodeRange::from_program_image_metadata(&metadata),
            Err(GuestImageError::MissingMachOExecutableCodeSection)
        );
    }

    #[test]
    fn mach_o_executable_code_range_rejects_ambiguous_code_sections() {
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            ProgramImageSections::from_items([
                ProgramImageSection::new(
                    ProgramImageSectionKind::Code,
                    image_range(0x1_0000_0000, 0x1_0000_1000),
                ),
                ProgramImageSection::new(
                    ProgramImageSectionKind::Code,
                    image_range(0x1_0000_2000, 0x1_0000_3000),
                ),
            ]),
            mapped_bytes(),
            symbols(),
            relocations(),
            imports(),
            unwind(),
        );

        assert_eq!(
            MachOExecutableCodeRange::from_program_image_metadata(&metadata),
            Err(GuestImageError::AmbiguousMachOExecutableCodeSections)
        );
    }

    #[test]
    fn mach_o_image_builds_guest_metadata_from_program_image_metadata() {
        let program_metadata = program_image_metadata();
        let image = MachOImage::executable_from_program_image_metadata(
            MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010)),
            &program_metadata,
        )
        .expect("entry is inside code segment");

        assert_eq!(
            image.guest_image().mapped_bytes_source(),
            GuestImageMappedBytesSource::ProgramImageMetadata
        );
        assert_eq!(image.code_segment(), mach_o_code_segment());
        assert_eq!(image.metadata(), &metadata());
    }

    #[test]
    fn guest_image_rejects_entry_outside_mapped_segments() {
        assert_eq!(
            GuestImage::mach_o_executable(
                GuestImageEntryPoint::new(X86Va::new(0x1_0000_1000)),
                code_segment(),
                metadata(),
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
                metadata(),
            ),
            Err(GuestImageError::MissingCodeSegment)
        );
    }
}
