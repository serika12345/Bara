use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
};

use bara_oracle::{
    binary_format_probe_report_from_json, binary_format_probe_report_to_json,
    compare_observed_results, corpus_report_to_json, executable_manifest_from_json,
    host_trap_plan_from_json, mach_o_entry_function_test_case,
    mach_o_entry_function_test_case_with_host_traps, observed_result_from_json,
    observed_result_to_json, probe_public_binary_format, test_case_from_json, BinaryFileBytes,
    BinaryFormatProbeError, BinaryFormatProbeReport, BinaryInput, CaseId, ComparisonReport,
    CorpusReport, ExecutableManifest, ExpectedResult, FailureKind, FailureMessage, FixtureOutcome,
    FixtureReport, MachOEntryFunctionTestCaseError, ObservedResult, TestCase,
};

mod blackbox_run;
mod executable_run;
mod function_run;
mod native_artifact;
#[cfg(test)]
mod native_artifact_cli_tests;

use blackbox_run::run_check_blackbox;
use executable_run::{run_executable_manifest, ExecutableRunError};
use function_run::{
    compile_test_case_function, compile_test_case_function_standalone_artifact,
    run_test_case_function, FunctionRunError,
};
use native_artifact::{
    link_arm64_main_executable, link_arm64_stdout_main_executable,
    native_artifact_metadata_to_json, observe_native_executable_artifact, NativeArtifactError,
};

fn main() -> ExitCode {
    match run_cli(env::args().skip(1).collect()) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli(args: Vec<String>) -> Result<String, CliError> {
    match args.as_slice() {
        [command] if command == "check-m1" => run_m1_check(),
        [command, case_path, expected_path] if command == "check-fixture" => {
            run_check_fixture(Path::new(case_path), Path::new(expected_path))
        }
        [command, manifest_path, expected_path] if command == "check-executable" => {
            run_check_executable(Path::new(manifest_path), Path::new(expected_path))
        }
        [command, binary_path, expected_path] if command == "check-mach-o" => {
            run_check_mach_o(Path::new(binary_path), Path::new(expected_path))
        }
        [command, binary_path, host_traps_path, expected_path]
            if command == "check-mach-o-host-traps" =>
        {
            run_check_mach_o_host_traps(
                Path::new(binary_path),
                Path::new(host_traps_path),
                Path::new(expected_path),
            )
        }
        [command, binary_path] if command == "probe-binary" => {
            run_probe_binary(Path::new(binary_path))
        }
        [command, binary_path, expected_path] if command == "check-binary-probe" => {
            run_check_binary_probe(Path::new(binary_path), Path::new(expected_path))
        }
        [command, case_path, output_path] if command == "emit-fixture-arm64" => {
            run_emit_fixture_arm64(Path::new(case_path), Path::new(output_path))
        }
        [command, case_path, output_path] if command == "link-fixture-arm64-main" => {
            run_link_fixture_arm64_main(Path::new(case_path), Path::new(output_path))
        }
        [command, case_path, output_path] if command == "link-fixture-arm64-stdout-main" => {
            run_link_fixture_arm64_stdout_main(Path::new(case_path), Path::new(output_path))
        }
        [command] if command == "check-blackbox" => run_check_blackbox(None),
        [command, output_flag, output_dir]
            if command == "check-blackbox" && output_flag == "--out" =>
        {
            run_check_blackbox(Some(Path::new(output_dir)))
        }
        [command, cases_dir, expected_dir] if command == "check-corpus" => {
            run_check_corpus(Path::new(cases_dir), Path::new(expected_dir))
        }
        [command, cases_dir, expected_dir, output_flag, output_dir]
            if command == "check-corpus" && output_flag == "--out" =>
        {
            run_check_corpus_with_output(
                Path::new(cases_dir),
                Path::new(expected_dir),
                Some(Path::new(output_dir)),
            )
        }
        _ => Err(CliError::Usage),
    }
}

fn run_m1_check() -> Result<String, CliError> {
    run_m1_check_from_fixtures(
        include_str!("../../../tests/cases/return_42.json"),
        include_str!("../../../tests/expected/return_42.json"),
    )
}

fn run_m1_check_from_fixtures(case_json: &str, expected_json: &str) -> Result<String, CliError> {
    let test_case = test_case_from_json(case_json).map_err(CliError::TestCase)?;
    let expected = observed_result_from_json(expected_json).map_err(CliError::ExpectedJson)?;
    run_test_case(test_case, expected)
}

fn run_check_fixture(case_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let expected_json = read_text_file(expected_path)?;

    run_m1_check_from_fixtures(&case_json, &expected_json)
}

fn run_emit_fixture_arm64(case_path: &Path, output_path: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let compiled = compile_test_case_function_standalone_artifact(&test_case)
        .map_err(CliError::FunctionRun)?;
    write_binary_file(output_path, compiled.arm64_bytes().as_slice())?;

    Ok(format!(
        "wrote ARM64 machine code for {} to {}",
        test_case.case_id().as_str(),
        output_path.display()
    ))
}

fn run_link_fixture_arm64_main(case_path: &Path, output_path: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let compiled = compile_test_case_function_standalone_artifact(&test_case)
        .map_err(CliError::FunctionRun)?;
    let artifact = link_arm64_main_executable(compiled.arm64_bytes(), output_path)
        .map_err(CliError::NativeArtifact)?;

    native_artifact_metadata_to_json(artifact.metadata()).map_err(CliError::Json)
}

fn run_link_fixture_arm64_stdout_main(
    case_path: &Path,
    output_path: &Path,
) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let compiled = compile_test_case_function(&test_case).map_err(CliError::FunctionRun)?;
    let artifact = link_arm64_stdout_main_executable(
        compiled.arm64_bytes(),
        test_case.host_trap_plan(),
        compiled.stdout_host_trap_request(),
        output_path,
    )
    .map_err(CliError::NativeArtifact)?;

    let actual = observe_native_executable_artifact(test_case.case_id().clone(), &artifact)
        .map_err(CliError::NativeArtifact)?;
    observed_result_to_json(&actual).map_err(CliError::Json)
}

