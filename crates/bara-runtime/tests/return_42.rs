use bara_arm64::{emit_program, EmittedFunction};
use bara_ir::X86Va;
use bara_isa_x86::{decode_function, lift_decoded_function};
use bara_oracle::{
    compare_observed_results, observed_result_from_json, test_case_from_json, ExpectedResult,
    ObservedResult, TestCase, TestCaseAbi,
};
use bara_runtime::{run_no_args_u64, RunError};

#[test]
fn return_42_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "return_42",
        include_str!("../../../tests/cases/return_42.json"),
        &[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6],
    )
}

#[test]
fn return_42_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "return_42",
        include_str!("../../../tests/cases/return_42.json"),
        include_str!("../../../tests/expected/return_42.json"),
    )
}

#[test]
fn add_eax_imm_return_45_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "add_eax_imm_return_45",
        include_str!("../../../tests/cases/add_eax_imm_return_45.json"),
        &[
            0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6,
        ],
    )
}

#[test]
fn add_eax_imm_return_45_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "add_eax_imm_return_45",
        include_str!("../../../tests/cases/add_eax_imm_return_45.json"),
        include_str!("../../../tests/expected/add_eax_imm_return_45.json"),
    )
}

#[test]
fn add_eax_imm32_return_45_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "add_eax_imm32_return_45",
        include_str!("../../../tests/cases/add_eax_imm32_return_45.json"),
        &[
            0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6,
        ],
    )
}

#[test]
fn add_eax_imm32_return_45_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "add_eax_imm32_return_45",
        include_str!("../../../tests/cases/add_eax_imm32_return_45.json"),
        include_str!("../../../tests/expected/add_eax_imm32_return_45.json"),
    )
}

#[test]
fn sub_eax_imm_return_39_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "sub_eax_imm_return_39",
        include_str!("../../../tests/cases/sub_eax_imm_return_39.json"),
        &[
            0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0xd1, 0xc0, 0x03, 0x5f, 0xd6,
        ],
    )
}

#[test]
fn sub_eax_imm_return_39_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "sub_eax_imm_return_39",
        include_str!("../../../tests/cases/sub_eax_imm_return_39.json"),
        include_str!("../../../tests/expected/sub_eax_imm_return_39.json"),
    )
}

#[test]
fn sub_eax_imm32_return_39_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "sub_eax_imm32_return_39",
        include_str!("../../../tests/cases/sub_eax_imm32_return_39.json"),
        &[
            0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0xd1, 0xc0, 0x03, 0x5f, 0xd6,
        ],
    )
}

#[test]
fn sub_eax_imm32_return_39_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "sub_eax_imm32_return_39",
        include_str!("../../../tests/cases/sub_eax_imm32_return_39.json"),
        include_str!("../../../tests/expected/sub_eax_imm32_return_39.json"),
    )
}

#[test]
fn add_sub_eax_imm_return_40_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "add_sub_eax_imm_return_40",
        include_str!("../../../tests/cases/add_sub_eax_imm_return_40.json"),
        &[
            0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0x91, 0x00, 0x14, 0x00, 0xd1, 0xc0, 0x03,
            0x5f, 0xd6,
        ],
    )
}

#[test]
fn add_sub_eax_imm_return_40_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "add_sub_eax_imm_return_40",
        include_str!("../../../tests/cases/add_sub_eax_imm_return_40.json"),
        include_str!("../../../tests/expected/add_sub_eax_imm_return_40.json"),
    )
}

fn assert_fixture_emits_arm64(
    case_name: &str,
    test_case_json: &str,
    expected_bytes: &[u8],
) -> Result<(), String> {
    let test_case = read_test_case(case_name, test_case_json)?;
    let emitted = decode_lift_emit(case_name, &test_case)?;

    assert_emitted_bytes_and_source_pc(&emitted, expected_bytes);

    Ok(())
}

fn assert_fixture_runs_like_expected(
    case_name: &str,
    test_case_json: &str,
    expected_json: &str,
) -> Result<(), String> {
    let test_case = read_test_case(case_name, test_case_json)?;
    assert_eq!(test_case.abi(), &TestCaseAbi::NoArgsU64);

    let expected = read_expected_result(case_name, expected_json)?;
    let emitted = decode_lift_emit(case_name, &test_case)?;

    assert_native_run_matches_expected(&test_case, &expected, &emitted)
}

fn read_test_case(case_name: &str, test_case_json: &str) -> Result<TestCase, String> {
    test_case_from_json(test_case_json)
        .map_err(|error| format!("{case_name} test case parses: {error}"))
}

fn read_expected_result(case_name: &str, expected_json: &str) -> Result<ExpectedResult, String> {
    observed_result_from_json(expected_json)
        .map_err(|error| format!("{case_name} expected result parses: {error}"))
}

fn decode_lift_emit(case_name: &str, test_case: &TestCase) -> Result<EmittedFunction, String> {
    let decoded = decode_function(test_case.x86_bytes())
        .map_err(|error| format!("{case_name} bytes decode: {error:?}"))?;
    let program = lift_decoded_function(&decoded)
        .map_err(|error| format!("{case_name} bytes lift: {error:?}"))?;

    emit_program(&program).map_err(|error| format!("{case_name} IR emits: {error:?}"))
}

fn assert_emitted_bytes_and_source_pc(emitted: &EmittedFunction, expected_bytes: &[u8]) {
    assert_eq!(emitted.code().bytes(), expected_bytes);
    assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
}

fn assert_native_run_matches_expected(
    test_case: &TestCase,
    expected: &ExpectedResult,
    emitted: &EmittedFunction,
) -> Result<(), String> {
    match run_no_args_u64(emitted.code().bytes()) {
        Ok(result) => {
            let actual = ObservedResult::new(
                test_case.case_id().clone(),
                0,
                result.return_value(),
                String::new(),
                String::new(),
            );
            assert!(compare_observed_results(expected, &actual).is_match());
        }
        Err(RunError::ExecutableMemory(error)) if cfg!(not(all(unix, target_arch = "aarch64"))) => {
            assert_eq!(error, bara_runtime::ExecutableMemoryError::UnsupportedHost);
        }
        Err(error) => return Err(format!("unexpected run error: {error:?}")),
    }

    Ok(())
}
