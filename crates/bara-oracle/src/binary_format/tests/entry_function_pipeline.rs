use crate::{
    mach_o_entry_function_test_case_with_embedded_host_traps,
    mach_o_entry_function_test_case_with_host_traps, CaseId, TestCaseAbi, TestCaseHostTrapPlan,
    TestCaseStackSize, TestCaseStdoutTrap,
};

use super::*;

#[test]
fn builds_no_args_u64_testcase_from_mach_o_binary_input() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000002000000600000000000000000000000",
        "1900000048000000",
        "5f5f5445585400000000000000000000",
        "0000000001000000",
        "0000000000000000",
        "8000000000000000",
        "0800000000000000",
        "00000000000000000000000000000000",
        "2800008018000000",
        "8200000000000000",
        "0020000000000000",
        "9090b82a000000c3",
    ))
    .expect("hex fixture is valid");
    let case_id = CaseId::new("mach_o_return_42").expect("case id is non-empty");

    let testcase =
        mach_o_entry_function_test_case(case_id.clone(), &input).expect("pipeline succeeds");

    assert_eq!(testcase.case_id(), &case_id);
    assert_eq!(testcase.abi(), &TestCaseAbi::NoArgsU64);
    assert_eq!(
        testcase.x86_bytes().bytes(),
        &[0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3]
    );
    assert!(testcase.host_trap_plan().is_empty());
    assert_eq!(
        testcase.stack_state().size(),
        Some(TestCaseStackSize::from_trusted_nonzero_byte_count(0x2000))
    );
}

#[test]
fn preserves_host_trap_plan_from_mach_o_binary_input() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000002000000600000000000000000000000",
        "1900000048000000",
        "5f5f5445585400000000000000000000",
        "0000000001000000",
        "0000000000000000",
        "8000000000000000",
        "0800000000000000",
        "00000000000000000000000000000000",
        "2800008018000000",
        "8200000000000000",
        "0020000000000000",
        "9090b82a000000c3",
    ))
    .expect("hex fixture is valid");
    let case_id = CaseId::new("mach_o_return_42").expect("case id is non-empty");
    let host_trap_plan = TestCaseHostTrapPlan::stdout(
        TestCaseStdoutTrap::from_text(String::from("hello trap\n"))
            .expect("stdout trap text is valid"),
    );

    let testcase =
        mach_o_entry_function_test_case_with_host_traps(case_id, &input, host_trap_plan.clone())
            .expect("pipeline succeeds");

    assert_eq!(testcase.host_trap_plan(), &host_trap_plan);
}

#[test]
fn derives_stdout_host_trap_plan_from_mach_o_embedded_metadata() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000002000000600000000000000000000000",
        "1900000048000000",
        "5f5f5445585400000000000000000000",
        "0000000001000000",
        "0000000000000000",
        "8000000000000000",
        "1d00000000000000",
        "00000000000000000000000000000000",
        "2800008018000000",
        "9800000000000000",
        "0020000000000000",
        "424152415f5354444f55540068656c6c6f20776f726c640a",
        "0f0b31c0c3",
    ))
    .expect("hex fixture is valid");
    let case_id = CaseId::new("mach_o_hello_world_stdout").expect("case id is non-empty");

    let testcase = mach_o_entry_function_test_case_with_embedded_host_traps(case_id, &input)
        .expect("pipeline derives embedded host trap plan");

    assert_eq!(testcase.abi(), &TestCaseAbi::NoArgsU64);
    assert_eq!(
        testcase.x86_bytes().bytes(),
        &[0x0f, 0x0b, 0x31, 0xc0, 0xc3]
    );
    assert_eq!(
        testcase
            .host_trap_plan()
            .stdout_trap()
            .expect("stdout trap is derived")
            .text(),
        "hello world\n"
    );
    assert_eq!(
        testcase.stack_state().size(),
        Some(TestCaseStackSize::from_trusted_nonzero_byte_count(0x2000))
    );
}

#[test]
fn reports_plan_error_for_not_convertible_mach_o_binary_input() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000001000000180000000000000000000000",
        "2800008018000000",
        "2000000000000000",
        "0020000000000000",
    ))
    .expect("hex fixture is valid");
    let case_id = CaseId::new("mach_o_missing_segment").expect("case id is non-empty");

    assert_eq!(
        mach_o_entry_function_test_case(case_id, &input),
        Err(MachOEntryFunctionTestCaseError::Plan(
            MachOExecutableImagePlanError::NotConvertible {
                blocker: MachOExecutableImageConversionBlocker::MissingSegment
            }
        ))
    );
}
