use super::*;

#[test]
fn materializes_mach_o_executable_image_from_planned_segment_bytes() {
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(vec![
        0xcf, 0xfa, 0xed, 0xfe, 0x90, 0x90, 0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3,
    ]));
    let plan = MachOExecutableImagePlan::new(
        MachOSegmentFileRange::new(
            MachOSegmentFileOffset::from_public_segment_value(4),
            MachOSegmentFileSize::from_public_segment_value(8),
        ),
        MachOSegmentVmAddr::from_public_segment_value(0x1000),
        MachOEntryPointSegmentOffset::from_valid_segment_relative_value(2),
        MachOEntryPointVirtualAddress::from_valid_runtime_value(0x1002),
    );

    let image = materialize_mach_o_executable_image(&input, &plan).expect("image is materialized");
    let entry_bytes = image
        .entry_function_bytes()
        .expect("entry bytes are inside segment");

    assert_eq!(entry_bytes.bytes(), &[0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3]);
    assert_eq!(entry_bytes.entry(), bara_ir::X86Va::new(0x1002));
    assert_eq!(
        image.code_segment().x86_bytes().entry(),
        bara_ir::X86Va::new(0x1000)
    );
    assert_eq!(image.entry().offset(), bara_ir::X86Va::new(0x1002));
}

#[test]
fn refuses_to_materialize_mach_o_executable_image_when_plan_range_is_out_of_bounds() {
    let input =
        BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(vec![0; 4]));
    let plan = MachOExecutableImagePlan::new(
        MachOSegmentFileRange::new(
            MachOSegmentFileOffset::from_public_segment_value(2),
            MachOSegmentFileSize::from_public_segment_value(8),
        ),
        MachOSegmentVmAddr::from_public_segment_value(0x1000),
        MachOEntryPointSegmentOffset::from_valid_segment_relative_value(0),
        MachOEntryPointVirtualAddress::from_valid_runtime_value(0x1000),
    );

    assert_eq!(
        materialize_mach_o_executable_image(&input, &plan),
        Err(MachOExecutableImageMaterializationError::SegmentFileRangeOutOfBounds)
    );
}
