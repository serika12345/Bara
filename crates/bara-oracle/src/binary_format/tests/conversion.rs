use super::*;

#[test]
fn reports_mach_o_with_entry_point_but_no_segment_as_not_convertible() {
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
        conversion.status(),
        MachOExecutableImageConversionStatus::NotConvertible
    );
    assert_eq!(
        conversion.blocker(),
        Some(MachOExecutableImageConversionBlocker::MissingSegment)
    );
}

#[test]
fn reports_mach_o_with_multiple_entry_points_as_ambiguous_not_convertible() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000002000000300000000000000000000000",
        "2800008018000000",
        "2000000000000000",
        "0020000000000000",
        "2800008018000000",
        "2100000000000000",
        "0030000000000000",
    ))
    .expect("hex fixture is valid");

    let report = probe_public_binary_format(&input).expect("probe succeeds");
    let load_commands = report.metadata().mach_o_metadata().load_commands();
    let conversion = report
        .metadata()
        .mach_o_metadata()
        .executable_image_conversion();

    assert_eq!(
        load_commands.summary().recognized_entry_points(),
        &[
            RecognizedMachOEntryPointCommand::new(
                MachOLoadCommandByteSize::from_public_header_value(24),
                MachOEntryPointCommandMetadata::new(
                    MachOEntryPointFileOffset::from_public_entry_point_value(32),
                    MachOEntryPointStackSize::from_public_entry_point_value(0x2000),
                ),
            ),
            RecognizedMachOEntryPointCommand::new(
                MachOLoadCommandByteSize::from_public_header_value(24),
                MachOEntryPointCommandMetadata::new(
                    MachOEntryPointFileOffset::from_public_entry_point_value(33),
                    MachOEntryPointStackSize::from_public_entry_point_value(0x3000),
                ),
            ),
        ]
    );
    assert_eq!(
        conversion.status(),
        MachOExecutableImageConversionStatus::NotConvertible
    );
    assert_eq!(
        conversion.blocker(),
        Some(MachOExecutableImageConversionBlocker::AmbiguousEntryPoint)
    );
}

#[test]
fn reports_mach_o_with_entry_point_outside_segment_as_not_convertible() {
    let metadata = MachOMetadata::new(
        MachOFileType::Executable,
        MachOLoadCommands::new(
            MachOLoadCommandCount::from_public_header_value(2),
            MachOLoadCommandByteSize::from_public_header_value(96),
            MachOLoadCommandSummary::new(
                vec![RecognizedMachOEntryPointCommand::new(
                    MachOLoadCommandByteSize::from_public_header_value(24),
                    MachOEntryPointCommandMetadata::new(
                        MachOEntryPointFileOffset::from_public_entry_point_value(0x1234),
                        MachOEntryPointStackSize::from_public_entry_point_value(0x2000),
                    ),
                )],
                vec![RecognizedMachOSegmentCommand::new(
                    MachOLoadCommandByteSize::from_public_header_value(72),
                    MachOSegmentCommandHeaderMetadata::new(
                        MachOSegmentName::from_public_fixed_field(b"__TEXT\0\0\0\0\0\0\0\0\0\0")
                            .expect("test segment name is valid"),
                        MachOSegmentVmAddr::from_public_segment_value(0x1_0000_0000),
                        MachOSegmentFileOffset::from_public_segment_value(0x1234),
                        MachOSegmentFileSize::from_public_segment_value(0),
                    ),
                )],
                Vec::<UnsupportedMachOLoadCommand>::new(),
            ),
        ),
    );
    let conversion = metadata.executable_image_conversion();

    assert_eq!(
        conversion.status(),
        MachOExecutableImageConversionStatus::NotConvertible
    );
    assert_eq!(
        conversion.blocker(),
        Some(MachOExecutableImageConversionBlocker::EntryPointOutsideSegment)
    );
}

