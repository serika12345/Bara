use bara_arm64::emit_program;
use bara_ir::X86Va;
use bara_isa_x86::{decode_function, lift_decoded_function, X86Bytes};
use bara_oracle::{
    compare_observed_results, observed_result_from_json, test_case_from_json, ObservedResult,
    TestCaseAbi,
};
use bara_runtime::{run_no_args_u64, RunError};

#[test]
fn return_42_decodes_lifts_and_emits_arm64() {
    let input = return_42_x86_bytes();
    let decoded = decode_function(&input).expect("M1 bytes decode");
    let program = lift_decoded_function(&decoded).expect("M1 bytes lift");
    let emitted = emit_program(&program).expect("M1 IR emits");

    assert_eq!(
        emitted.code().bytes(),
        &[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
    );
    assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
}

#[test]
fn return_42_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
        .expect("return_42 test case parses");
    assert_eq!(test_case.abi(), &TestCaseAbi::NoArgsU64);

    let expected =
        observed_result_from_json(include_str!("../../../tests/expected/return_42.json"))
            .expect("return_42 expected result parses");
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).expect("M1 bytes decode");
    let program = lift_decoded_function(&decoded).expect("M1 bytes lift");
    let emitted = emit_program(&program).expect("M1 IR emits");

    match run_no_args_u64(emitted.code().bytes()) {
        Ok(result) => {
            let actual = ObservedResult::new(
                test_case.case_id().clone(),
                0,
                result.return_value(),
                String::new(),
                String::new(),
            );
            assert!(compare_observed_results(&expected, &actual).is_match());
        }
        Err(RunError::ExecutableMemory(error)) if cfg!(not(all(unix, target_arch = "aarch64"))) => {
            assert_eq!(error, bara_runtime::ExecutableMemoryError::UnsupportedHost);
        }
        Err(error) => return Err(format!("unexpected run error: {error:?}")),
    }

    Ok(())
}

#[test]
fn add_eax_imm_return_45_decodes_lifts_and_emits_arm64() {
    let test_case = test_case_from_json(include_str!(
        "../../../tests/cases/add_eax_imm_return_45.json"
    ))
    .expect("add_eax_imm_return_45 test case parses");
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).expect("add eax imm8 bytes decode");
    let program = lift_decoded_function(&decoded).expect("add eax imm8 bytes lift");
    let emitted = emit_program(&program).expect("add eax imm8 IR emits");

    assert_eq!(
        emitted.code().bytes(),
        &[0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6]
    );
    assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
}

#[test]
fn add_eax_imm_return_45_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    let test_case = test_case_from_json(include_str!(
        "../../../tests/cases/add_eax_imm_return_45.json"
    ))
    .expect("add_eax_imm_return_45 test case parses");
    assert_eq!(test_case.abi(), &TestCaseAbi::NoArgsU64);

    let expected = observed_result_from_json(include_str!(
        "../../../tests/expected/add_eax_imm_return_45.json"
    ))
    .expect("add_eax_imm_return_45 expected result parses");
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).expect("add eax imm8 bytes decode");
    let program = lift_decoded_function(&decoded).expect("add eax imm8 bytes lift");
    let emitted = emit_program(&program).expect("add eax imm8 IR emits");

    match run_no_args_u64(emitted.code().bytes()) {
        Ok(result) => {
            let actual = ObservedResult::new(
                test_case.case_id().clone(),
                0,
                result.return_value(),
                String::new(),
                String::new(),
            );
            assert!(compare_observed_results(&expected, &actual).is_match());
        }
        Err(RunError::ExecutableMemory(error)) if cfg!(not(all(unix, target_arch = "aarch64"))) => {
            assert_eq!(error, bara_runtime::ExecutableMemoryError::UnsupportedHost);
        }
        Err(error) => return Err(format!("unexpected run error: {error:?}")),
    }

    Ok(())
}

#[test]
fn add_eax_imm32_return_45_decodes_lifts_and_emits_arm64() {
    let test_case = test_case_from_json(include_str!(
        "../../../tests/cases/add_eax_imm32_return_45.json"
    ))
    .expect("add_eax_imm32_return_45 test case parses");
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).expect("add eax imm32 bytes decode");
    let program = lift_decoded_function(&decoded).expect("add eax imm32 bytes lift");
    let emitted = emit_program(&program).expect("add eax imm32 IR emits");

    assert_eq!(
        emitted.code().bytes(),
        &[0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6]
    );
    assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
}

#[test]
fn add_eax_imm32_return_45_runs_on_supported_aarch64_unix_hosts() -> Result<(), String> {
    let test_case = test_case_from_json(include_str!(
        "../../../tests/cases/add_eax_imm32_return_45.json"
    ))
    .expect("add_eax_imm32_return_45 test case parses");
    assert_eq!(test_case.abi(), &TestCaseAbi::NoArgsU64);

    let expected = observed_result_from_json(include_str!(
        "../../../tests/expected/add_eax_imm32_return_45.json"
    ))
    .expect("add_eax_imm32_return_45 expected result parses");
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).expect("add eax imm32 bytes decode");
    let program = lift_decoded_function(&decoded).expect("add eax imm32 bytes lift");
    let emitted = emit_program(&program).expect("add eax imm32 IR emits");

    match run_no_args_u64(emitted.code().bytes()) {
        Ok(result) => {
            let actual = ObservedResult::new(
                test_case.case_id().clone(),
                0,
                result.return_value(),
                String::new(),
                String::new(),
            );
            assert!(compare_observed_results(&expected, &actual).is_match());
        }
        Err(RunError::ExecutableMemory(error)) if cfg!(not(all(unix, target_arch = "aarch64"))) => {
            assert_eq!(error, bara_runtime::ExecutableMemoryError::UnsupportedHost);
        }
        Err(error) => return Err(format!("unexpected run error: {error:?}")),
    }

    Ok(())
}

fn return_42_x86_bytes() -> X86Bytes {
    test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
        .expect("return_42 test case parses")
        .x86_bytes()
        .clone()
}
