use std::{error::Error, fmt};

use bara_oracle::{
    ExecutableManifest, ExecutableManifestJsonError, FailureKind, HostHelperName,
    HostHelperResolutionPlan, HostHelperSignature, ObservedResult, TestCaseHostTrapPlan,
};

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
    Preflight(ExecutableRunPreflightError),
    Function(FunctionRunError),
}

impl ExecutableRunError {
    pub(crate) fn failure_kind(&self) -> FailureKind {
        match self {
            Self::EntryFunction(_) => FailureKind::InvalidTestCase,
            Self::Preflight(_) => FailureKind::InvalidTestCase,
            Self::Function(error) => error.failure_kind(),
        }
    }
}

impl fmt::Display for ExecutableRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryFunction(error) => write!(formatter, "entry function error: {error}"),
            Self::Preflight(error) => write!(formatter, "execution preflight error: {error}"),
            Self::Function(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ExecutableRunError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExecutableRunPreflight {
    stdout_host_helper: Option<PreflightHostHelper>,
}

impl ExecutableRunPreflight {
    fn no_host_helpers() -> Self {
        Self {
            stdout_host_helper: None,
        }
    }

    fn with_stdout_host_helper(stdout_host_helper: PreflightHostHelper) -> Self {
        Self {
            stdout_host_helper: Some(stdout_host_helper),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreflightHostHelper {
    name: HostHelperName,
    signature: HostHelperSignature,
}

impl PreflightHostHelper {
    const fn write_stdout_ptr_len_to_unit() -> Self {
        Self {
            name: HostHelperName::WriteStdout,
            signature: HostHelperSignature::PtrLenToUnit,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ExecutableRunPreflightError {
    MissingResolvedWriteStdout,
    UnexpectedWriteStdoutResolution {
        actual_name: HostHelperName,
        actual_signature: HostHelperSignature,
    },
}

impl fmt::Display for ExecutableRunPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingResolvedWriteStdout => {
                write!(formatter, "stdout host trap requires resolved write_stdout helper")
            }
            Self::UnexpectedWriteStdoutResolution {
                actual_name,
                actual_signature,
            } => write!(
                formatter,
                "stdout host trap resolved unexpected helper: name={actual_name:?}, signature={actual_signature:?}"
            ),
        }
    }
}

impl Error for ExecutableRunPreflightError {}

fn preflight_executable_run(
    host_trap_plan: &TestCaseHostTrapPlan,
    host_helper_resolution_plan: &HostHelperResolutionPlan,
) -> Result<ExecutableRunPreflight, ExecutableRunPreflightError> {
    if host_trap_plan.stdout_trap().is_none() {
        return Ok(ExecutableRunPreflight::no_host_helpers());
    }

    let write_stdout = host_helper_resolution_plan
        .write_stdout()
        .ok_or(ExecutableRunPreflightError::MissingResolvedWriteStdout)?;
    if write_stdout.name() != HostHelperName::WriteStdout
        || write_stdout.signature() != HostHelperSignature::PtrLenToUnit
    {
        return Err(
            ExecutableRunPreflightError::UnexpectedWriteStdoutResolution {
                actual_name: write_stdout.name(),
                actual_signature: write_stdout.signature(),
            },
        );
    }

    Ok(ExecutableRunPreflight::with_stdout_host_helper(
        PreflightHostHelper::write_stdout_ptr_len_to_unit(),
    ))
}

pub(crate) fn run_executable_manifest(
    manifest: &ExecutableManifest,
) -> Result<ExecutableRunResult, ExecutableRunError> {
    let entry_function = manifest
        .entry_function()
        .map_err(ExecutableRunError::EntryFunction)?;
    let _preflight = preflight_executable_run(
        entry_function.host_trap_plan(),
        manifest.host_helper_resolution_plan(),
    )
    .map_err(ExecutableRunError::Preflight)?;
    let function_result =
        run_test_case_function(&entry_function).map_err(ExecutableRunError::Function)?;

    Ok(ExecutableRunResult::from_entry_function(
        manifest.executable_id().clone(),
        function_result,
    ))
}

#[cfg(test)]
mod tests {
    use bara_oracle::{
        executable_manifest_from_json, HostHelperName, HostHelperResolutionPlan,
        HostHelperSignature, ObservedResult, TestCaseHostTrapPlan, TestCaseStdoutTrap,
    };

    use super::{
        preflight_executable_run, run_executable_manifest, ExecutableRunPreflight,
        ExecutableRunPreflightError, PreflightHostHelper,
    };

    #[test]
    fn executable_manifest_run_preflights_resolved_stdout_helper() {
        let manifest = executable_manifest_from_json(include_str!(
            "../../../tests/executables/hello_world_executable_manifest.json"
        ))
        .expect("executable manifest parses");

        let actual = run_executable_manifest(&manifest)
            .expect("executable manifest runs")
            .into_observed_result();

        assert_eq!(
            actual,
            ObservedResult::new(
                manifest.executable_id().clone(),
                0,
                0,
                String::from("hello world\n"),
                String::new(),
            )
        );
    }

    #[test]
    fn stdout_host_trap_preflight_reports_resolved_write_stdout_helper() {
        let manifest = executable_manifest_from_json(include_str!(
            "../../../tests/executables/hello_world_executable_manifest.json"
        ))
        .expect("executable manifest parses");
        let entry_function = manifest
            .entry_function()
            .expect("entry function conversion succeeds");

        let preflight = preflight_executable_run(
            entry_function.host_trap_plan(),
            manifest.host_helper_resolution_plan(),
        )
        .expect("stdout host trap helper is resolved");

        assert_eq!(
            preflight,
            ExecutableRunPreflight::with_stdout_host_helper(PreflightHostHelper {
                name: HostHelperName::WriteStdout,
                signature: HostHelperSignature::PtrLenToUnit,
            })
        );
    }

    #[test]
    fn stdout_host_trap_preflight_rejects_missing_resolved_helper() {
        let stdout = TestCaseStdoutTrap::from_text(String::from("hello world\n"))
            .expect("stdout trap text is valid");
        let host_trap_plan = TestCaseHostTrapPlan::stdout(stdout);
        let resolution_plan = HostHelperResolutionPlan::empty();

        assert_eq!(
            preflight_executable_run(&host_trap_plan, &resolution_plan),
            Err(ExecutableRunPreflightError::MissingResolvedWriteStdout)
        );
    }
}