fn run_check_executable(manifest_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let manifest_json = read_text_file(manifest_path)?;
    let expected_json = read_text_file(expected_path)?;
    let manifest =
        executable_manifest_from_json(&manifest_json).map_err(CliError::ExecutableManifest)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;

    run_executable(manifest, expected)
}

fn run_check_mach_o(binary_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let expected_json = read_text_file(expected_path)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    let test_case = mach_o_entry_function_test_case(case_id_from_path(binary_path), &input)
        .map_err(CliError::MachOEntryFunctionTestCase)?;

    run_test_case(test_case, expected)
}

fn run_check_mach_o_host_traps(
    binary_path: &Path,
    host_traps_path: &Path,
    expected_path: &Path,
) -> Result<String, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let host_traps_json = read_text_file(host_traps_path)?;
    let expected_json = read_text_file(expected_path)?;
    let host_trap_plan =
        host_trap_plan_from_json(&host_traps_json).map_err(CliError::HostTrapPlan)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    let test_case = mach_o_entry_function_test_case_with_host_traps(
        case_id_from_path(binary_path),
        &input,
        host_trap_plan,
    )
    .map_err(CliError::MachOEntryFunctionTestCase)?;

    run_test_case(test_case, expected)
}

fn run_probe_binary(binary_path: &Path) -> Result<String, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    let report = probe_public_binary_format(&input).map_err(CliError::BinaryFormatProbe)?;

    binary_format_probe_report_to_json(&report).map_err(CliError::Json)
}

fn run_check_binary_probe(binary_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let expected_json = read_text_file(expected_path)?;
    let expected = binary_format_probe_report_from_json(&expected_json)
        .map_err(CliError::ExpectedProbeJson)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    let actual = probe_public_binary_format(&input).map_err(CliError::BinaryFormatProbe)?;

    if actual != expected {
        return Err(CliError::BinaryProbeComparisonMismatch {
            expected: Box::new(expected),
            actual: Box::new(actual),
        });
    }

    binary_format_probe_report_to_json(&actual).map_err(CliError::Json)
}

fn run_check_corpus(cases_dir: &Path, expected_dir: &Path) -> Result<String, CliError> {
    run_check_corpus_with_output(cases_dir, expected_dir, None)
}

fn run_check_corpus_with_output(
    cases_dir: &Path,
    expected_dir: &Path,
    output_dir: Option<&Path>,
) -> Result<String, CliError> {
    let case_paths = sorted_case_paths(cases_dir)?;
    if case_paths.is_empty() {
        return Err(CliError::EmptyCorpus {
            cases_dir: cases_dir.to_path_buf(),
        });
    }

    let mut fixture_runs = Vec::new();
    for case_path in case_paths {
        fixture_runs.push(run_corpus_fixture(&case_path, expected_dir));
    }

    let report = fixture_runs
        .iter()
        .map(|run| run.report.clone())
        .collect::<CorpusReport>();
    if let Some(output_dir) = output_dir {
        write_corpus_outputs(output_dir, &report, &fixture_runs)?;
    }
    if !report.is_success() {
        return Err(CliError::CorpusFailures(report));
    }

    corpus_report_to_json(&report).map_err(CliError::Json)
}

