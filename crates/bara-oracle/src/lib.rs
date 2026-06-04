pub mod binary_format;
pub mod compare;
pub mod executable_manifest;
pub mod json;
pub mod observation;
pub mod report;
pub mod testcase;

pub use binary_format::{
    mach_o_entry_function_test_case, mach_o_entry_function_test_case_with_host_traps,
    mach_o_executable_image_entry_function, mach_o_executable_image_entry_function_with_host_traps,
    materialize_mach_o_executable_image, plan_mach_o_executable_image, probe_public_binary_format,
    BinaryFileBytes, BinaryFormat, BinaryFormatProbeError, BinaryFormatProbeMetadata,
    BinaryFormatProbeReport, BinaryFormatProbeStatus, BinaryInput, BinaryInputError,
    MachOEntryFunctionTestCaseError, MachOEntryPointCommandMetadata, MachOEntryPointFileOffset,
    MachOEntryPointSegmentOffset, MachOEntryPointStackSize, MachOExecutableImageConversion,
    MachOExecutableImageConversionBlocker, MachOExecutableImageConversionStatus,
    MachOExecutableImageEntryFunctionError, MachOExecutableImageMaterializationError,
    MachOExecutableImagePlan, MachOExecutableImagePlanError, MachOFileType,
    MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary, MachOLoadCommandType,
    MachOLoadCommands, MachOMetadata, MachOSegmentCommandHeaderMetadata, MachOSegmentFileOffset,
    MachOSegmentFileRange, MachOSegmentFileSize, MachOSegmentName, MachOSegmentVmAddr,
    RecognizedMachOEntryPointCommand, RecognizedMachOSegmentCommand, UnsupportedMachOLoadCommand,
};
pub use compare::{compare_observed_results, ComparisonIssue, ComparisonReport};
pub use executable_manifest::{
    executable_manifest_from_json, CodeSegment, ExecutableEntry, ExecutableImage,
    ExecutableImageError, ExecutableManifest, ExecutableManifestJsonError, HostHelperImport,
    HostHelperImportTable, HostHelperImportTableError, HostHelperName, HostHelperResolutionPlan,
    HostHelperSignature, ResolvedHostHelperImport,
};
pub use json::{
    binary_format_probe_report_from_json, binary_format_probe_report_to_json,
    corpus_report_to_json, observed_result_from_json, observed_result_to_json, JsonError,
};
pub use observation::{CaseId, CaseIdError, ExpectedResult, ObservedResult};
pub use report::{CorpusReport, FailureKind, FailureMessage, FixtureOutcome, FixtureReport};
pub use testcase::{
    host_trap_plan_from_json, test_case_from_json, TestCase, TestCaseAbi, TestCaseHostTrapPlan,
    TestCaseInputMemory, TestCaseInputMemoryError, TestCaseJsonError, TestCaseStackSize,
    TestCaseStackSizeError, TestCaseStackState, TestCaseStdoutTrap, TestCaseStdoutTrapError,
    TestCaseU64,
};
