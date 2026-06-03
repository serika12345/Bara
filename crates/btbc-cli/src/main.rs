use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
};

use bara_arm64::emit_program;
use bara_isa_x86::{decode_function, lift_decoded_function};
use bara_oracle::{
    compare_observed_results, observed_result_from_json, observed_result_to_json,
    test_case_from_json, ComparisonReport, ExpectedResult, ObservedResult, TestCase,
};
use bara_runtime::run_no_args_u64;

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
        [command, cases_dir, expected_dir] if command == "check-corpus" => {
            run_check_corpus(Path::new(cases_dir), Path::new(expected_dir))
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

fn run_check_corpus(cases_dir: &Path, expected_dir: &Path) -> Result<String, CliError> {
    let case_paths = sorted_case_paths(cases_dir)?;
    if case_paths.is_empty() {
        return Err(CliError::EmptyCorpus {
            cases_dir: cases_dir.to_path_buf(),
        });
    }

    let mut checked_case_ids = Vec::new();
    for case_path in case_paths {
        let case_json = read_text_file(&case_path)?;
        let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
        let expected_path = expected_dir.join(format!("{}.json", test_case.case_id().as_str()));
        let expected_json = read_text_file(&expected_path)?;
        let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
        let case_id = test_case.case_id().as_str().to_owned();

        run_test_case(test_case, expected)?;
        checked_case_ids.push(case_id);
    }

    Ok(format_corpus_success(&checked_case_ids))
}

fn run_test_case(test_case: TestCase, expected: ExpectedResult) -> Result<String, CliError> {
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).map_err(CliError::Decode)?;
    let program = lift_decoded_function(&decoded).map_err(CliError::Lift)?;
    let emitted = emit_program(&program).map_err(CliError::Emit)?;
    let result = run_no_args_u64(emitted.code().bytes()).map_err(CliError::Run)?;

    let actual = ObservedResult::new(
        test_case.case_id().clone(),
        0,
        result.return_value(),
        String::new(),
        String::new(),
    );
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        return Err(CliError::Comparison(comparison));
    }

    observed_result_to_json(&actual).map_err(CliError::Json)
}

fn read_text_file(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::ReadFile {
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

fn format_corpus_success(case_ids: &[String]) -> String {
    let mut lines = case_ids
        .iter()
        .map(|case_id| format!("{case_id}: ok"))
        .collect::<Vec<_>>();
    lines.push(format!("checked {} fixture(s)", case_ids.len()));
    lines.join("\n")
}

#[derive(Debug)]
enum CliError {
    Usage,
    ReadFile { path: PathBuf, source: io::Error },
    ReadDir { path: PathBuf, source: io::Error },
    ReadDirEntry { path: PathBuf, source: io::Error },
    EmptyCorpus { cases_dir: PathBuf },
    TestCase(bara_oracle::TestCaseJsonError),
    ExpectedJson(bara_oracle::JsonError),
    Decode(bara_isa_x86::DecodeError),
    Lift(bara_isa_x86::LiftError),
    Emit(bara_arm64::EmitError),
    Run(bara_runtime::RunError),
    Comparison(ComparisonReport),
    Json(bara_oracle::JsonError),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage => write!(
                formatter,
                "usage: btbc-cli check-m1 | check-fixture <case.json> <expected.json> | check-corpus <cases-dir> <expected-dir>"
            ),
            Self::ReadFile { path, source } => {
                write!(formatter, "failed to read file {}: {source}", path.display())
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
            Self::TestCase(error) => write!(formatter, "testcase error: {error}"),
            Self::ExpectedJson(error) => write!(formatter, "expected json error: {error}"),
            Self::Decode(error) => write!(formatter, "decode error: {error:?}"),
            Self::Lift(error) => write!(formatter, "lift error: {error:?}"),
            Self::Emit(error) => write!(formatter, "emit error: {error:?}"),
            Self::Run(error) => write!(formatter, "run error: {error:?}"),
            Self::Comparison(report) => write!(formatter, "comparison failed: {report:?}"),
            Self::Json(error) => write!(formatter, "{error}"),
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

    use bara_oracle::{observed_result_from_json, observed_result_to_json};

    use super::{format_corpus_success, run_cli, run_m1_check_from_fixtures, CliError};

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
    fn check_corpus_reads_all_case_json_files_in_order() {
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

        assert_eq!(output, "return_42: ok\nchecked 1 fixture(s)");
    }

    #[test]
    fn format_corpus_success_reports_each_case_and_total() {
        assert_eq!(
            format_corpus_success(&[String::from("a"), String::from("b")]),
            "a: ok\nb: ok\nchecked 2 fixture(s)"
        );
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
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).expect("test temp dir is removed");
        }
    }

    fn write_file(path: &Path, contents: &str) {
        fs::write(path, contents).expect("test fixture file is written");
    }
}
