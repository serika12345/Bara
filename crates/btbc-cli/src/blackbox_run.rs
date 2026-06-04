use std::path::{Path, PathBuf};

use bara_oracle::{
    binary_format_probe_report_from_json, corpus_report_to_json, observed_result_from_json, CaseId,
    CorpusReport, FailureKind,
};

use crate::{
    case_id_from_path, run_check_binary_probe, run_check_executable, run_check_mach_o,
    run_check_mach_o_host_traps, run_corpus_fixture, sorted_case_paths, write_corpus_outputs,
    CliError, FixtureRun,
};

pub(crate) fn run_check_blackbox(output_dir: Option<&Path>) -> Result<String, CliError> {
    let mut fixture_runs = Vec::new();
    for case_path in sorted_case_paths(&repo_fixture_path("tests/cases"))? {
        fixture_runs.push(run_corpus_fixture(
            &case_path,
            &repo_fixture_path("tests/expected"),
        ));
    }

    fixture_runs.extend(
        BLACKBOX_FIXTURES
            .iter()
            .map(BlackboxFixtureSpec::run_fixture),
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
    BlackboxFixtureSpec::MachOHostTraps {
        binary: "tests/binaries/mach_o_hello_world_stdout.bin",
        host_traps: "tests/host-traps/mach_o_hello_world_stdout.json",
        expected: "tests/expected/mach_o_hello_world_stdout.json",
    },
    BlackboxFixtureSpec::BinaryProbe {
        binary: "tests/binaries/mach_o_execute_header.bin",
        expected: "tests/expected-probes/mach_o_execute_header.json",
    },
];

enum BlackboxFixtureSpec {
    Executable {
        manifest: &'static str,
        expected: &'static str,
    },
    MachO {
        binary: &'static str,
        expected: &'static str,
    },
    MachOHostTraps {
        binary: &'static str,
        host_traps: &'static str,
        expected: &'static str,
    },
    BinaryProbe {
        binary: &'static str,
        expected: &'static str,
    },
}

impl BlackboxFixtureSpec {
    fn run_fixture(&self) -> FixtureRun {
        match self {
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
