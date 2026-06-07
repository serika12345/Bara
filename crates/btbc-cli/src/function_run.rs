use std::{error::Error, fmt};

use bara_arm64::{emit_program, EmittedHostTrapRequests};
use bara_isa_x86::{decode_function, lift_decoded_function};
use bara_oracle::{FailureKind, ObservedResult, TestCase, TestCaseAbi};
use bara_runtime::{
    run_no_args_u64_with_host_traps, run_one_input_memory_ptr, run_one_u64, HostTrapPlan,
    InputMemory, InputMemoryError, RunArgumentU64, RunError, RunStdout, RunStdoutError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionCompileResult {
    emitted: bara_arm64::EmittedFunction,
}

impl FunctionCompileResult {
    fn new(emitted: bara_arm64::EmittedFunction) -> Self {
        Self { emitted }
    }

    pub(crate) fn arm64_bytes(&self) -> FunctionArm64Bytes<'_> {
        FunctionArm64Bytes::new(self.emitted.code())
    }

    fn emitted(&self) -> &bara_arm64::EmittedFunction {
        &self.emitted
    }

    pub(crate) fn stdout_host_trap_request(&self) -> FunctionStdoutHostTrapRequest {
        FunctionStdoutHostTrapRequest::new(self.emitted.host_trap_requests().stdout_requested())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FunctionArm64Bytes<'a>(&'a bara_arm64::Arm64MachineCode);

impl<'a> FunctionArm64Bytes<'a> {
    const fn new(code: &'a bara_arm64::Arm64MachineCode) -> Self {
        Self(code)
    }

    pub(crate) fn as_slice(self) -> &'a [u8] {
        self.0.bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FunctionStdoutHostTrapRequest {
    requested: bool,
}

impl FunctionStdoutHostTrapRequest {
    const fn new(requested: bool) -> Self {
        Self { requested }
    }

    pub(crate) const fn is_requested(self) -> bool {
        self.requested
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionRunResult {
    return_value: FunctionReturnValue,
    stdout: FunctionStdout,
}

impl FunctionRunResult {
    fn from_runtime(result: &bara_runtime::RunResult) -> Self {
        Self {
            return_value: FunctionReturnValue::from_runtime(result),
            stdout: FunctionStdout::from_runtime(result),
        }
    }

    pub(crate) fn into_observed_result(self, case_id: bara_oracle::CaseId) -> ObservedResult {
        ObservedResult::new(
            case_id,
            0,
            self.return_value.into_raw(),
            self.stdout.into_text(),
            String::new(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionReturnValue(u64);

impl FunctionReturnValue {
    fn from_runtime(result: &bara_runtime::RunResult) -> Self {
        Self(result.return_value())
    }

    fn into_raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionStdout(String);

impl FunctionStdout {
    fn from_runtime(result: &bara_runtime::RunResult) -> Self {
        Self(result.stdout().to_owned())
    }

    fn into_text(self) -> String {
        self.0
    }
}

#[derive(Debug)]
pub(crate) enum FunctionRunError {
    Decode(bara_isa_x86::DecodeError),
    Lift(bara_isa_x86::LiftError),
    Emit(bara_arm64::EmitError),
    StandaloneArtifact(FunctionStandaloneArtifactError),
    InputMemory(InputMemoryError),
    StdoutTrap(RunStdoutError),
    Run(RunError),
}

impl FunctionRunError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::Decode(_) => FailureKind::DecodeError,
            Self::Lift(_) => FailureKind::LiftError,
            Self::Emit(_) => FailureKind::EmitError,
            Self::StandaloneArtifact(_) => FailureKind::EmitError,
            Self::InputMemory(_) | Self::StdoutTrap(_) | Self::Run(_) => FailureKind::RunError,
        }
    }
}

impl fmt::Display for FunctionRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "decode error: {error:?}"),
            Self::Lift(error) => write!(formatter, "lift error: {error:?}"),
            Self::Emit(error) => write!(formatter, "emit error: {error:?}"),
            Self::StandaloneArtifact(error) => {
                write!(formatter, "standalone artifact error: {error}")
            }
            Self::InputMemory(error) => write!(formatter, "input memory error: {error:?}"),
            Self::StdoutTrap(error) => write!(formatter, "stdout trap error: {error:?}"),
            Self::Run(error) => write!(formatter, "run error: {error:?}"),
        }
    }
}

impl Error for FunctionRunError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FunctionStandaloneArtifactError {
    HostTrapRequested,
}

impl fmt::Display for FunctionStandaloneArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostTrapRequested => write!(
                formatter,
                "host trap requested by testcase; standalone ARM64 artifact is unsupported"
            ),
        }
    }
}

