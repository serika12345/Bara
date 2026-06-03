use bara_arm64::{emit_program, EmittedFunction, EmittedHostTrapRequests};
use bara_ir::X86Va;
use bara_isa_x86::{decode_function, lift_decoded_function};
use bara_oracle::{
    compare_observed_results, executable_manifest_from_json, observed_result_from_json,
    test_case_from_json, ExpectedResult, ObservedResult, TestCase, TestCaseAbi,
};
use bara_runtime::{
    run_no_args_u64_with_host_traps, run_one_input_memory_ptr, run_one_u64, HostTrapPlan,
    InputMemory, RunArgumentU64, RunError, RunStdout,
};

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

#[test]
fn xor_eax_eax_return_0_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "xor_eax_eax_return_0",
        include_str!("../../../tests/cases/xor_eax_eax_return_0.json"),
        &[0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6],
    )
}

#[test]
fn xor_eax_eax_return_0_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "xor_eax_eax_return_0",
        include_str!("../../../tests/cases/xor_eax_eax_return_0.json"),
        include_str!("../../../tests/expected/xor_eax_eax_return_0.json"),
    )
}

#[test]
fn xor_then_add_eax_return_7_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "xor_then_add_eax_return_7",
        include_str!("../../../tests/cases/xor_then_add_eax_return_7.json"),
        &[
            0x40, 0x05, 0x80, 0xd2, 0x00, 0x00, 0x80, 0xd2, 0x00, 0x1c, 0x00, 0x91, 0xc0, 0x03,
            0x5f, 0xd6,
        ],
    )
}

#[test]
fn xor_then_add_eax_return_7_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "xor_then_add_eax_return_7",
        include_str!("../../../tests/cases/xor_then_add_eax_return_7.json"),
        include_str!("../../../tests/expected/xor_then_add_eax_return_7.json"),
    )
}

#[test]
fn identity_u64_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "identity_u64",
        include_str!("../../../tests/cases/identity_u64.json"),
        &[0xc0, 0x03, 0x5f, 0xd6],
    )
}

#[test]
fn identity_u64_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "identity_u64",
        include_str!("../../../tests/cases/identity_u64.json"),
        include_str!("../../../tests/expected/identity_u64.json"),
    )
}

#[test]
fn load_u8_from_rdi_return_72_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "load_u8_from_rdi_return_72",
        include_str!("../../../tests/cases/load_u8_from_rdi_return_72.json"),
        &[0x00, 0x00, 0x40, 0x39, 0xc0, 0x03, 0x5f, 0xd6],
    )
}

#[test]
fn load_u8_from_rdi_return_72_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "load_u8_from_rdi_return_72",
        include_str!("../../../tests/cases/load_u8_from_rdi_return_72.json"),
        include_str!("../../../tests/expected/load_u8_from_rdi_return_72.json"),
    )
}

#[test]
fn stdout_trap_return_0_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "stdout_trap_return_0",
        include_str!("../../../tests/cases/stdout_trap_return_0.json"),
        include_str!("../../../tests/expected/stdout_trap_return_0.json"),
    )
}

#[test]
fn stdout_metadata_without_sentinel_does_not_emit_stdout() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "stdout_metadata_without_sentinel",
        r#"{
  "case_id": "stdout_metadata_without_sentinel",
  "entry": 0,
  "bytes": "31c0c3",
  "abi": {
    "args": [],
    "return": "u64"
  },
  "host_traps": [
    {
      "kind": "stdout",
      "text": "hello trap\n"
    }
  ]
}"#,
        r#"{
  "case_id": "stdout_metadata_without_sentinel",
  "exit_status": 0,
  "return_value": 0,
  "stdout": "",
  "stderr": ""
}"#,
    )
}

#[test]
fn stdout_trap_return_0_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "stdout_trap_return_0",
        include_str!("../../../tests/cases/stdout_trap_return_0.json"),
        &[0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6],
    )
}

#[test]
fn hello_world_stdout_return_0_decodes_lifts_and_emits_arm64() -> Result<(), String> {
    assert_fixture_emits_arm64(
        "hello_world_stdout_return_0",
        include_str!("../../../tests/cases/hello_world_stdout_return_0.json"),
        &[0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6],
    )
}

#[test]
fn hello_world_stdout_return_0_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    assert_fixture_runs_like_expected(
        "hello_world_stdout_return_0",
        include_str!("../../../tests/cases/hello_world_stdout_return_0.json"),
        include_str!("../../../tests/expected/hello_world_stdout_return_0.json"),
    )
}

#[test]
fn hello_world_executable_manifest_runs_through_raw_function_pipeline() -> Result<(), String> {
    let manifest = executable_manifest_from_json(include_str!(
        "../../../tests/executables/hello_world_executable_manifest.json"
    ))
    .map_err(|error| format!("hello_world_executable_manifest parses: {error}"))?;
    let expected = read_expected_result(
        "hello_world_executable_manifest",
        include_str!("../../../tests/expected/hello_world_executable_manifest.json"),
    )?;
    let test_case = manifest.into_entry_function();
    let emitted = decode_lift_emit("hello_world_executable_manifest", &test_case)?;

    assert_native_run_matches_expected(&test_case, &expected, &emitted)
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
                .map_err(|error| format!("testcase input memory converts: {error:?}"))?;
            run_one_input_memory_ptr(emitted.code().bytes(), memory)
        }
    };

    match result {
        Ok(result) => {
            let actual = ObservedResult::new(
                test_case.case_id().clone(),
                0,
                result.return_value(),
                result.stdout().to_owned(),
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

fn runtime_host_trap_plan(
    plan: &bara_oracle::TestCaseHostTrapPlan,
    requests: &EmittedHostTrapRequests,
) -> Result<HostTrapPlan, String> {
    if !requests.stdout_requested() {
        return Ok(HostTrapPlan::none());
    }

    let Some(stdout) = plan.stdout_trap() else {
        return Ok(HostTrapPlan::none());
    };

    let stdout = RunStdout::from_text(stdout.text().to_owned())
        .map_err(|error| format!("testcase stdout trap converts: {error:?}"))?;
    Ok(HostTrapPlan::stdout(stdout))
}
