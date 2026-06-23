use bara_ir::{ProgramImageMetadata, ProgramImageRange, ProgramImageSectionKind, X86Va};

use super::{
    GuestImage, GuestImageAddressSpace, GuestImageEntryPoint, GuestImageError, GuestImageImports,
    GuestImageMappedBytes, GuestImageMappedBytesSource, GuestImageMetadata, GuestImageRelocations,
    GuestImageSections, GuestImageSegment, GuestImageSegmentKind, GuestImageSegmentSource,
    GuestImageSymbols, GuestImageUnwindMetadata,
};

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

    pub fn executable_mapping(&self) -> MachOExecutableImageMapping {
        MachOExecutableImageMapping::new(
            self.code_segment,
            self.entry_point(),
            self.metadata().mapped_bytes_value().clone(),
        )
    }

    pub fn executable_metadata(&self) -> MachOExecutableImageMetadata {
        MachOExecutableImageMetadata::new(self.metadata().clone())
    }

    pub const fn metadata(&self) -> &GuestImageMetadata {
        self.guest_image.metadata()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOExecutableImageMetadata {
    metadata: GuestImageMetadata,
}

impl MachOExecutableImageMetadata {
    pub const fn new(metadata: GuestImageMetadata) -> Self {
        Self { metadata }
    }

    pub const fn mapped_bytes(&self) -> &GuestImageMappedBytes {
        self.metadata.mapped_bytes_value()
    }

    pub const fn sections(&self) -> &GuestImageSections {
        self.metadata.sections_value()
    }

    pub const fn symbols(&self) -> &GuestImageSymbols {
        self.metadata.symbols_value()
    }

    pub const fn relocations(&self) -> &GuestImageRelocations {
        self.metadata.relocations_value()
    }

    pub const fn imports(&self) -> &GuestImageImports {
        self.metadata.imports_value()
    }

    pub const fn unwind(&self) -> &GuestImageUnwindMetadata {
        self.metadata.unwind_value()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOExecutableImageMapping {
    code_segment: MachOExecutableCodeSegment,
    entry_point: MachOExecutableEntryPoint,
    mapped_bytes: GuestImageMappedBytes,
}

impl MachOExecutableImageMapping {
    pub fn new(
        code_segment: MachOExecutableCodeSegment,
        entry_point: MachOExecutableEntryPoint,
        mapped_bytes: GuestImageMappedBytes,
    ) -> Self {
        Self {
            code_segment,
            entry_point,
            mapped_bytes,
        }
    }

    pub const fn code_segment(&self) -> MachOExecutableCodeSegment {
        self.code_segment
    }

    pub const fn entry_point(&self) -> MachOExecutableEntryPoint {
        self.entry_point
    }

    pub const fn mapped_bytes(&self) -> &GuestImageMappedBytes {
        &self.mapped_bytes
    }

    pub const fn mapped_bytes_source(&self) -> GuestImageMappedBytesSource {
        self.mapped_bytes.source()
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

    pub(super) const fn guest_image_entry_point(self) -> GuestImageEntryPoint {
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
pub struct MachOExecutableCodeByteLen {
    byte_len: usize,
}

impl MachOExecutableCodeByteLen {
    const fn new(byte_len: usize) -> Self {
        Self { byte_len }
    }

    pub const fn as_usize(self) -> usize {
        self.byte_len
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOExecutableCodeSegment {
    segment: GuestImageSegment,
}

impl MachOExecutableCodeSegment {
    pub const fn new(range: MachOExecutableCodeRange) -> Self {
        Self {
            segment: GuestImageSegment::new(
                GuestImageSegmentKind::Code,
                range.range(),
                GuestImageSegmentSource::LcSegment64FileRange,
                GuestImageAddressSpace::MachOVirtualAddress,
            ),
        }
    }

    pub const fn range(self) -> MachOExecutableCodeRange {
        MachOExecutableCodeRange::new(self.segment.range())
    }

    pub const fn vmaddr(self) -> X86Va {
        self.segment.range().start()
    }

    pub fn byte_len(self) -> Result<MachOExecutableCodeByteLen, GuestImageError> {
        let range = self.segment.range();
        let byte_len = range
            .end()
            .value()
            .checked_sub(range.start().value())
            .ok_or(GuestImageError::MachOExecutableCodeByteLenOverflow)?;
        let byte_len = usize::try_from(byte_len)
            .map_err(|_| GuestImageError::MachOExecutableCodeByteLenOverflow)?;

        Ok(MachOExecutableCodeByteLen::new(byte_len))
    }

    pub const fn source(self) -> GuestImageSegmentSource {
        self.segment.source()
    }

    pub const fn address_space(self) -> GuestImageAddressSpace {
        self.segment.address_space()
    }

    pub(super) const fn guest_image_segment(self) -> GuestImageSegment {
        self.segment
    }
}