fn run_corpus_fixture(case_path: &Path, expected_dir: &Path) -> FixtureRun {
    let fallback_case_id = case_id_from_path(case_path);
    let case_json = match read_text_file(case_path) {
        Ok(case_json) => case_json,
        Err(error) => {
            return FixtureRun::failed(
                fallback_case_id,
                FailureKind::InvalidTestCase,
                error.to_string(),
            );
        }
    };
    let test_case = match test_case_from_json(&case_json) {
        Ok(test_case) => test_case,
        Err(error) => {
            return FixtureRun::failed(
                fallback_case_id,
                FailureKind::InvalidTestCase,
                error.to_string(),
            );
        }
    };
    let case_id = test_case.case_id().clone();
    let expected_path = expected_dir.join(format!("{}.json", case_id.as_str()));
    let expected_json = match read_text_file(&expected_path) {
        Ok(expected_json) => expected_json,
        Err(error) => {
            return FixtureRun::failed(case_id, FailureKind::MissingExpected, error.to_string());
        }
    };
    let expected = match observed_result_from_json(&expected_json) {
        Ok(expected) => expected,
        Err(error) => {
            return FixtureRun::failed(case_id, FailureKind::InvalidExpected, error.to_string());
        }
    };

    let actual = match observe_test_case(&test_case) {
        Ok(actual) => actual,
        Err(error) => {
            return FixtureRun::failed(case_id, error.failure_kind(), error.to_string());
        }
    };
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        return FixtureRun::failed_with_actual(
            case_id,
            FailureKind::ComparisonMismatch,
            format!("comparison failed: {comparison:?}"),
            actual,
        );
    }

    FixtureRun::passed_observed(case_id, actual)
}

fn run_test_case(test_case: TestCase, expected: ExpectedResult) -> Result<String, CliError> {
    let actual = observe_test_case(&test_case)?;
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        return Err(CliError::Comparison(comparison));
    }

    observed_result_to_json(&actual).map_err(CliError::Json)
}

fn observe_test_case(test_case: &TestCase) -> Result<ObservedResult, CliError> {
    run_test_case_function(test_case)
        .map_err(CliError::FunctionRun)
        .map(|result| result.into_observed_result(test_case.case_id().clone()))
}

