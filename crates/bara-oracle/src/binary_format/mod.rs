mod input;
mod mach_o;
mod mach_o_entry_function_pipeline;
mod mach_o_entry_point_command;
mod mach_o_executable_image_conversion;
mod mach_o_executable_image_entry_function;
mod mach_o_executable_image_materialization;
mod mach_o_executable_image_plan;
mod mach_o_load_command;
mod mach_o_segment_command;
mod probe;

pub use input::{BinaryFileBytes, BinaryInput, BinaryInputError};
pub use mach_o::{MachOFileType, MachOLoadCommands, MachOMetadata};
pub use mach_o_entry_function_pipeline::{
    mach_o_entry_function_test_case, mach_o_entry_function_test_case_with_host_traps,
    MachOEntryFunctionTestCaseError,
};
pub use mach_o_entry_point_command::{
    MachOEntryPointCommandMetadata, MachOEntryPointFileOffset, MachOEntryPointStackSize,
};
pub use mach_o_executable_image_conversion::{
    MachOExecutableImageConversion, MachOExecutableImageConversionBlocker,
    MachOExecutableImageConversionStatus,
};
pub use mach_o_executable_image_entry_function::{
    mach_o_executable_image_entry_function, mach_o_executable_image_entry_function_with_host_traps,
    MachOExecutableImageEntryFunctionError,
};
pub use mach_o_executable_image_materialization::{
    materialize_mach_o_executable_image, MachOExecutableImageMaterializationError,
};
pub use mach_o_executable_image_plan::{
    plan_mach_o_executable_image, MachOEntryPointSegmentOffset, MachOExecutableImagePlan,
    MachOExecutableImagePlanError, MachOSegmentFileRange,
};
pub use mach_o_load_command::{
    MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary, MachOLoadCommandType,
    RecognizedMachOEntryPointCommand, RecognizedMachOSegmentCommand, UnsupportedMachOLoadCommand,
};
pub use mach_o_segment_command::{
    MachOSegmentCommandHeaderMetadata, MachOSegmentFileOffset, MachOSegmentFileSize,
    MachOSegmentName, MachOSegmentVmAddr,
};
pub use probe::{
    probe_public_binary_format, BinaryFormat, BinaryFormatProbeError, BinaryFormatProbeMetadata,
    BinaryFormatProbeReport, BinaryFormatProbeStatus,
};

#[cfg(test)]
mod tests;
