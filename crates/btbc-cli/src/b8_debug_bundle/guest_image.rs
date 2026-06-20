use bara_ir::{ProgramImageMetadataError, ProgramImageRange};
use bara_oracle::MachOEntryFunctionInput;
use bara_runtime::{
    GuestImage, GuestImageAddressSpace, GuestImageEntryPoint, GuestImageError,
    GuestImageMappedBytesSource, GuestImageMetadata, GuestImageSegment, GuestImageSegmentKind,
    GuestImageSegmentSource,
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
        let guest_image = guest_image_from_entry_input(entry_input)?;
        Self::from_guest_image(&guest_image)
    }

    fn from_guest_image(guest_image: &GuestImage) -> Result<Self, B8DebugGuestImageMappingError> {
        let code_segment = guest_image
            .code_segment()
            .ok_or(B8DebugGuestImageMappingError::MissingCodeSegment)?;
        let code_segment_range = code_segment.range();
        let code_segment_byte_len =
            usize::try_from(code_segment_range.end().value() - code_segment_range.start().value())
                .map_err(|_| B8DebugGuestImageMappingError::AddressOverflow)?;

        Ok(Self {
            status: B8DebugStageStatus::Executed,
            segment_source: code_segment.source().into(),
            address_space: code_segment.address_space().into(),
            code_segment_vmaddr: code_segment_range.start().value(),
            code_segment_byte_len,
            entry_pc: guest_image.entry_point().address().value(),
            mapped_bytes_source: guest_image.mapped_bytes_source().into(),
        })
    }
}

fn guest_image_from_entry_input(
    entry_input: &MachOEntryFunctionInput,
) -> Result<GuestImage, B8DebugGuestImageMappingError> {
    let code = entry_input.executable_image().code_segment().x86_bytes();
    let code_len = u64::try_from(code.bytes().len())
        .map_err(|_| B8DebugGuestImageMappingError::AddressOverflow)?;
    let code_end = code.entry().checked_add(code_len).map_err(|_| {
        B8DebugGuestImageMappingError::ImageMetadata(ProgramImageMetadataError::AddressOverflow)
    })?;
    let code_range = ProgramImageRange::new(code.entry(), code_end)
        .map_err(B8DebugGuestImageMappingError::ImageMetadata)?;
    let code_segment = GuestImageSegment::new(
        GuestImageSegmentKind::Code,
        code_range,
        GuestImageSegmentSource::LcSegment64FileRange,
        GuestImageAddressSpace::MachOVirtualAddress,
    );

    GuestImage::mach_o_executable(
        GuestImageEntryPoint::new(entry_input.executable_image().entry().offset()),
        code_segment,
        GuestImageMetadata::from_program_image_metadata(
            GuestImageMappedBytesSource::ProgramImageMetadata,
            entry_input.program_image_metadata(),
        ),
    )
    .map_err(B8DebugGuestImageMappingError::GuestImage)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum B8DebugGuestImageMappingError {
    AddressOverflow,
    ImageMetadata(ProgramImageMetadataError),
    GuestImage(GuestImageError),
    MissingCodeSegment,
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
