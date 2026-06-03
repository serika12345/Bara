use std::{env, process::ExitCode};

use bara_arm64::emit_program;
use bara_isa_x86::{decode_function, lift_decoded_function};
use bara_oracle::{
    compare_observed_results, observed_result_from_json, observed_result_to_json,
    test_case_from_json, ComparisonReport, ObservedResult,
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

#[derive(Debug)]
enum CliError {
    Usage,
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
            Self::Usage => write!(formatter, "usage: btbc-cli check-m1"),
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
    use bara_oracle::{observed_result_from_json, observed_result_to_json};

    use super::{run_cli, run_m1_check_from_fixtures, CliError};

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
}
