use bara_ir::X86Va;
use bara_isa_x86::X86Bytes;

use crate::{
    executable_manifest::{CodeSegment, ExecutableEntry, ExecutableImage},
    mach_o_executable_image_entry_function_with_host_traps, CaseId, TestCaseAbi,
    TestCaseHostTrapPlan, TestCaseStdoutTrap,
};

use super::*;

#[test]
fn converts_mach_o_executable_image_entry_to_no_args_u64_testcase() {
    let image = ExecutableImage::new(
        CodeSegment::from_x86_bytes(
            X86Bytes::new(
                X86Va::new(0),
                vec![0x0f, 0x0b, 0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3],
            )
            .expect("test bytes decode"),
        ),
        ExecutableEntry::new(X86Va::new(2)),
    )
    .expect("entry is inside image");
    let case_id = CaseId::new("mach_o_return_42").expect("case id is non-empty");

    let testcase =
        mach_o_executable_image_entry_function(case_id.clone(), &image).expect("entry converts");

    assert_eq!(testcase.case_id(), &case_id);
    assert_eq!(testcase.abi(), &TestCaseAbi::NoArgsU64);
    assert_eq!(
        testcase.x86_bytes().bytes(),
        &[0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3]
    );
    assert!(testcase.host_trap_plan().is_empty());
    assert!(testcase.stack_state().is_empty());
}

#[test]
fn preserves_host_trap_plan_for_mach_o_executable_image_entry_function() {
    let image = ExecutableImage::new(
        CodeSegment::from_x86_bytes(
            X86Bytes::new(
                X86Va::new(0),
                vec![0x0f, 0x0b, 0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3],
            )
            .expect("test bytes decode"),
        ),
        ExecutableEntry::new(X86Va::new(2)),
    )
    .expect("entry is inside image");
    let case_id = CaseId::new("mach_o_return_42").expect("case id is non-empty");
    let host_trap_plan = TestCaseHostTrapPlan::stdout(
        TestCaseStdoutTrap::from_text(String::from("hello trap\n"))
            .expect("stdout trap text is valid"),
    );

    let testcase = mach_o_executable_image_entry_function_with_host_traps(
        case_id,
        &image,
        host_trap_plan.clone(),
    )
    .expect("entry converts");

    assert_eq!(testcase.host_trap_plan(), &host_trap_plan);
}
