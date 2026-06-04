use super::*;

#[test]
fn refuses_to_plan_not_convertible_mach_o_executable_image_conversion() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000001000000180000000000000000000000",
        "2800008018000000",
        "2000000000000000",
        "0020000000000000",
    ))
    .expect("hex fixture is valid");

    let report = probe_public_binary_format(&input).expect("probe succeeds");
    let conversion = report
        .metadata()
        .mach_o_metadata()
        .executable_image_conversion();

    assert_eq!(
        plan_mach_o_executable_image(conversion),
        Err(MachOExecutableImagePlanError::NotConvertible {
            blocker: MachOExecutableImageConversionBlocker::MissingSegment
        })
    );
}

#[test]
fn plans_convertible_mach_o_executable_image_candidate() {
    let entry_point = RecognizedMachOEntryPointCommand::new(
        MachOLoadCommandByteSize::from_public_header_value(24),
        MachOEntryPointCommandMetadata::new(
            MachOEntryPointFileOffset::from_public_entry_point_value(0x1234),
            MachOEntryPointStackSize::from_public_entry_point_value(0x2000),
        ),
    );
    let segment = RecognizedMachOSegmentCommand::new(
        MachOLoadCommandByteSize::from_public_header_value(72),
        MachOSegmentCommandHeaderMetadata::new(
            MachOSegmentName::from_public_fixed_field(b"__TEXT\0\0\0\0\0\0\0\0\0\0")
                .expect("test segment name is valid"),
            MachOSegmentVmAddr::from_public_segment_value(0x1_0000_0000),
            MachOSegmentFileOffset::from_public_segment_value(0x1200),
            MachOSegmentFileSize::from_public_segment_value(0x100),
        ),
    );
    let metadata = MachOMetadata::new(
        MachOFileType::Executable,
        MachOLoadCommands::new(
            MachOLoadCommandCount::from_public_header_value(2),
            MachOLoadCommandByteSize::from_public_header_value(96),
            MachOLoadCommandSummary::new(
                vec![entry_point],
                vec![segment],
                Vec::<UnsupportedMachOLoadCommand>::new(),
            ),
        ),
    );
    let conversion = metadata.executable_image_conversion();

    let plan = plan_mach_o_executable_image(conversion).expect("conversion is plannable");

    assert_eq!(
        plan.segment_file_range().offset(),
        MachOSegmentFileOffset::from_public_segment_value(0x1200)
    );
    assert_eq!(
        plan.segment_file_range().size(),
        MachOSegmentFileSize::from_public_segment_value(0x100)
    );
    assert_eq!(
        plan.entry_point_segment_offset(),
        MachOEntryPointSegmentOffset::from_valid_segment_relative_value(0x34)
    );
}
