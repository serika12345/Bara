use bara_ir::X86Va;
use bara_isa_x86::X86Bytes;

use crate::{
    executable_manifest::{CodeSegment, ExecutableEntry, ExecutableImage},
    CaseId, TestCaseAbi,
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
}
