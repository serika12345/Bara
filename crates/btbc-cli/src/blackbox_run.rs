use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{
    binary_format_probe_report_from_json, compare_observed_results, corpus_report_to_json,
    observed_result_from_json, CaseId, CorpusReport, FailureKind, ObservedResult,
};

use crate::{
    case_id_from_path, run_check_binary_probe, run_check_executable, run_check_mach_o,
    run_check_mach_o_host_traps, run_corpus_fixture, run_link_fixture_arm64_main,
    run_link_mach_o_arm64_main, run_link_mach_o_arm64_stdout_main, sorted_case_paths,
    write_corpus_outputs, CliError, FixtureRun,
};

pub(crate) fn run_check_blackbox(output_dir: Option<&Path>) -> Result<String, CliError> {
    let mut fixture_runs = Vec::new();
    for case_path in sorted_case_paths(&repo_fixture_path("tests/cases"))? {
        fixture_runs.push(run_corpus_fixture(
            &case_path,
            &repo_fixture_path("tests/expected"),
        ));
    }

    let native_artifact_dir = output_dir.map(|path| path.join("native-artifacts"));
    fixture_runs.extend(
        BLACKBOX_FIXTURES
            .iter()
            .map(|fixture| fixture.run_fixture(native_artifact_dir.as_deref())),
    );

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

const BLACKBOX_FIXTURES: &[BlackboxFixtureSpec] = &[
    BlackboxFixtureSpec::NativeExecutableSmoke {
        case: "tests/cases/return_42.json",
        case_id: "return_42_native_executable_smoke",
        expected_exit_status: 42,
    },
    BlackboxFixtureSpec::Executable {
        manifest: "tests/executables/hello_world_executable_manifest.json",
        expected: "tests/expected/hello_world_executable_manifest.json",
    },
    BlackboxFixtureSpec::Executable {
        manifest: "tests/executables/entry_offset_return_42_manifest.json",
        expected: "tests/expected/entry_offset_return_42_manifest.json",
    },
    BlackboxFixtureSpec::MachO {
        binary: "tests/binaries/mach_o_return_42.bin",
        expected: "tests/expected/mach_o_return_42.json",
    },
    BlackboxFixtureSpec::MachONativeExecutableSmoke {
        binary: "tests/binaries/mach_o_return_42.bin",
        case_id: "mach_o_return_42_native_executable_smoke",
        expected_exit_status: 42,
    },
    BlackboxFixtureSpec::MachOHostTraps {
        binary: "tests/binaries/mach_o_hello_world_stdout.bin",
        host_traps: "tests/host-traps/mach_o_hello_world_stdout.json",
        expected: "tests/expected/mach_o_hello_world_stdout.json",
    },
    BlackboxFixtureSpec::MachOStdoutNativeExecutable {
        binary: "tests/binaries/mach_o_hello_world_stdout.bin",
        host_traps: "tests/host-traps/mach_o_hello_world_stdout.json",
        expected: "tests/expected/mach_o_hello_world_stdout.json",
        case_id: "mach_o_hello_world_stdout_native_executable",
    },
    BlackboxFixtureSpec::BinaryProbe {
        binary: "tests/binaries/mach_o_execute_header.bin",
        expected: "tests/expected-probes/mach_o_execute_header.json",
    },
];

enum BlackboxFixtureSpec {
    NativeExecutableSmoke {
        case: &'static str,
        case_id: &'static str,
        expected_exit_status: i32,
    },
    Executable {
        manifest: &'static str,
        expected: &'static str,
    },
    MachO {
        binary: &'static str,
        expected: &'static str,
    },
    MachONativeExecutableSmoke {
        binary: &'static str,
        case_id: &'static str,
        expected_exit_status: i32,
    },
    MachOHostTraps {
        binary: &'static str,
        host_traps: &'static str,
        expected: &'static str,
    },
    MachOStdoutNativeExecutable {
        binary: &'static str,
        host_traps: &'static str,
        expected: &'static str,
        case_id: &'static str,
    },
    BinaryProbe {
        binary: &'static str,
        expected: &'static str,
    },
}

impl BlackboxFixtureSpec {
    fn run_fixture(&self, native_artifact_dir: Option<&Path>) -> FixtureRun {
        match self {
            Self::NativeExecutableSmoke {
                case,
                case_id,
                expected_exit_status,
            } => run_native_executable_smoke(
                case_id,
                *expected_exit_status,
                native_artifact_dir,
                |artifact_path| {
                    run_link_fixture_arm64_main(&repo_fixture_path(case), artifact_path)
                },
            ),
            Self::Executable { manifest, expected } => {
                let manifest_path = repo_fixture_path(manifest);
                let expected_path = repo_fixture_path(expected);
                run_single_observed_fixture(case_id_from_path(&manifest_path), || {
                    run_check_executable(&manifest_path, &expected_path)
                })
            }
            Self::MachO { binary, expected } => {
                let binary_path = repo_fixture_path(binary);
                let expected_path = repo_fixture_path(expected);
                run_single_observed_fixture(case_id_from_path(&binary_path), || {
                    run_check_mach_o(&binary_path, &expected_path)
                })
            }
            Self::MachONativeExecutableSmoke {
                binary,
                case_id,
                expected_exit_status,
            } => run_native_executable_smoke(
                case_id,
                *expected_exit_status,
                native_artifact_dir,
                |artifact_path| {
                    run_link_mach_o_arm64_main(&repo_fixture_path(binary), artifact_path)
                },
            ),
            Self::MachOHostTraps {
                binary,
                host_traps,
                expected,
            } => {
                let binary_path = repo_fixture_path(binary);
                let host_traps_path = repo_fixture_path(host_traps);
                let expected_path = repo_fixture_path(expected);
                run_single_observed_fixture(case_id_from_path(&binary_path), || {
                    run_check_mach_o_host_traps(&binary_path, &host_traps_path, &expected_path)
                })
            }
            Self::MachOStdoutNativeExecutable {
                binary,
                host_traps,
                expected,
                case_id,
            } => {
                let binary_path = repo_fixture_path(binary);
                let host_traps_path = repo_fixture_path(host_traps);
                let expected_path = repo_fixture_path(expected);
                run_mach_o_stdout_native_executable_fixture(
                    &expected_path,
                    &binary_path,
                    &host_traps_path,
                    case_id,
                    native_artifact_dir,
                )
            }
            Self::BinaryProbe { binary, expected } => {
                let binary_path = repo_fixture_path(binary);
                let expected_path = repo_fixture_path(expected);
                run_single_probe_fixture(
                    case_id_from_path_with_suffix(&binary_path, "_probe"),
                    || run_check_binary_probe(&binary_path, &expected_path),
                )
            }
        }
    }
}

fn run_native_executable_smoke(
    case_id: &str,
    expected_exit_status: i32,
    native_artifact_dir: Option<&Path>,
    link_artifact: impl FnOnce(&Path) -> Result<String, CliError>,
) -> FixtureRun {
    let case_id = CaseId::new(case_id).expect("native smoke case id is non-empty");
    let artifact = match NativeSmokeArtifact::new(&case_id, native_artifact_dir) {
        Ok(artifact) => artifact,
        Err(message) => {
            return FixtureRun::failed(case_id, FailureKind::InvalidTestCase, message);
        }
    };

    if let Err(error) = link_artifact(artifact.path()) {
        return FixtureRun::failed(case_id, error.failure_kind(), error.to_string());
    }

    let output = match Command::new(artifact.path()).output() {
        Ok(output) => output,
        Err(error) => {
            return FixtureRun::failed(
                case_id,
                FailureKind::RunError,
                format!(
                    "failed to run native executable smoke artifact {}: {error}",
                    artifact.path().display()
                ),
            );
        }
    };

    if output.status.code() != Some(expected_exit_status)
        || !output.stdout.is_empty()
        || !output.stderr.is_empty()
    {
        return FixtureRun::failed(
            case_id,
            FailureKind::ComparisonMismatch,
            format!(
                "native executable smoke mismatch: expected exit status {expected_exit_status}, empty stdout, empty stderr; actual exit status {:?}, stdout {:?}, stderr {:?}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        );
    }

    FixtureRun::passed(case_id)
}

struct NativeSmokeArtifact {
    path: PathBuf,
    remove_on_drop: bool,
}

impl NativeSmokeArtifact {
    fn new(case_id: &CaseId, native_artifact_dir: Option<&Path>) -> Result<Self, String> {
        match native_artifact_dir {
            Some(dir) => {
                fs::create_dir_all(dir).map_err(|error| {
                    format!(
                        "failed to create native artifact directory {}: {error}",
                        dir.display()
                    )
                })?;
                Ok(Self {
                    path: dir.join(case_id.as_str()),
                    remove_on_drop: false,
                })
            }
            None => {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| {
                        format!("failed to build native smoke temporary artifact path: {error}")
                    })?
                    .as_nanos();
                Ok(Self {
                    path: std::env::temp_dir().join(format!("bara-{}-{nanos}", case_id.as_str())),
                    remove_on_drop: true,
                })
            }
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for NativeSmokeArtifact {
    fn drop(&mut self) {
        if self.remove_on_drop {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn run_single_observed_fixture(
    fallback_case_id: CaseId,
    run_fixture: impl FnOnce() -> Result<String, CliError>,
) -> FixtureRun {
    match run_fixture() {
        Ok(actual_json) => match observed_result_from_json(&actual_json) {
            Ok(actual) => FixtureRun::passed_observed(actual.case_id().clone(), actual),
            Err(error) => FixtureRun::failed(
                fallback_case_id,
                FailureKind::InvalidTestCase,
                error.to_string(),
            ),
        },
        Err(error) => FixtureRun::failed(fallback_case_id, error.failure_kind(), error.to_string()),
    }
}

fn run_observed_comparison_fixture(
    fallback_case_id: CaseId,
    expected_path: &Path,
    run_fixture: impl FnOnce() -> Result<String, CliError>,
) -> FixtureRun {
    let expected_json = match fs::read_to_string(expected_path) {
        Ok(expected_json) => expected_json,
        Err(error) => {
            return FixtureRun::failed(
                fallback_case_id,
                FailureKind::MissingExpected,
                error.to_string(),
            );
        }
    };
    let expected = match observed_result_from_json(&expected_json) {
        Ok(expected) => expected,
        Err(error) => {
            return FixtureRun::failed(
                fallback_case_id,
                FailureKind::InvalidExpected,
                error.to_string(),
            );
        }
    };
    let expected = observed_result_with_case_id(expected, fallback_case_id.clone());

    let actual_json = match run_fixture() {
        Ok(actual_json) => actual_json,
        Err(error) => {
            return FixtureRun::failed(fallback_case_id, error.failure_kind(), error.to_string());
        }
    };
    let actual = match observed_result_from_json(&actual_json) {
        Ok(actual) => actual,
        Err(error) => {
            return FixtureRun::failed(
                fallback_case_id,
                FailureKind::InvalidTestCase,
                error.to_string(),
            );
        }
    };
    let actual = observed_result_with_case_id(actual, fallback_case_id.clone());
    let comparison = compare_observed_results(&expected, &actual);
    let actual_case_id = actual.case_id().clone();
    if !comparison.is_match() {
        return FixtureRun::failed_with_actual(
            actual_case_id,
            FailureKind::ComparisonMismatch,
            format!("comparison failed: {comparison:?}"),
            actual,
        );
    }

    FixtureRun::passed_observed(actual_case_id, actual)
}

fn run_mach_o_stdout_native_executable_fixture(
    expected_path: &Path,
    binary_path: &Path,
    host_traps_path: &Path,
    case_id: &str,
    native_artifact_dir: Option<&Path>,
) -> FixtureRun {
    let case_id = CaseId::new(case_id).expect("native stdout case id is non-empty");
    let artifact = match NativeSmokeArtifact::new(&case_id, native_artifact_dir) {
        Ok(artifact) => artifact,
        Err(message) => {
            return FixtureRun::failed(case_id, FailureKind::InvalidTestCase, message);
        }
    };

    run_observed_comparison_fixture(case_id.clone(), expected_path, || {
        run_link_mach_o_arm64_stdout_main(binary_path, host_traps_path, artifact.path())
    })
}

fn observed_result_with_case_id(actual: ObservedResult, case_id: CaseId) -> ObservedResult {
    ObservedResult::new(
        case_id,
        actual.exit_status(),
        actual.return_value(),
        actual.stdout().to_owned(),
        actual.stderr().to_owned(),
    )
}

fn run_single_probe_fixture(
    case_id: CaseId,
    run_fixture: impl FnOnce() -> Result<String, CliError>,
) -> FixtureRun {
    match run_fixture() {
        Ok(actual_json) => match binary_format_probe_report_from_json(&actual_json) {
            Ok(actual) => FixtureRun::passed_probe(case_id, actual),
            Err(error) => {
                FixtureRun::failed(case_id, FailureKind::InvalidTestCase, error.to_string())
            }
        },
        Err(error) => FixtureRun::failed(case_id, error.failure_kind(), error.to_string()),
    }
}

fn case_id_from_path_with_suffix(path: &Path, suffix: &str) -> CaseId {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| CaseId::new(format!("{stem}{suffix}")).ok())
        .unwrap_or_else(|| CaseId::new("unknown").expect("fallback case id is non-empty"))
}

fn repo_fixture_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}