#[test]
fn reports_mach_o_with_entry_point_inside_single_segment_as_convertible_candidate() {
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
            MachOSegmentFileOffset::from_public_segment_value(0),
            MachOSegmentFileSize::from_public_segment_value(0x1235),
        ),
    );
    let metadata = MachOMetadata::new(
        MachOFileType::Executable,
        MachOLoadCommands::new(
            MachOLoadCommandCount::from_public_header_value(2),
            MachOLoadCommandByteSize::from_public_header_value(96),
            MachOLoadCommandSummary::new(
                vec![entry_point],
                vec![segment.clone()],
                Vec::<UnsupportedMachOLoadCommand>::new(),
            ),
        ),
    );
    let conversion = metadata.executable_image_conversion();

    assert_eq!(
        conversion.status(),
        MachOExecutableImageConversionStatus::Convertible
    );
    assert_eq!(conversion.blocker(), None);
    assert_eq!(conversion.entry_point(), Some(entry_point));
    assert_eq!(conversion.segment(), Some(&segment));
}

#[test]
fn reports_mach_o_with_entry_point_inside_multiple_segments_as_ambiguous_not_convertible() {
    let metadata = MachOMetadata::new(
        MachOFileType::Executable,
        MachOLoadCommands::new(
            MachOLoadCommandCount::from_public_header_value(3),
            MachOLoadCommandByteSize::from_public_header_value(168),
            MachOLoadCommandSummary::new(
                vec![RecognizedMachOEntryPointCommand::new(
                    MachOLoadCommandByteSize::from_public_header_value(24),
                    MachOEntryPointCommandMetadata::new(
                        MachOEntryPointFileOffset::from_public_entry_point_value(0x1234),
                        MachOEntryPointStackSize::from_public_entry_point_value(0x2000),
                    ),
                )],
                vec![
                    RecognizedMachOSegmentCommand::new(
                        MachOLoadCommandByteSize::from_public_header_value(72),
                        MachOSegmentCommandHeaderMetadata::new(
                            MachOSegmentName::from_public_fixed_field(
                                b"__TEXT\0\0\0\0\0\0\0\0\0\0",
                            )
                            .expect("test segment name is valid"),
                            MachOSegmentVmAddr::from_public_segment_value(0x1_0000_0000),
                            MachOSegmentFileOffset::from_public_segment_value(0),
                            MachOSegmentFileSize::from_public_segment_value(0x2000),
                        ),
                    ),
                    RecognizedMachOSegmentCommand::new(
                        MachOLoadCommandByteSize::from_public_header_value(72),
                        MachOSegmentCommandHeaderMetadata::new(
                            MachOSegmentName::from_public_fixed_field(
                                b"__DATA\0\0\0\0\0\0\0\0\0\0",
                            )
                            .expect("test segment name is valid"),
                            MachOSegmentVmAddr::from_public_segment_value(0x1_0000_1000),
                            MachOSegmentFileOffset::from_public_segment_value(0x1000),
                            MachOSegmentFileSize::from_public_segment_value(0x1000),
                        ),
                    ),
                ],
                Vec::<UnsupportedMachOLoadCommand>::new(),
            ),
        ),
    );
    let conversion = metadata.executable_image_conversion();

    assert_eq!(
        conversion.status(),
        MachOExecutableImageConversionStatus::NotConvertible
    );
    assert_eq!(
        conversion.blocker(),
        Some(MachOExecutableImageConversionBlocker::AmbiguousEntrySegment)
    );
}

#[test]
fn reports_mach_o_without_entry_point_as_not_convertible_to_executable_image() {
    let input =
        BinaryInput::from_hex("cffaedfe07000001030000000200000000000000000000000000000000000000")
            .expect("hex fixture is valid");

    let report = probe_public_binary_format(&input).expect("probe succeeds");
    let conversion = report
        .metadata()
        .mach_o_metadata()
        .executable_image_conversion();

    assert_eq!(
        conversion.status(),
        MachOExecutableImageConversionStatus::NotConvertible
    );
    assert_eq!(
        conversion.blocker(),
        Some(MachOExecutableImageConversionBlocker::MissingEntryPoint)
    );
}
