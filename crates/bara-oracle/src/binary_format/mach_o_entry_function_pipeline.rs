use crate::{CaseId, TestCase, TestCaseHostTrapPlan};

use super::{
    input::BinaryInput,
    mach_o_executable_image_entry_function::{
        mach_o_executable_image_entry_function_with_host_traps,
        MachOExecutableImageEntryFunctionError,
    },
    mach_o_executable_image_materialization::{
        materialize_mach_o_executable_image, MachOExecutableImageMaterializationError,
    },
    mach_o_executable_image_plan::{plan_mach_o_executable_image, MachOExecutableImagePlanError},
    probe::{probe_public_binary_format, BinaryFormatProbeError},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachOEntryFunctionTestCaseError {
    Probe(BinaryFormatProbeError),
    Plan(MachOExecutableImagePlanError),
    Materialization(MachOExecutableImageMaterializationError),
    EntryFunction(MachOExecutableImageEntryFunctionError),
}

pub fn mach_o_entry_function_test_case(
    case_id: CaseId,
    input: &BinaryInput,
) -> Result<TestCase, MachOEntryFunctionTestCaseError> {
    mach_o_entry_function_test_case_with_host_traps(case_id, input, TestCaseHostTrapPlan::none())
}

pub fn mach_o_entry_function_test_case_with_host_traps(
    case_id: CaseId,
    input: &BinaryInput,
    host_trap_plan: TestCaseHostTrapPlan,
) -> Result<TestCase, MachOEntryFunctionTestCaseError> {
    let report =
        probe_public_binary_format(input).map_err(MachOEntryFunctionTestCaseError::Probe)?;
    let conversion = report
        .metadata()
        .mach_o_metadata()
        .executable_image_conversion();
    let plan =
        plan_mach_o_executable_image(conversion).map_err(MachOEntryFunctionTestCaseError::Plan)?;
    let image = materialize_mach_o_executable_image(input, &plan)
        .map_err(MachOEntryFunctionTestCaseError::Materialization)?;

    mach_o_executable_image_entry_function_with_host_traps(case_id, &image, host_trap_plan)
        .map_err(MachOEntryFunctionTestCaseError::EntryFunction)
}
