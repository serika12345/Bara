use bara_oracle::MachOEntryFunctionInput;
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
    pub(super) fn from_entry_input(entry_input: &MachOEntryFunctionInput) -> Self {
        let code = entry_input.executable_image().code_segment().x86_bytes();
        Self {
            status: B8DebugStageStatus::Executed,
            segment_source: B8DebugGuestImageSegmentSource::LcSegment64FileRange,
            address_space: B8DebugGuestImageAddressSpace::MachOVirtualAddress,
            code_segment_vmaddr: code.entry().value(),
            code_segment_byte_len: code.bytes().len(),
            entry_pc: entry_input.executable_image().entry().offset().value(),
            mapped_bytes_source: B8DebugGuestImageMappedBytesSource::ProgramImageMetadata,
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