impl Error for FunctionStandaloneArtifactError {}

pub(crate) fn compile_test_case_function(
    test_case: &TestCase,
) -> Result<FunctionCompileResult, FunctionRunError> {
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).map_err(FunctionRunError::Decode)?;
    let program = lift_decoded_function(&decoded).map_err(FunctionRunError::Lift)?;
    let emitted = emit_program(&program).map_err(FunctionRunError::Emit)?;

    Ok(FunctionCompileResult::new(emitted))
}

pub(crate) fn compile_test_case_function_standalone_artifact(
    test_case: &TestCase,
) -> Result<FunctionCompileResult, FunctionRunError> {
    let compiled = compile_test_case_function(test_case)?;
    if !test_case.host_trap_plan().is_empty()
        || compiled.emitted().host_trap_requests().stdout_requested()
    {
        return Err(FunctionRunError::StandaloneArtifact(
            FunctionStandaloneArtifactError::HostTrapRequested,
        ));
    }

    Ok(compiled)
}

pub(crate) fn run_test_case_function(
    test_case: &TestCase,
) -> Result<FunctionRunResult, FunctionRunError> {
    let compiled = compile_test_case_function(test_case)?;
    let emitted = compiled.emitted();
    let result = match test_case.abi() {
        TestCaseAbi::NoArgsU64 => run_no_args_u64_with_host_traps(
            emitted.code().bytes(),
            runtime_host_trap_plan(test_case.host_trap_plan(), emitted.host_trap_requests())?,
        ),
        TestCaseAbi::OneU64ArgReturnsU64 { argument } => run_one_u64(
            emitted.code().bytes(),
            RunArgumentU64::new(argument.value()),
        ),
        TestCaseAbi::OneInputMemoryPtrReturnsU64 { memory } => {
            let memory = InputMemory::from_bytes(memory.bytes().to_vec())
                .map_err(FunctionRunError::InputMemory)?;
            run_one_input_memory_ptr(emitted.code().bytes(), memory)
        }
    }
    .map_err(FunctionRunError::Run)?;

    Ok(FunctionRunResult::from_runtime(&result))
}

fn runtime_host_trap_plan(
    plan: &bara_oracle::TestCaseHostTrapPlan,
    requests: &EmittedHostTrapRequests,
) -> Result<HostTrapPlan, FunctionRunError> {
    if !requests.stdout_requested() {
        return Ok(HostTrapPlan::none());
    }

    let Some(stdout) = plan.stdout_trap() else {
        return Ok(HostTrapPlan::none());
    };
    let stdout =
        RunStdout::from_text(stdout.text().to_owned()).map_err(FunctionRunError::StdoutTrap)?;
    Ok(HostTrapPlan::stdout(stdout))
}

#[cfg(test)]
mod tests {
    use bara_oracle::test_case_from_json;

    use super::{
        compile_test_case_function, compile_test_case_function_standalone_artifact,
        FunctionRunError, FunctionStandaloneArtifactError,
    };

    #[test]
    fn compile_only_returns_return_42_arm64_bytes() {
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");

        let compiled =
            compile_test_case_function(&test_case).expect("return_42 compile-only succeeds");

        assert_eq!(
            compiled.arm64_bytes().as_slice(),
            &[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn standalone_artifact_rejects_stdout_host_trap_fixture() {
        let test_case = test_case_from_json(include_str!(
            "../../../tests/cases/stdout_trap_return_0.json"
        ))
        .expect("stdout trap testcase parses");

        let error = compile_test_case_function_standalone_artifact(&test_case)
            .expect_err("stdout host trap fixture is not exportable as standalone artifact");

        assert!(matches!(
            error,
            FunctionRunError::StandaloneArtifact(
                FunctionStandaloneArtifactError::HostTrapRequested
            )
        ));
    }
}
