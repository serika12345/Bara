use std::{error::Error, fmt};

use bara_arm64::{emit_program, EmittedHostTrapRequests};
use bara_isa_x86::{decode_function, lift_decoded_function};
use bara_oracle::{FailureKind, ObservedResult, TestCase, TestCaseAbi};
use bara_runtime::{
    run_no_args_u64_with_host_traps, run_one_input_memory_ptr, run_one_u64, HostTrapPlan,
    InputMemory, InputMemoryError, RunArgumentU64, RunError, RunStdout, RunStdoutError,
};

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
            Self::InputMemory(error) => write!(formatter, "input memory error: {error:?}"),
            Self::StdoutTrap(error) => write!(formatter, "stdout trap error: {error:?}"),
            Self::Run(error) => write!(formatter, "run error: {error:?}"),
        }
    }
}

impl Error for FunctionRunError {}

pub(crate) fn run_test_case_function(
    test_case: &TestCase,
) -> Result<FunctionRunResult, FunctionRunError> {
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).map_err(FunctionRunError::Decode)?;
    let program = lift_decoded_function(&decoded).map_err(FunctionRunError::Lift)?;
    let emitted = emit_program(&program).map_err(FunctionRunError::Emit)?;
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
