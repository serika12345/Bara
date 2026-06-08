mod input;
mod mach_o_entry_function_pipeline;
mod mach_o_executable_image_entry_function;
mod output;

pub use input::{
    probe_public_binary_format, BinaryFileBytes, BinaryFormat, BinaryFormatProbeError,
    BinaryFormatProbeMetadata, BinaryFormatProbeReport, BinaryFormatProbeStatus, BinaryInput,
    BinaryInputError, MachOEntryPointCommandMetadata, MachOEntryPointFileOffset,
    MachOEntryPointStackSize, MachOExecutableImageConversion,
    MachOExecutableImageConversionBlocker, MachOExecutableImageConversionStatus, MachOFileType,
    MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary, MachOLoadCommandType,
    MachOLoadCommands, MachOMetadata, MachOSegmentCommandHeaderMetadata, MachOSegmentFileOffset,
    MachOSegmentFileSize, MachOSegmentName, MachOSegmentVmAddr, RecognizedMachOEntryPointCommand,
    RecognizedMachOSegmentCommand, UnsupportedMachOLoadCommand,
};
pub use mach_o_entry_function_pipeline::{
    mach_o_entry_function_test_case, mach_o_entry_function_test_case_with_embedded_host_traps,
    mach_o_entry_function_test_case_with_host_traps, MachOEntryFunctionTestCaseError,
};
pub use mach_o_executable_image_entry_function::{
    mach_o_executable_image_entry_function, mach_o_executable_image_entry_function_with_host_traps,
    MachOExecutableImageEntryFunctionError,
};
pub use output::{
    materialize_mach_o_executable_image, plan_mach_o_executable_image,
    MachOEntryPointSegmentOffset, MachOExecutableImageMaterializationError,
    MachOExecutableImagePlan, MachOExecutableImagePlanError, MachOSegmentFileRange,
};

#[cfg(test)]
mod tests;
