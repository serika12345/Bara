use crate::{
    executable_manifest::{ExecutableImage, ExecutableImageError},
    CaseId, TestCase, TestCaseAbi, TestCaseHostTrapPlan,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachOExecutableImageEntryFunctionError {
    ExecutableImage(ExecutableImageError),
}

pub fn mach_o_executable_image_entry_function(
    case_id: CaseId,
    image: &ExecutableImage,
) -> Result<TestCase, MachOExecutableImageEntryFunctionError> {
    mach_o_executable_image_entry_function_with_host_traps(
        case_id,
        image,
        TestCaseHostTrapPlan::none(),
    )
}

pub fn mach_o_executable_image_entry_function_with_host_traps(
    case_id: CaseId,
    image: &ExecutableImage,
    host_trap_plan: TestCaseHostTrapPlan,
) -> Result<TestCase, MachOExecutableImageEntryFunctionError> {
    let entry_bytes = image
        .entry_function_bytes()
        .map_err(MachOExecutableImageEntryFunctionError::ExecutableImage)?;

    Ok(TestCase::with_host_traps(
        case_id,
        entry_bytes,
        TestCaseAbi::NoArgsU64,
        host_trap_plan,
    ))
}
