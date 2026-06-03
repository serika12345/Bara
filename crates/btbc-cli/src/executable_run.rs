use std::{error::Error, fmt};

use bara_oracle::{ExecutableManifest, ExecutableManifestJsonError, FailureKind, ObservedResult};

use crate::function_run::{run_test_case_function, FunctionRunError, FunctionRunResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutableRunResult {
    executable_id: bara_oracle::CaseId,
    process_result: ProcessLikeRunResult,
}

impl ExecutableRunResult {
    fn from_entry_function(
        executable_id: bara_oracle::CaseId,
        function_result: FunctionRunResult,
    ) -> Self {
        Self {
            executable_id,
            process_result: ProcessLikeRunResult::from_function(function_result),
        }
    }

    pub(crate) fn into_observed_result(self) -> ObservedResult {
        self.process_result.into_observed_result(self.executable_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProcessLikeRunResult {
    exit_status: ProcessExitStatus,
    function_result: FunctionRunResult,
    stderr: ProcessStderr,
}

impl ProcessLikeRunResult {
    fn from_function(function_result: FunctionRunResult) -> Self {
        Self {
            exit_status: ProcessExitStatus::success(),
            function_result,
            stderr: ProcessStderr::empty(),
        }
    }

    fn into_observed_result(self, executable_id: bara_oracle::CaseId) -> ObservedResult {
        let function_observed = self.function_result.into_observed_result(executable_id);
        ObservedResult::new(
            function_observed.case_id().clone(),
            self.exit_status.into_raw(),
            function_observed.return_value(),
            function_observed.stdout().to_owned(),
            self.stderr.into_text(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProcessExitStatus(i32);

impl ProcessExitStatus {
    const fn success() -> Self {
        Self(0)
    }

    const fn into_raw(self) -> i32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProcessStderr(String);

impl ProcessStderr {
    fn empty() -> Self {
        Self(String::new())
    }

    fn into_text(self) -> String {
        self.0
    }
}

#[derive(Debug)]
pub(crate) enum ExecutableRunError {
    EntryFunction(ExecutableManifestJsonError),
    Function(FunctionRunError),
}

impl ExecutableRunError {
    pub(crate) fn failure_kind(&self) -> FailureKind {
        match self {
            Self::EntryFunction(_) => FailureKind::InvalidTestCase,
            Self::Function(error) => error.failure_kind(),
        }
    }
}

impl fmt::Display for ExecutableRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryFunction(error) => write!(formatter, "entry function error: {error}"),
            Self::Function(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ExecutableRunError {}

pub(crate) fn run_executable_manifest(
    manifest: &ExecutableManifest,
) -> Result<ExecutableRunResult, ExecutableRunError> {
    let entry_function = manifest
        .entry_function()
        .map_err(ExecutableRunError::EntryFunction)?;
    let function_result =
        run_test_case_function(&entry_function).map_err(ExecutableRunError::Function)?;

    Ok(ExecutableRunResult::from_entry_function(
        manifest.executable_id().clone(),
        function_result,
    ))
}
