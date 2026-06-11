use crate::{
    executable_manifest::ExecutableImage, CaseId, TestCase, TestCaseHostTrapPlan,
    TestCaseStackSize, TestCaseStackState, TestCaseStdoutTrap, TestCaseStdoutTrapError,
};

use bara_ir::{
    ProgramImageImports, ProgramImageMappedByteSegment, ProgramImageMappedBytes,
    ProgramImageMetadata, ProgramImageMetadataError, ProgramImageRange, ProgramImageRelocations,
    ProgramImageSection, ProgramImageSectionKind, ProgramImageSections, ProgramImageSymbols,
    ProgramUnwindMetadata,
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
    ProgramImageMetadata(ProgramImageMetadataError),
    EntryFunction(MachOExecutableImageEntryFunctionError),
    EmbeddedStdoutTrap(TestCaseStdoutTrapError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOEntryFunctionInput {
    test_case: TestCase,
    executable_image: ExecutableImage,
    program_image_metadata: ProgramImageMetadata,
}

impl MachOEntryFunctionInput {
    const fn new(
        test_case: TestCase,
        executable_image: ExecutableImage,
        program_image_metadata: ProgramImageMetadata,
    ) -> Self {
        Self {
            test_case,
            executable_image,
            program_image_metadata,
        }
    }

    pub const fn test_case(&self) -> &TestCase {
        &self.test_case
    }

    pub const fn executable_image(&self) -> &ExecutableImage {
        &self.executable_image
    }

    pub const fn program_image_metadata(&self) -> &ProgramImageMetadata {
        &self.program_image_metadata
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
    let embedded_stdout_metadata = embedded_stdout_metadata_from_image(&image)?;
    let host_trap_plan = host_trap_plan_from_image(&image)?;
    let program_image_metadata =
        program_image_metadata_from_executable_image(&image, embedded_stdout_metadata.as_ref())
            .map_err(MachOEntryFunctionTestCaseError::ProgramImageMetadata)?;

    let test_case = mach_o_executable_image_entry_function_with_host_traps_and_stack_state(
        case_id,
        &image,
        host_trap_plan,
        stack_state,
    )
    .map_err(MachOEntryFunctionTestCaseError::EntryFunction)?;

    Ok(MachOEntryFunctionInput::new(
        test_case,
        image,
        program_image_metadata,
    ))
}

fn program_image_metadata_from_executable_image(
    image: &ExecutableImage,
    embedded_stdout_metadata: Option<&EmbeddedStdoutMetadata>,
) -> Result<ProgramImageMetadata, ProgramImageMetadataError> {
    let code = image.code_segment().x86_bytes();
    let code_len = u64::try_from(code.bytes().len())
        .map_err(|_| ProgramImageMetadataError::AddressOverflow)?;
    let code_start = code
        .entry()
        .checked_add(image.entry().offset().value())
        .map_err(|_| ProgramImageMetadataError::AddressOverflow)?;
    let code_end = code
        .entry()
        .checked_add(code_len)
        .map_err(|_| ProgramImageMetadataError::AddressOverflow)?;
    let mapped_code_range = ProgramImageRange::new(code.entry(), code_end)?;
    let mapped_bytes =
        ProgramImageMappedBytes::from_segments([ProgramImageMappedByteSegment::new(
            mapped_code_range,
            code.bytes().to_vec(),
        )?]);
    let code_range = ProgramImageRange::new(code_start, code_end)?;
    let mut sections = vec![ProgramImageSection::new(
        ProgramImageSectionKind::Code,
        code_range,
    )];

    if let Some(metadata) = embedded_stdout_metadata {
        sections.push(ProgramImageSection::new(
            ProgramImageSectionKind::ConstData,
            metadata.const_data_range(),
        ));
    }

    Ok(ProgramImageMetadata::new_with_mapped_bytes(
        ProgramImageSections::from_items(sections),
        mapped_bytes,
        ProgramImageSymbols::empty(),
        ProgramImageRelocations::empty(),
        ProgramImageImports::empty(),
        ProgramUnwindMetadata::empty(),
    ))
}

const MACH_O_EMBEDDED_STDOUT_TRAP_MAGIC: &[u8] = b"BARA_STDOUT\0";

fn testcase_host_trap_plan_from_embedded_stdout_metadata(
    image: &ExecutableImage,
) -> Result<TestCaseHostTrapPlan, MachOEntryFunctionTestCaseError> {
    let Some(metadata) = embedded_stdout_metadata_from_image(image)? else {
        return Ok(TestCaseHostTrapPlan::none());
    };

    Ok(TestCaseHostTrapPlan::stdout(metadata.stdout_trap()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EmbeddedStdoutMetadata {
    stdout: TestCaseStdoutTrap,
    const_data_range: ProgramImageRange,
}

impl EmbeddedStdoutMetadata {
    const fn new(stdout: TestCaseStdoutTrap, const_data_range: ProgramImageRange) -> Self {
        Self {
            stdout,
            const_data_range,
        }
    }

    fn stdout_trap(&self) -> TestCaseStdoutTrap {
        self.stdout.clone()
    }

    const fn const_data_range(&self) -> ProgramImageRange {
        self.const_data_range
    }
}

fn embedded_stdout_metadata_from_image(
    image: &ExecutableImage,
) -> Result<Option<EmbeddedStdoutMetadata>, MachOEntryFunctionTestCaseError> {
    let Ok(entry_offset) = usize::try_from(image.entry().offset().value()) else {
        return Ok(None);
    };
    let code = image.code_segment().x86_bytes();
    let Some(entry_prefix) = code.bytes().get(..entry_offset) else {
        return Ok(None);
    };
    let Some(stdout_payload) = entry_prefix.strip_prefix(MACH_O_EMBEDDED_STDOUT_TRAP_MAGIC) else {
        return Ok(None);
    };

    let stdout_text = std::str::from_utf8(stdout_payload)
        .map_err(|_| {
            MachOEntryFunctionTestCaseError::EmbeddedStdoutTrap(TestCaseStdoutTrapError::NonAscii)
        })?
        .to_owned();
    let stdout = TestCaseStdoutTrap::from_text(stdout_text)
        .map_err(MachOEntryFunctionTestCaseError::EmbeddedStdoutTrap)?;
    let const_data_end = code
        .entry()
        .checked_add(u64::try_from(entry_offset).map_err(|_| {
            MachOEntryFunctionTestCaseError::ProgramImageMetadata(
                ProgramImageMetadataError::AddressOverflow,
            )
        })?)
        .map_err(|_| {
            MachOEntryFunctionTestCaseError::ProgramImageMetadata(
                ProgramImageMetadataError::AddressOverflow,
            )
        })?;
    let const_data_range = ProgramImageRange::new(code.entry(), const_data_end)
        .map_err(MachOEntryFunctionTestCaseError::ProgramImageMetadata)?;

    Ok(Some(EmbeddedStdoutMetadata::new(stdout, const_data_range)))
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