fn run_executable(
    manifest: ExecutableManifest,
    expected: ExpectedResult,
) -> Result<String, CliError> {
    let actual = run_executable_manifest(&manifest)
        .map_err(CliError::ExecutableRun)?
        .into_observed_result();
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        return Err(CliError::Comparison(comparison));
    }

    observed_result_to_json(&actual).map_err(CliError::Json)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FixtureRun {
    report: FixtureReport,
    output: Option<FixtureOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FixtureOutput {
    Observed(ObservedResult),
    Probe(BinaryFormatProbeReport),
}

impl FixtureRun {
    fn passed(case_id: CaseId) -> Self {
        Self {
            report: FixtureReport::new(case_id, FixtureOutcome::Passed),
            output: None,
        }
    }

    fn passed_observed(case_id: CaseId, actual: ObservedResult) -> Self {
        Self {
            report: FixtureReport::new(case_id, FixtureOutcome::Passed),
            output: Some(FixtureOutput::Observed(actual)),
        }
    }

    fn passed_probe(case_id: CaseId, actual: BinaryFormatProbeReport) -> Self {
        Self {
            report: FixtureReport::new(case_id, FixtureOutcome::Passed),
            output: Some(FixtureOutput::Probe(actual)),
        }
    }

    fn failed(case_id: CaseId, kind: FailureKind, message: String) -> Self {
        Self {
            report: failed_fixture_report(case_id, kind, message),
            output: None,
        }
    }

    fn failed_with_actual(
        case_id: CaseId,
        kind: FailureKind,
        message: String,
        actual: ObservedResult,
    ) -> Self {
        Self {
            report: failed_fixture_report(case_id, kind, message),
            output: Some(FixtureOutput::Observed(actual)),
        }
    }
}

fn failed_fixture_report(case_id: CaseId, kind: FailureKind, message: String) -> FixtureReport {
    FixtureReport::new(
        case_id,
        FixtureOutcome::failed(kind, FailureMessage::from(message)),
    )
}

fn write_corpus_outputs(
    output_dir: &Path,
    report: &CorpusReport,
    fixture_runs: &[FixtureRun],
) -> Result<(), CliError> {
    create_dir(output_dir)?;
    let actual_dir = output_dir.join("actual");
    create_dir(&actual_dir)?;
    create_dir(&output_dir.join("compiled"))?;
    create_dir(&output_dir.join("ir"))?;
    create_dir(&output_dir.join("pcmap"))?;

    let report_json = corpus_report_to_json(report).map_err(CliError::Json)?;
    write_text_file(&output_dir.join("report.json"), &report_json)?;

    for run in fixture_runs {
        if let Some(output) = &run.output {
            let (case_id, actual_json) = match output {
                FixtureOutput::Observed(actual) => (
                    actual.case_id().as_str(),
                    observed_result_to_json(actual).map_err(CliError::Json)?,
                ),
                FixtureOutput::Probe(actual) => (
                    run.report.case_id().as_str(),
                    binary_format_probe_report_to_json(actual).map_err(CliError::Json)?,
                ),
            };
            write_text_file(&actual_dir.join(format!("{case_id}.json")), &actual_json)?;
        }
    }

    Ok(())
}

fn create_dir(path: &Path) -> Result<(), CliError> {
    fs::create_dir_all(path).map_err(|source| CliError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

fn read_text_file(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

fn read_binary_file(path: &Path) -> Result<Vec<u8>, CliError> {
    fs::read(path).map_err(|source| CliError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), CliError> {
    fs::write(path, contents).map_err(|source| CliError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn write_binary_file(path: &Path, contents: &[u8]) -> Result<(), CliError> {
    fs::write(path, contents).map_err(|source| CliError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn sorted_case_paths(cases_dir: &Path) -> Result<Vec<PathBuf>, CliError> {
    let mut paths = Vec::new();
    let entries = fs::read_dir(cases_dir).map_err(|source| CliError::ReadDir {
        path: cases_dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| CliError::ReadDirEntry {
            path: cases_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn case_id_from_path(path: &Path) -> CaseId {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| CaseId::new(stem).ok())
        .unwrap_or_else(|| CaseId::new("unknown").expect("fallback case id is non-empty"))
}

#[derive(Debug)]
enum CliError {
    Usage,
    ReadFile {
        path: PathBuf,
        source: io::Error,
    },
    WriteFile {
        path: PathBuf,
        source: io::Error,
    },
    CreateDir {
        path: PathBuf,
        source: io::Error,
    },
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    ReadDirEntry {
        path: PathBuf,
        source: io::Error,
    },
    EmptyCorpus {
        cases_dir: PathBuf,
    },
    ExecutableManifest(bara_oracle::ExecutableManifestJsonError),
    TestCase(bara_oracle::TestCaseJsonError),
    HostTrapPlan(bara_oracle::TestCaseJsonError),
    ExpectedJson(bara_oracle::JsonError),
    ExpectedProbeJson(bara_oracle::JsonError),
    BinaryFormatProbe(BinaryFormatProbeError),
    MachOEntryFunctionTestCase(MachOEntryFunctionTestCaseError),
    BinaryProbeComparisonMismatch {
        expected: Box<BinaryFormatProbeReport>,
        actual: Box<BinaryFormatProbeReport>,
    },
    FunctionRun(FunctionRunError),
    NativeArtifact(NativeArtifactError),
    ExecutableRun(ExecutableRunError),
    Comparison(ComparisonReport),
    Json(bara_oracle::JsonError),
    CorpusFailures(CorpusReport),
}

impl CliError {
    fn failure_kind(&self) -> FailureKind {
        match self {
            Self::TestCase(_) => FailureKind::InvalidTestCase,
            Self::HostTrapPlan(_) => FailureKind::InvalidTestCase,
            Self::ExecutableManifest(_) => FailureKind::InvalidTestCase,
            Self::ExpectedJson(_) => FailureKind::InvalidExpected,
            Self::ExpectedProbeJson(_) => FailureKind::InvalidExpected,
            Self::BinaryFormatProbe(_) => FailureKind::InvalidTestCase,
            Self::MachOEntryFunctionTestCase(_) => FailureKind::InvalidTestCase,
            Self::BinaryProbeComparisonMismatch { .. } => FailureKind::ComparisonMismatch,
            Self::FunctionRun(error) => error.failure_kind(),
            Self::NativeArtifact(error) => error.failure_kind(),
            Self::ExecutableRun(error) => error.failure_kind(),
            Self::Comparison(_) => FailureKind::ComparisonMismatch,
            Self::ReadFile { .. } | Self::WriteFile { .. } | Self::CreateDir { .. } => {
                FailureKind::InvalidTestCase
            }
            Self::ReadDir { .. }
            | Self::ReadDirEntry { .. }
            | Self::EmptyCorpus { .. }
            | Self::Usage
            | Self::Json(_)
            | Self::CorpusFailures(_) => FailureKind::InvalidTestCase,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage => write!(
                formatter,
                "usage: btbc-cli check-m1 | check-fixture <case.json> <expected.json> | check-executable <manifest.json> <expected.json> | check-mach-o <binary> <expected.json> | check-mach-o-host-traps <binary> <host-traps.json> <expected.json> | check-corpus <cases-dir> <expected-dir> [--out <dir>] | probe-binary <path> | check-binary-probe <binary> <expected.json> | emit-fixture-arm64 <case.json> <out.bin> | link-fixture-arm64-main <case.json> <out-exe> | link-fixture-arm64-stdout-main <case.json> <out-exe> | check-blackbox [--out <dir>]"
            ),
            Self::ReadFile { path, source } => {
                write!(formatter, "failed to read file {}: {source}", path.display())
            }
            Self::WriteFile { path, source } => {
                write!(
                    formatter,
                    "failed to write file {}: {source}",
                    path.display()
                )
            }
            Self::CreateDir { path, source } => {
                write!(
                    formatter,
                    "failed to create directory {}: {source}",
                    path.display()
                )
            }
            Self::ReadDir { path, source } => {
                write!(
                    formatter,
                    "failed to read directory {}: {source}",
                    path.display()
                )
            }
            Self::ReadDirEntry { path, source } => {
                write!(
                    formatter,
                    "failed to read directory entry under {}: {source}",
                    path.display()
                )
            }
            Self::EmptyCorpus { cases_dir } => {
                write!(
                    formatter,
                    "no testcase json files found in {}",
                    cases_dir.display()
                )
            }
            Self::ExecutableManifest(error) => {
                write!(formatter, "executable manifest error: {error}")
            }
            Self::TestCase(error) => write!(formatter, "testcase error: {error}"),
            Self::HostTrapPlan(error) => write!(formatter, "host trap plan error: {error}"),
            Self::ExpectedJson(error) => write!(formatter, "expected json error: {error}"),
            Self::ExpectedProbeJson(error) => {
                write!(formatter, "expected probe json error: {error}")
            }
            Self::BinaryFormatProbe(error) => {
                write!(formatter, "binary format probe error: {error:?}")
            }
            Self::MachOEntryFunctionTestCase(error) => {
                write!(formatter, "mach-o entry function testcase error: {error:?}")
            }
            Self::BinaryProbeComparisonMismatch { expected, actual } => {
                write!(
                    formatter,
                    "binary probe comparison failed: expected {expected:?}, actual {actual:?}"
                )
            }
            Self::FunctionRun(error) => write!(formatter, "function run error: {error}"),
            Self::NativeArtifact(error) => write!(formatter, "native artifact error: {error}"),
            Self::ExecutableRun(error) => write!(formatter, "executable run error: {error}"),
            Self::Comparison(report) => write!(formatter, "comparison failed: {report:?}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::CorpusFailures(report) => match corpus_report_to_json(report) {
                Ok(json) => write!(formatter, "{json}"),
                Err(error) => write!(formatter, "failed to serialize corpus report: {error}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bara_oracle::{
        binary_format_probe_report_from_json, binary_format_probe_report_to_json,
        observed_result_from_json, observed_result_to_json, FailureKind, FailureMessage,
        FixtureOutcome,
    };

    use super::{run_cli, run_m1_check_from_fixtures, CliError};

    const MACH_O_HELLO_WORLD_STDOUT_HOST_TRAPS_JSON: &str = concat!(
        "{\n",
        "  \"host_traps\": [\n",
        "    {\n",
        "      \"kind\": \"stdout\",\n",
        "      \"text\": \"hello world\\n\"\n",
        "    }\n",
        "  ]\n",
        "}\n",
    );
    const MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON: &str = concat!(
        "{\n",
        "  \"case_id\": \"mach_o_hello_world_stdout\",\n",
        "  \"exit_status\": 0,\n",
        "  \"return_value\": 0,\n",
        "  \"stdout\": \"hello world\\n\",\n",
        "  \"stderr\": \"\"\n",
        "}\n",
    );

    #[test]
    fn unknown_command_reports_usage() {
        assert!(matches!(
            run_cli(vec!["unknown".to_owned()]),
            Err(CliError::Usage)
        ));
    }

    #[test]
    fn no_command_reports_usage() {
        assert!(matches!(run_cli(Vec::new()), Err(CliError::Usage)));
    }

    #[test]
    fn check_m1_matches_return_42_fixtures() {
        let output = run_m1_check_from_fixtures(
            include_str!("../../../tests/cases/return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        )
        .expect("return_42 fixture check succeeds on supported host");
        let expected =
            observed_result_from_json(include_str!("../../../tests/expected/return_42.json"))
                .and_then(|result| observed_result_to_json(&result))
                .expect("expected fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_fixture_reads_case_and_expected_files() {
        let temp_dir = TestTempDir::new("check_fixture_reads_case_and_expected_files");
        let case_path = temp_dir.write_file(
            "case.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-fixture"),
            case_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("fixture check succeeds on supported host");
        let expected =
            observed_result_from_json(include_str!("../../../tests/expected/return_42.json"))
                .and_then(|result| observed_result_to_json(&result))
                .expect("expected fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_executable_reads_manifest_and_expected_files() {
        let temp_dir = TestTempDir::new("check_executable_reads_manifest_and_expected_files");
        let manifest_path = temp_dir.write_file(
            "manifest.json",
            include_str!("../../../tests/executables/hello_world_executable_manifest.json"),
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/hello_world_executable_manifest.json"),
        );

        let output = run_cli(vec![
            String::from("check-executable"),
            manifest_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("executable manifest check succeeds on supported host");
        let expected = observed_result_from_json(include_str!(
            "../../../tests/expected/hello_world_executable_manifest.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("expected executable fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_mach_o_reads_binary_and_expected_files() {
        let temp_dir = TestTempDir::new("check_mach_o_reads_binary_and_expected_files");
        let binary_path = temp_dir.write_binary_file(
            "mach_o_return_42.bin",
            include_bytes!("../../../tests/binaries/mach_o_return_42.bin"),
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/mach_o_return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-mach-o"),
            binary_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("mach-o fixture check succeeds on supported host");
        let expected = observed_result_from_json(include_str!(
            "../../../tests/expected/mach_o_return_42.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("expected mach-o fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_mach_o_host_traps_reads_binary_plan_and_expected_files() {
        let temp_dir =
            TestTempDir::new("check_mach_o_host_traps_reads_binary_plan_and_expected_files");
        let binary_path = temp_dir.write_binary_file(
            "mach_o_hello_world_stdout.bin",
            include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
        );
        let host_traps_path =
            temp_dir.write_file("host-traps.json", MACH_O_HELLO_WORLD_STDOUT_HOST_TRAPS_JSON);
        let expected_path =
            temp_dir.write_file("expected.json", MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON);

        let output = run_cli(vec![
            String::from("check-mach-o-host-traps"),
            binary_path.to_string_lossy().into_owned(),
            host_traps_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("mach-o host trap fixture check succeeds on supported host");
        let expected = observed_result_from_json(MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON)
            .and_then(|result| observed_result_to_json(&result))
            .expect("expected mach-o host trap fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_mach_o_host_traps_does_not_derive_plan_from_expected_json() {
        let temp_dir =
            TestTempDir::new("check_mach_o_host_traps_does_not_derive_plan_from_expected_json");
        let binary_path = temp_dir.write_binary_file(
            "mach_o_hello_world_stdout.bin",
            include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
        );
        let host_traps_path = temp_dir.write_file("host-traps.json", "{}");
        let expected_path =
            temp_dir.write_file("expected.json", MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON);

        let error = run_cli(vec![
            String::from("check-mach-o-host-traps"),
            binary_path.to_string_lossy().into_owned(),
            host_traps_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect_err("missing explicit host trap plan fails comparison");

        assert!(matches!(error, CliError::Comparison(_)));
    }

    #[test]
    fn probe_binary_reads_file_and_reports_unsupported_mach_o() {
        let temp_dir = TestTempDir::new("probe_binary_reads_file_and_reports_unsupported_mach_o");
        let binary_path = temp_dir.write_binary_file(
            "fixture.bin",
            &[
                0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ],
        );

        let output = run_cli(vec![
            String::from("probe-binary"),
            binary_path.to_string_lossy().into_owned(),
        ])
        .expect("binary probe succeeds for recognized public binary format");

        assert_eq!(
            output,
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}}}}"
        );
    }

    #[test]
    fn check_binary_probe_compares_probe_report_with_expected_json() {
        let temp_dir =
            TestTempDir::new("check_binary_probe_compares_probe_report_with_expected_json");
        let binary_path = temp_dir.write_binary_file(
            "fixture.bin",
            &[
                0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ],
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            "{\n  \"format\": \"mach_o_64_little_endian\",\n  \"status\": \"recognized_but_unsupported\",\n  \"metadata\": {\n    \"mach_o\": {\n      \"file_type\": \"executable\",\n      \"load_commands\": {\n        \"count\": 0,\n        \"byte_size\": 0,\n        \"recognized_entry_points\": [],\n        \"recognized_segments\": [],\n        \"unsupported_commands\": []\n      },\n      \"executable_image_conversion\": {\n        \"status\": \"not_convertible\",\n        \"blocker\": \"missing_entry_point\"\n      }\n    }\n  }\n}\n",
        );

        let output = run_cli(vec![
            String::from("check-binary-probe"),
            binary_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("binary probe check succeeds");

        assert_eq!(
            output,
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}}}}"
        );
    }

    #[test]
    fn probe_binary_reports_short_input_as_classified_error() {
        let temp_dir = TestTempDir::new("probe_binary_reports_short_input_as_classified_error");
        let binary_path = temp_dir.write_binary_file("fixture.bin", &[0xcf, 0xfa, 0xed]);

        let error = run_cli(vec![
            String::from("probe-binary"),
            binary_path.to_string_lossy().into_owned(),
        ])
        .expect_err("short binary input is classified");

        assert!(matches!(
            error,
            CliError::BinaryFormatProbe(bara_oracle::BinaryFormatProbeError::InputTooShort)
        ));
        assert_eq!(
            error.to_string(),
            "binary format probe error: InputTooShort"
        );
    }

    #[test]
    fn probe_binary_reports_unknown_magic_as_classified_error() {
        let temp_dir = TestTempDir::new("probe_binary_reports_unknown_magic_as_classified_error");
        let binary_path = temp_dir.write_binary_file("fixture.bin", &[0x00, 0x00, 0x00, 0x00]);

        let error = run_cli(vec![
            String::from("probe-binary"),
            binary_path.to_string_lossy().into_owned(),
        ])
        .expect_err("unknown binary magic is classified");

        assert!(matches!(
            error,
            CliError::BinaryFormatProbe(bara_oracle::BinaryFormatProbeError::UnknownMagic)
        ));
        assert_eq!(error.to_string(), "binary format probe error: UnknownMagic");
    }

    #[test]
    fn usage_includes_probe_binary_command() {
        let error = run_cli(Vec::new()).expect_err("missing command reports usage");

        assert!(error
            .to_string()
            .contains("check-mach-o <binary> <expected.json>"));
        assert!(error
            .to_string()
            .contains("check-mach-o-host-traps <binary> <host-traps.json> <expected.json>"));
        assert!(error.to_string().contains("probe-binary <path>"));
        assert!(error
            .to_string()
            .contains("check-binary-probe <binary> <expected.json>"));
        assert!(error
            .to_string()
            .contains("emit-fixture-arm64 <case.json> <out.bin>"));
        assert!(error
            .to_string()
            .contains("link-fixture-arm64-main <case.json> <out-exe>"));
        assert!(error
            .to_string()
            .contains("link-fixture-arm64-stdout-main <case.json> <out-exe>"));
        assert!(error.to_string().contains("check-blackbox [--out <dir>]"));
    }

    #[test]
    fn executable_manifest_run_result_converts_to_observed_result() {
        let manifest = bara_oracle::executable_manifest_from_json(include_str!(
            "../../../tests/executables/hello_world_executable_manifest.json"
        ))
        .expect("executable manifest parses");

        let actual = super::run_executable_manifest(&manifest)
            .expect("executable manifest runs on supported host")
            .into_observed_result();
        let expected = observed_result_from_json(include_str!(
            "../../../tests/expected/hello_world_executable_manifest.json"
        ))
        .expect("expected executable fixture parses");

        assert_eq!(actual, expected);
    }

    #[test]
    fn check_corpus_reports_all_case_json_files_in_order() {
        let temp_dir = TestTempDir::new("check_corpus_reads_all_case_json_files_in_order");
        let cases_dir = temp_dir.create_dir("cases");
        let expected_dir = temp_dir.create_dir("expected");
        write_file(
            &cases_dir.join("return_42.json"),
            include_str!("../../../tests/cases/return_42.json"),
        );
        write_file(
            &expected_dir.join("return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-corpus"),
            cases_dir.to_string_lossy().into_owned(),
            expected_dir.to_string_lossy().into_owned(),
        ])
        .expect("corpus check succeeds on supported host");

        assert_eq!(
            output,
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"}]}"
        );
    }

    #[test]
    fn check_corpus_writes_report_and_actual_outputs() {
        let temp_dir = TestTempDir::new("check_corpus_writes_report_and_actual_outputs");
        let cases_dir = temp_dir.create_dir("cases");
        let expected_dir = temp_dir.create_dir("expected");
        let output_dir = temp_dir.create_dir("out");
        write_file(
            &cases_dir.join("return_42.json"),
            include_str!("../../../tests/cases/return_42.json"),
        );
        write_file(
            &expected_dir.join("return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-corpus"),
            cases_dir.to_string_lossy().into_owned(),
            expected_dir.to_string_lossy().into_owned(),
            String::from("--out"),
            output_dir.to_string_lossy().into_owned(),
        ])
        .expect("corpus check succeeds on supported host");

        assert_eq!(
            output,
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"}]}"
        );
        assert_eq!(
            read_file(&output_dir.join("report.json")),
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"}]}"
        );
        assert_eq!(
            read_file(&output_dir.join("actual").join("return_42.json")),
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}"
        );
        assert!(output_dir.join("compiled").is_dir());
        assert!(output_dir.join("ir").is_dir());
        assert!(output_dir.join("pcmap").is_dir());
    }

    #[test]
    fn check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures() {
        let output = run_cli(vec![String::from("check-blackbox")])
            .expect("blackbox check succeeds on supported host");

        assert_eq!(output, expected_blackbox_report_json());
    }

    #[test]
    fn check_blackbox_writes_report_and_schema_specific_actual_outputs() {
        let temp_dir =
            TestTempDir::new("check_blackbox_writes_report_and_schema_specific_actual_outputs");
        let output_dir = temp_dir.create_dir("out");

        let output = run_cli(vec![
            String::from("check-blackbox"),
            String::from("--out"),
            output_dir.to_string_lossy().into_owned(),
        ])
        .expect("blackbox check succeeds on supported host");

        assert_eq!(output, expected_blackbox_report_json());
        assert_eq!(
            read_file(&output_dir.join("report.json")),
            expected_blackbox_report_json()
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("hello_world_executable_manifest.json")
            ),
            "{\"case_id\":\"hello_world_executable_manifest\",\"exit_status\":0,\"return_value\":0,\"stdout\":\"hello world\\n\",\"stderr\":\"\"}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("mach_o_hello_world_stdout.json")
            ),
            "{\"case_id\":\"mach_o_hello_world_stdout\",\"exit_status\":0,\"return_value\":0,\"stdout\":\"hello world\\n\",\"stderr\":\"\"}"
        );
        let expected_probe = binary_format_probe_report_from_json(include_str!(
            "../../../tests/expected-probes/mach_o_execute_header.json"
        ))
        .and_then(|report| binary_format_probe_report_to_json(&report))
        .expect("expected probe report normalizes to output json");
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("mach_o_execute_header_probe.json")
            ),
            expected_probe
        );
    }

    #[test]
    fn check_corpus_continues_after_failed_case() -> Result<(), String> {
        let temp_dir = TestTempDir::new("check_corpus_continues_after_failed_case");
        let cases_dir = temp_dir.create_dir("cases");
        let expected_dir = temp_dir.create_dir("expected");
        write_file(
            &cases_dir.join("bad_hex.json"),
            r#"{"case_id":"bad_hex","entry":0,"bytes":"cg","abi":{"args":[],"return":"u64"}}"#,
        );
        write_file(
            &cases_dir.join("return_42.json"),
            include_str!("../../../tests/cases/return_42.json"),
        );
        write_file(
            &expected_dir.join("return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        );

        let error = run_cli(vec![
            String::from("check-corpus"),
            cases_dir.to_string_lossy().into_owned(),
            expected_dir.to_string_lossy().into_owned(),
        ])
        .expect_err("corpus check reports failures after scanning every case");

        let report = match error {
            CliError::CorpusFailures(report) => report,
            other => return Err(format!("unexpected error: {other:?}")),
        };

        assert!(!report.is_success());
        assert_eq!(report.fixtures().len(), 2);
        assert_eq!(report.fixtures()[0].case_id().as_str(), "bad_hex");
        assert_eq!(
            report.fixtures()[0].outcome(),
            &FixtureOutcome::failed(
                FailureKind::InvalidTestCase,
                FailureMessage::from("invalid hex digit at index 1")
            )
        );
        assert_eq!(report.fixtures()[1].case_id().as_str(), "return_42");
        assert_eq!(report.fixtures()[1].outcome(), &FixtureOutcome::Passed);

        Ok(())
    }

    struct TestTempDir {
        path: PathBuf,
    }

    impl TestTempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("bara-{name}-{nanos}"));
            fs::create_dir(&path).expect("test temp dir is created");
            Self { path }
        }

        fn create_dir(&self, name: &str) -> PathBuf {
            let path = self.path.join(name);
            fs::create_dir(&path).expect("test child dir is created");
            path
        }

        fn write_file(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.path.join(name);
            write_file(&path, contents);
            path
        }

        fn write_binary_file(&self, name: &str, contents: &[u8]) -> PathBuf {
            let path = self.path.join(name);
            fs::write(&path, contents).expect("test binary fixture file is written");
            path
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).expect("test temp dir is removed");
        }
    }

    fn write_file(path: &Path, contents: &str) {
        fs::write(path, contents).expect("test fixture file is written");
    }

    fn read_file(path: &Path) -> String {
        fs::read_to_string(path).expect("test fixture file is read")
    }

    fn expected_blackbox_report_json() -> &'static str {
        "{\"fixtures\":[{\"case_id\":\"add_eax_imm32_return_45\",\"outcome\":\"passed\"},{\"case_id\":\"add_eax_imm_return_45\",\"outcome\":\"passed\"},{\"case_id\":\"add_sub_eax_imm_return_40\",\"outcome\":\"passed\"},{\"case_id\":\"branch_eq_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"direct_jmp_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"hello_world_stdout_return_0\",\"outcome\":\"passed\"},{\"case_id\":\"identity_u64\",\"outcome\":\"passed\"},{\"case_id\":\"load_u8_from_rdi_return_72\",\"outcome\":\"passed\"},{\"case_id\":\"return_42\",\"outcome\":\"passed\"},{\"case_id\":\"stdout_trap_return_0\",\"outcome\":\"passed\"},{\"case_id\":\"sub_eax_imm32_return_39\",\"outcome\":\"passed\"},{\"case_id\":\"sub_eax_imm_return_39\",\"outcome\":\"passed\"},{\"case_id\":\"xor_eax_eax_return_0\",\"outcome\":\"passed\"},{\"case_id\":\"xor_then_add_eax_return_7\",\"outcome\":\"passed\"},{\"case_id\":\"return_42_native_executable_smoke\",\"outcome\":\"passed\"},{\"case_id\":\"hello_world_executable_manifest\",\"outcome\":\"passed\"},{\"case_id\":\"entry_offset_return_42_manifest\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_hello_world_stdout\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_execute_header_probe\",\"outcome\":\"passed\"}]}"
    }
}
