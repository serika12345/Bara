use bara_oracle::MachOEntryFunctionInput;
use bara_runtime::{
    GuestImageAddressSpace, GuestImageError, GuestImageMappedBytesSource, GuestImageSegmentSource,
    MachOExecutableEntryPoint, MachOExecutableImageMapping, MachOImage,
};
use serde::Serialize;

use super::report::B8DebugStageStatus;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugGuestImageMappingReport {
    status: B8DebugStageStatus,
    segment_source: B8DebugGuestImageSegmentSource,
    address_space: B8DebugGuestImageAddressSpace,
    code_segment_vmaddr: u64,
    code_segment_byte_len: usize,
    entry_pc: u64,
    mapped_bytes_source: B8DebugGuestImageMappedBytesSource,
}

impl B8DebugGuestImageMappingReport {
    pub(super) fn from_entry_input(
        entry_input: &MachOEntryFunctionInput,
    ) -> Result<Self, B8DebugGuestImageMappingError> {
        let mach_o_image = mach_o_image_from_entry_input(entry_input)?;
        Self::from_mach_o_mapping(mach_o_image.executable_mapping())
    }

    fn from_mach_o_mapping(
        mapping: MachOExecutableImageMapping,
    ) -> Result<Self, B8DebugGuestImageMappingError> {
        let code_segment = mapping.code_segment();
        let code_segment_byte_len = code_segment
            .byte_len()
            .map_err(B8DebugGuestImageMappingError::GuestImage)?;

        Ok(Self {
            status: B8DebugStageStatus::Executed,
            segment_source: code_segment.source().into(),
            address_space: code_segment.address_space().into(),
            code_segment_vmaddr: code_segment.vmaddr().value(),
            code_segment_byte_len: code_segment_byte_len.as_usize(),
            entry_pc: mapping.entry_point().address().value(),
            mapped_bytes_source: mapping.mapped_bytes_source().into(),
        })
    }
}

fn mach_o_image_from_entry_input(
    entry_input: &MachOEntryFunctionInput,
) -> Result<MachOImage, B8DebugGuestImageMappingError> {
    MachOImage::executable_from_program_image_metadata(
        MachOExecutableEntryPoint::new(entry_input.executable_image().entry().offset()),
        entry_input.program_image_metadata(),
    )
    .map_err(B8DebugGuestImageMappingError::GuestImage)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum B8DebugGuestImageMappingError {
    GuestImage(GuestImageError),
}

impl From<GuestImageSegmentSource> for B8DebugGuestImageSegmentSource {
    fn from(source: GuestImageSegmentSource) -> Self {
        match source {
            GuestImageSegmentSource::LcSegment64FileRange => Self::LcSegment64FileRange,
        }
    }
}

impl From<GuestImageAddressSpace> for B8DebugGuestImageAddressSpace {
    fn from(address_space: GuestImageAddressSpace) -> Self {
        match address_space {
            GuestImageAddressSpace::MachOVirtualAddress => Self::MachOVirtualAddress,
        }
    }
}

impl From<GuestImageMappedBytesSource> for B8DebugGuestImageMappedBytesSource {
    fn from(source: GuestImageMappedBytesSource) -> Self {
        match source {
            GuestImageMappedBytesSource::ProgramImageMetadata => Self::ProgramImageMetadata,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugGuestImageSegmentSource {
    LcSegment64FileRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugGuestImageAddressSpace {
    MachOVirtualAddress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugGuestImageMappedBytesSource {
    ProgramImageMetadata,
}

#[cfg(test)]
mod tests {
    use bara_ir::{
        ProgramImageImports, ProgramImageMappedByteSegment, ProgramImageMappedBytes,
        ProgramImageMetadata, ProgramImageRange, ProgramImageRelocations, ProgramImageSection,
        ProgramImageSectionKind, ProgramImageSections, ProgramImageSymbols, ProgramUnwindMetadata,
        X86Va,
    };

    use super::*;

    fn image_range(start: u64, end: u64) -> ProgramImageRange {
        ProgramImageRange::new(X86Va::new(start), X86Va::new(end))
            .expect("test image range is valid")
    }

    fn program_image_metadata() -> ProgramImageMetadata {
        let code_range = image_range(0x1_0000_0000, 0x1_0000_0010);
        let mapped_segment = ProgramImageMappedByteSegment::new(code_range, vec![0x90; 0x10])
            .expect("test mapped byte segment is valid");

        ProgramImageMetadata::new_with_mapped_bytes(
            ProgramImageSections::from_items([ProgramImageSection::new(
                ProgramImageSectionKind::Code,
                code_range,
            )]),
            ProgramImageMappedBytes::from_segments([mapped_segment]),
            ProgramImageSymbols::empty(),
            ProgramImageRelocations::empty(),
            ProgramImageImports::empty(),
            ProgramUnwindMetadata::empty(),
        )
    }

    #[test]
    fn image_mapping_report_uses_typed_mach_o_code_segment() {
        let image = MachOImage::executable_from_program_image_metadata(
            MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0008)),
            &program_image_metadata(),
        )
        .expect("test Mach-O image is valid");

        let report =
            B8DebugGuestImageMappingReport::from_mach_o_mapping(image.executable_mapping())
                .expect("test mapping report is valid");

        assert_eq!(report.status, B8DebugStageStatus::Executed);
        assert_eq!(
            report.segment_source,
            B8DebugGuestImageSegmentSource::LcSegment64FileRange
        );
        assert_eq!(
            report.address_space,
            B8DebugGuestImageAddressSpace::MachOVirtualAddress
        );
        assert_eq!(report.code_segment_vmaddr, 0x1_0000_0000);
        assert_eq!(report.code_segment_byte_len, 0x10);
        assert_eq!(report.entry_pc, 0x1_0000_0008);
        assert_eq!(
            report.mapped_bytes_source,
            B8DebugGuestImageMappedBytesSource::ProgramImageMetadata
        );
    }
}
