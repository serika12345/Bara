mod input;
mod mach_o_entry_function_pipeline;
mod mach_o_executable_image_entry_function;
mod output;

pub use input::{
    decode_mach_o_chained_fixups_for_target, probe_public_binary_format,
    resolve_mach_o_symbol_stub_for_target, BinaryFileBytes, BinaryFormat, BinaryFormatProbeError,
    BinaryFormatProbeMetadata, BinaryFormatProbeReport, BinaryFormatProbeStatus, BinaryInput,
    BinaryInputError, MachOChainedFixupTargetAddress, MachOChainedFixupsBlocker,
    MachOChainedFixupsTargetReport, MachOChainedFixupsTargetStatus,
    MachOChainedImportIdentityReport, MachOChainedRebaseTargetIdentityReport,
    MachODyldInfoCommandKind, MachODylibImportCommandKind, MachODylibPath, MachODylibTimestamp,
    MachODylibVersion, MachOEntryPointCommandMetadata, MachOEntryPointFileOffset,
    MachOEntryPointStackSize, MachOExecutableImageConversion,
    MachOExecutableImageConversionBlocker, MachOExecutableImageConversionStatus, MachOFileType,
    MachOIndirectSymbolTableFileOffset, MachOIndirectSymbolTableSlot, MachOLinkeditByteSize,
    MachOLinkeditDataCommandKind, MachOLinkeditDataRange, MachOLinkeditEntryCount,
    MachOLinkeditFileOffset, MachOLoadCommandByteSize, MachOLoadCommandCount,
    MachOLoadCommandSummary, MachOLoadCommandType, MachOLoadCommands, MachOMetadata,
    MachOResolvedStubSymbol, MachOSectionAddress, MachOSectionAlignment, MachOSectionByteSize,
    MachOSectionFileOffset, MachOSectionFlags, MachOSectionMetadata, MachOSectionName,
    MachOSectionRelocationCount, MachOSectionRelocationFileOffset, MachOSectionReserved1,
    MachOSectionReserved2, MachOSectionReserved3, MachOSegmentCommandHeaderMetadata,
    MachOSegmentFileOffset, MachOSegmentFileSize, MachOSegmentName, MachOSegmentVmAddr,
    MachOStubByteSize, MachOStubIndex, MachOStubSymbolName, MachOStubSymbolResolution,
    MachOStubSymbolResolutionBlocker, MachOStubSymbolResolutionStatus, MachOStubVirtualAddress,
    MachOSymbolIndex, RecognizedMachODyldInfoCommand, RecognizedMachODylibImportCommand,
    RecognizedMachODynamicSymbolTableCommand, RecognizedMachOEntryPointCommand,
    RecognizedMachOLinkeditDataCommand, RecognizedMachOSegmentCommand,
    RecognizedMachOSymbolTableCommand, UnsupportedMachOLoadCommand,
};
pub use mach_o_entry_function_pipeline::{
    mach_o_entry_function_input, mach_o_entry_function_input_with_embedded_host_traps,
    mach_o_entry_function_input_with_host_traps, mach_o_entry_function_test_case,
    mach_o_entry_function_test_case_with_embedded_host_traps,
    mach_o_entry_function_test_case_with_host_traps, MachOEntryFunctionInput,
    MachOEntryFunctionTestCaseError,
};
pub use mach_o_executable_image_entry_function::{
    mach_o_executable_image_entry_function, mach_o_executable_image_entry_function_with_host_traps,
    MachOExecutableImageEntryFunctionError,
};
pub use output::{
    materialize_mach_o_executable_image, plan_mach_o_executable_image,
    MachOEntryPointSegmentOffset, MachOEntryPointVirtualAddress,
    MachOExecutableImageMaterializationError, MachOExecutableImagePlan,
    MachOExecutableImagePlanError, MachOSegmentFileRange,
};

#[cfg(test)]
mod tests;
