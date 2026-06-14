use bara_oracle::{BinaryFormatProbeReport, BinaryInput, MachOEntryFunctionInput};
use serde::Serialize;

use super::import_boundary::B8DebugImportBoundaryReport;
use super::report::{B8DebugDecodeReport, B8DebugEntrySource, B8DebugStageStatus};
use super::B8DebugHelperBoundaryRequestReport;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugLoaderPlanReport {
    schema: &'static str,
    source: &'static str,
    status: B8DebugStageStatus,
    input_metadata: B8DebugLoaderInputMetadata,
    image_mapping: B8DebugLoaderImageMappingReport,
    relocation_binding: B8DebugLoaderDeferredStepReport,
    import_boundary: B8DebugImportBoundaryReport,
    entry_source_for_this_bundle: B8DebugEntrySource,
    next_entry_source: B8DebugLoaderNextEntrySource,
}

impl B8DebugLoaderPlanReport {
    pub(super) fn real_lc_main_attempted(
        input: &BinaryInput,
        entry_input: &MachOEntryFunctionInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
    ) -> Self {
        let code = entry_input.executable_image().code_segment().x86_bytes();
        Self {
            schema: "b8_debug_loader_plan_v0",
            source: "bara_runtime_user_space_launch_plan",
            status: B8DebugStageStatus::Executed,
            input_metadata: B8DebugLoaderInputMetadata::PublicMachOProbe,
            image_mapping: B8DebugLoaderImageMappingReport {
                status: B8DebugStageStatus::Executed,
                segment_source: B8DebugLoaderSegmentSource::LcSegment64FileRange,
                address_space: B8DebugLoaderAddressSpace::MachOVirtualAddress,
                code_segment_vmaddr: code.entry().value(),
                code_segment_byte_len: code.bytes().len(),
                entry_pc: entry_input.executable_image().entry().offset().value(),
                mapped_bytes_source: B8DebugLoaderMappedBytesSource::ProgramImageMetadata,
            },
            relocation_binding: B8DebugLoaderDeferredStepReport {
                status: B8DebugStageStatus::Skipped,
                reason: "public rebase/bind/import application is represented as import_boundary and remains blocked until chained fixups are decoded",
                next_action: B8DebugLoaderDeferredAction::ResolvePublicRebaseBindImports,
            },
            import_boundary: B8DebugImportBoundaryReport::from_probe_and_decode_report(
                input,
                input_probe,
                decode_report,
                code,
                entry_input.program_image_metadata(),
            ),
            entry_source_for_this_bundle: B8DebugEntrySource::PublicLcMainEntryoff,
            next_entry_source: B8DebugLoaderNextEntrySource::FirstUnsupportedBoundary,
        }
    }

    pub(super) fn helper_boundary_request(&self) -> B8DebugHelperBoundaryRequestReport {
        self.import_boundary.helper_boundary_request()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderInputMetadata {
    PublicMachOProbe,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLoaderImageMappingReport {
    status: B8DebugStageStatus,
    segment_source: B8DebugLoaderSegmentSource,
    address_space: B8DebugLoaderAddressSpace,
    code_segment_vmaddr: u64,
    code_segment_byte_len: usize,
    entry_pc: u64,
    mapped_bytes_source: B8DebugLoaderMappedBytesSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderSegmentSource {
    LcSegment64FileRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderAddressSpace {
    MachOVirtualAddress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderMappedBytesSource {
    ProgramImageMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLoaderDeferredStepReport {
    status: B8DebugStageStatus,
    reason: &'static str,
    next_action: B8DebugLoaderDeferredAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderDeferredAction {
    ResolvePublicRebaseBindImports,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderNextEntrySource {
    FirstUnsupportedBoundary,
}
