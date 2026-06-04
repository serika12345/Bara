use crate::{
    executable_manifest::{ExecutableImage, ExecutableImageError},
    CaseId, TestCase, TestCaseAbi,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachOExecutableImageEntryFunctionError {
    ExecutableImage(ExecutableImageError),
}

pub fn mach_o_executable_image_entry_function(
    case_id: CaseId,
    image: &ExecutableImage,
) -> Result<TestCase, MachOExecutableImageEntryFunctionError> {
    let entry_bytes = image
        .entry_function_bytes()
        .map_err(MachOExecutableImageEntryFunctionError::ExecutableImage)?;

    Ok(TestCase::new(case_id, entry_bytes, TestCaseAbi::NoArgsU64))
}
