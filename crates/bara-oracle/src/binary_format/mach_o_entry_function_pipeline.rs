use crate::{
    executable_manifest::ExecutableImage, CaseId, TestCase, TestCaseHostTrapPlan,
    TestCaseStackSize, TestCaseStackState, TestCaseStdoutTrap, TestCaseStdoutTrapError,
};

use super::{
    mach_o_executable_image_entry_function::{
        mach_o_executable_image_entry_function_with_host_traps_and_stack_state,
        MachOExecutableImageEntryFunctionError,
    },
    materialize_mach_o_executable_image, plan_mach_o_executable_image, probe_public_binary_format,
    BinaryFormatProbeError, BinaryInput, MachOExecutableImageConversion,
    MachOExecutableImageMaterializationError, MachOExecutableImagePlanError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachOEntryFunctionTestCaseError {
    Probe(BinaryFormatProbeError),
    Plan(MachOExecutableImagePlanError),
    Materialization(MachOExecutableImageMaterializationError),
    EntryFunction(MachOExecutableImageEntryFunctionError),
    EmbeddedStdoutTrap(TestCaseStdoutTrapError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOEntryFunctionInput {
    test_case: TestCase,
    executable_image: ExecutableImage,
}

impl MachOEntryFunctionInput {
    const fn new(test_case: TestCase, executable_image: ExecutableImage) -> Self {
        Self {
            test_case,
            executable_image,
        }
    }

    pub const fn test_case(&self) -> &TestCase {
        &self.test_case
    }

    pub const fn executable_image(&self) -> &ExecutableImage {
        &self.executable_image
    }

    pub fn into_test_case(self) -> TestCase {
        self.test_case
    }
}

pub fn mach_o_entry_function_input(
    case_id: CaseId,
    input: &BinaryInput,
) -> Result<MachOEntryFunctionInput, MachOEntryFunctionTestCaseError> {
    mach_o_entry_function_input_with_host_traps(case_id, input, TestCaseHostTrapPlan::none())
}

pub fn mach_o_entry_function_input_with_host_traps(
    case_id: CaseId,
    input: &BinaryInput,
    host_trap_plan: TestCaseHostTrapPlan,
) -> Result<MachOEntryFunctionInput, MachOEntryFunctionTestCaseError> {
    mach_o_entry_function_input_with_host_trap_plan(case_id, input, move |_| Ok(host_trap_plan))
}

pub fn mach_o_entry_function_input_with_embedded_host_traps(
    case_id: CaseId,
    input: &BinaryInput,
) -> Result<MachOEntryFunctionInput, MachOEntryFunctionTestCaseError> {
    mach_o_entry_function_input_with_host_trap_plan(
        case_id,
        input,
        testcase_host_trap_plan_from_embedded_stdout_metadata,
    )
}

pub fn mach_o_entry_function_test_case(
    case_id: CaseId,
    input: &BinaryInput,
) -> Result<TestCase, MachOEntryFunctionTestCaseError> {
    mach_o_entry_function_input(case_id, input).map(MachOEntryFunctionInput::into_test_case)
}

pub fn mach_o_entry_function_test_case_with_host_traps(
    case_id: CaseId,
    input: &BinaryInput,
    host_trap_plan: TestCaseHostTrapPlan,
) -> Result<TestCase, MachOEntryFunctionTestCaseError> {
    mach_o_entry_function_input_with_host_traps(case_id, input, host_trap_plan)
        .map(MachOEntryFunctionInput::into_test_case)
}

pub fn mach_o_entry_function_test_case_with_embedded_host_traps(
    case_id: CaseId,
    input: &BinaryInput,
) -> Result<TestCase, MachOEntryFunctionTestCaseError> {
    mach_o_entry_function_input_with_embedded_host_traps(case_id, input)
        .map(MachOEntryFunctionInput::into_test_case)
}

fn mach_o_entry_function_input_with_host_trap_plan(
    case_id: CaseId,
    input: &BinaryInput,
    host_trap_plan_from_image: impl FnOnce(
        &ExecutableImage,
    ) -> Result<
        TestCaseHostTrapPlan,
        MachOEntryFunctionTestCaseError,
    >,
) -> Result<MachOEntryFunctionInput, MachOEntryFunctionTestCaseError> {
    let report =
        probe_public_binary_format(input).map_err(MachOEntryFunctionTestCaseError::Probe)?;
    let conversion = report
        .metadata()
        .mach_o_metadata()
        .executable_image_conversion();
    let stack_state = testcase_stack_state_from_mach_o_conversion(conversion);
    let plan =
        plan_mach_o_executable_image(conversion).map_err(MachOEntryFunctionTestCaseError::Plan)?;
    let image = materialize_mach_o_executable_image(input, &plan)
        .map_err(MachOEntryFunctionTestCaseError::Materialization)?;
    let host_trap_plan = host_trap_plan_from_image(&image)?;

    let test_case = mach_o_executable_image_entry_function_with_host_traps_and_stack_state(
        case_id,
        &image,
        host_trap_plan,
        stack_state,
    )
    .map_err(MachOEntryFunctionTestCaseError::EntryFunction)?;

    Ok(MachOEntryFunctionInput::new(test_case, image))
}

const MACH_O_EMBEDDED_STDOUT_TRAP_MAGIC: &[u8] = b"BARA_STDOUT\0";

fn testcase_host_trap_plan_from_embedded_stdout_metadata(
    image: &ExecutableImage,
) -> Result<TestCaseHostTrapPlan, MachOEntryFunctionTestCaseError> {
    let Ok(entry_offset) = usize::try_from(image.entry().offset().value()) else {
        return Ok(TestCaseHostTrapPlan::none());
    };
    let Some(entry_prefix) = image.code_segment().x86_bytes().bytes().get(..entry_offset) else {
        return Ok(TestCaseHostTrapPlan::none());
    };
    let Some(stdout_payload) = entry_prefix.strip_prefix(MACH_O_EMBEDDED_STDOUT_TRAP_MAGIC) else {
        return Ok(TestCaseHostTrapPlan::none());
    };

    let stdout_text = std::str::from_utf8(stdout_payload)
        .map_err(|_| {
            MachOEntryFunctionTestCaseError::EmbeddedStdoutTrap(TestCaseStdoutTrapError::NonAscii)
        })?
        .to_owned();
    let stdout = TestCaseStdoutTrap::from_text(stdout_text)
        .map_err(MachOEntryFunctionTestCaseError::EmbeddedStdoutTrap)?;

    Ok(TestCaseHostTrapPlan::stdout(stdout))
}

fn testcase_stack_state_from_mach_o_conversion(
    conversion: &MachOExecutableImageConversion,
) -> TestCaseStackState {
    let Some(entry_point) = conversion.entry_point() else {
        return TestCaseStackState::none();
    };
    let stacksize = entry_point.metadata().stacksize().as_u64();
    let Some(size) = TestCaseStackSize::from_optional_nonzero_byte_count(stacksize) else {
        return TestCaseStackState::none();
    };

    TestCaseStackState::with_size(size)
}
