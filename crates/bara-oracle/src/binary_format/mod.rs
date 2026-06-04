mod input;
mod mach_o;
mod mach_o_entry_point_command;
mod mach_o_load_command;
mod mach_o_segment_command;
mod probe;

pub use input::{BinaryFileBytes, BinaryInput, BinaryInputError};
pub use mach_o::{
    MachOExecutableImageConversion, MachOExecutableImageConversionBlocker,
    MachOExecutableImageConversionStatus, MachOFileType, MachOLoadCommands, MachOMetadata,
};
pub use mach_o_entry_point_command::{
    MachOEntryPointCommandMetadata, MachOEntryPointFileOffset, MachOEntryPointStackSize,
};
pub use mach_o_load_command::{
    MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary, MachOLoadCommandType,
    RecognizedMachOEntryPointCommand, RecognizedMachOSegmentCommand, UnsupportedMachOLoadCommand,
};
pub use mach_o_segment_command::{
    MachOSegmentCommandHeaderMetadata, MachOSegmentFileOffset, MachOSegmentFileSize,
    MachOSegmentName, MachOSegmentVmAddr,
};
pub use probe::{
    probe_public_binary_format, BinaryFormat, BinaryFormatProbeError, BinaryFormatProbeMetadata,
    BinaryFormatProbeReport, BinaryFormatProbeStatus,
};

#[cfg(test)]
mod tests {
    use super::{
        probe_public_binary_format, BinaryFileBytes, BinaryFormat, BinaryFormatProbeError,
        BinaryFormatProbeMetadata, BinaryFormatProbeReport, BinaryFormatProbeStatus, BinaryInput,
        MachOEntryPointCommandMetadata, MachOEntryPointFileOffset, MachOEntryPointStackSize,
        MachOExecutableImageConversionBlocker, MachOExecutableImageConversionStatus, MachOFileType,
        MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary,
        MachOLoadCommandType, MachOLoadCommands, MachOMetadata, MachOSegmentCommandHeaderMetadata,
        MachOSegmentFileOffset, MachOSegmentFileSize, MachOSegmentName, MachOSegmentVmAddr,
        RecognizedMachOEntryPointCommand, RecognizedMachOSegmentCommand,
        UnsupportedMachOLoadCommand,
    };

    fn empty_load_commands() -> MachOLoadCommands {
        MachOLoadCommands::new(
            MachOLoadCommandCount::from_public_header_value(0),
            MachOLoadCommandByteSize::from_public_header_value(0),
            MachOLoadCommandSummary::empty(),
        )
    }

    #[test]
    fn recognizes_mach_o_64_little_endian_executable_as_unsupported_binary_with_metadata() {
        let input = BinaryInput::from_hex(
            "cffaedfe07000001030000000200000000000000000000000000000000000000",
        )
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Ok(BinaryFormatProbeReport::new(
                BinaryFormat::MachO64LittleEndian,
                BinaryFormatProbeStatus::RecognizedButUnsupported,
                BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                    MachOFileType::Executable,
                    empty_load_commands()
                ))
            ))
        );
    }

    #[test]
    fn reads_mach_o_load_command_header_fields_as_typed_metadata() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000003000000300000000000000000000000",
            "01000000100000000000000000000000",
            "02000000100000000000000000000000",
            "03000000100000000000000000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Ok(BinaryFormatProbeReport::new(
                BinaryFormat::MachO64LittleEndian,
                BinaryFormatProbeStatus::RecognizedButUnsupported,
                BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                    MachOFileType::Executable,
                    MachOLoadCommands::new(
                        MachOLoadCommandCount::from_public_header_value(3),
                        MachOLoadCommandByteSize::from_public_header_value(48),
                        MachOLoadCommandSummary::from_unsupported_commands(vec![
                            UnsupportedMachOLoadCommand::new(
                                MachOLoadCommandType::from_public_command_value(1),
                                MachOLoadCommandByteSize::from_public_header_value(16)
                            ),
                            UnsupportedMachOLoadCommand::new(
                                MachOLoadCommandType::from_public_command_value(2),
                                MachOLoadCommandByteSize::from_public_header_value(16)
                            ),
                            UnsupportedMachOLoadCommand::new(
                                MachOLoadCommandType::from_public_command_value(3),
                                MachOLoadCommandByteSize::from_public_header_value(16)
                            )
                        ])
                    )
                ))
            ))
        );
    }

    #[test]
    fn summarizes_unsupported_mach_o_load_command_envelopes() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000080000000000000000000000",
            "0100000008000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Ok(BinaryFormatProbeReport::new(
                BinaryFormat::MachO64LittleEndian,
                BinaryFormatProbeStatus::RecognizedButUnsupported,
                BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                    MachOFileType::Executable,
                    MachOLoadCommands::new(
                        MachOLoadCommandCount::from_public_header_value(1),
                        MachOLoadCommandByteSize::from_public_header_value(8),
                        MachOLoadCommandSummary::from_unsupported_commands(vec![
                            UnsupportedMachOLoadCommand::new(
                                MachOLoadCommandType::from_public_command_value(1),
                                MachOLoadCommandByteSize::from_public_header_value(8)
                            )
                        ])
                    )
                ))
            ))
        );
    }

    #[test]
    fn recognizes_mach_o_segment_64_load_command_kind() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000480000000000000000000000",
            "1900000048000000",
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
        ))
        .expect("hex fixture is valid");

        let report = probe_public_binary_format(&input).expect("probe succeeds");

        assert_eq!(
            serde_json::to_value(report).expect("probe report serializes"),
            serde_json::json!({
                "format": "mach_o_64_little_endian",
                "status": "recognized_but_unsupported",
                "metadata": {
                    "mach_o": {
                        "file_type": "executable",
                        "load_commands": {
                            "count": 1,
                            "byte_size": 72,
                            "recognized_entry_points": [],
                            "recognized_segments": [
                                {
                                    "byte_size": 72,
                                    "name": "",
                                    "vmaddr": 0,
                                    "fileoff": 0,
                                    "filesize": 0
                                }
                            ],
                            "unsupported_commands": []
                        },
                        "executable_image_conversion": {
                            "status": "not_convertible",
                            "blocker": "missing_entry_point"
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn reads_mach_o_segment_64_command_header_metadata_as_typed_values() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000480000000000000000000000",
            "1900000048000000",
            "5f5f5445585400000000000000000000",
            "0000000001000000",
            "0000000000000000",
            "6800000000000000",
            "0400000000000000",
            "00000000000000000000000000000000",
            "2a000000",
        ))
        .expect("hex fixture is valid");

        let report = probe_public_binary_format(&input).expect("probe succeeds");

        assert_eq!(
            serde_json::to_value(report).expect("probe report serializes"),
            serde_json::json!({
                "format": "mach_o_64_little_endian",
                "status": "recognized_but_unsupported",
                "metadata": {
                    "mach_o": {
                        "file_type": "executable",
                        "load_commands": {
                            "count": 1,
                            "byte_size": 72,
                            "recognized_entry_points": [],
                            "recognized_segments": [
                                {
                                    "byte_size": 72,
                                    "name": "__TEXT",
                                    "vmaddr": 4294967296_u64,
                                    "fileoff": 104,
                                    "filesize": 4
                                }
                            ],
                            "unsupported_commands": []
                        },
                        "executable_image_conversion": {
                            "status": "not_convertible",
                            "blocker": "missing_entry_point"
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn accepts_mach_o_segment_64_zero_size_file_range_at_end_of_input() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000480000000000000000000000",
            "1900000048000000",
            "5f5f5445585400000000000000000000",
            "0000000001000000",
            "0000000000000000",
            "6800000000000000",
            "0000000000000000",
            "00000000000000000000000000000000",
        ))
        .expect("hex fixture is valid");

        let report = probe_public_binary_format(&input).expect("probe succeeds");
        let segment = report
            .metadata()
            .mach_o_metadata()
            .load_commands()
            .summary()
            .recognized_segments()
            .first()
            .expect("segment is recognized");

        assert_eq!(
            segment.header().fileoff(),
            MachOSegmentFileOffset::from_public_segment_value(104)
        );
        assert_eq!(
            segment.header().filesize(),
            MachOSegmentFileSize::from_public_segment_value(0)
        );
    }

    #[test]
    fn accepts_mach_o_segment_64_nonzero_file_range_inside_input() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000480000000000000000000000",
            "1900000048000000",
            "5f5f5445585400000000000000000000",
            "0000000001000000",
            "0000000000000000",
            "6800000000000000",
            "0400000000000000",
            "00000000000000000000000000000000",
            "2a000000",
        ))
        .expect("hex fixture is valid");

        let report = probe_public_binary_format(&input).expect("probe succeeds");
        let segment = report
            .metadata()
            .mach_o_metadata()
            .load_commands()
            .summary()
            .recognized_segments()
            .first()
            .expect("segment is recognized");

        assert_eq!(
            segment.header().fileoff(),
            MachOSegmentFileOffset::from_public_segment_value(104)
        );
        assert_eq!(
            segment.header().filesize(),
            MachOSegmentFileSize::from_public_segment_value(4)
        );
    }

    #[test]
    fn rejects_mach_o_segment_64_file_range_outside_input() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000480000000000000000000000",
            "1900000048000000",
            "5f5f5445585400000000000000000000",
            "0000000001000000",
            "0000000000000000",
            "6800000000000000",
            "0100000000000000",
            "00000000000000000000000000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::SegmentFileRangeOutOfBounds)
        );
    }

    #[test]
    fn rejects_mach_o_segment_64_file_range_that_overflows() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000480000000000000000000000",
            "1900000048000000",
            "5f5f5445585400000000000000000000",
            "0000000001000000",
            "0000000000000000",
            "ffffffffffffffff",
            "0100000000000000",
            "00000000000000000000000000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::SegmentFileRangeOutOfBounds)
        );
    }

    #[test]
    fn recognizes_mach_o_entry_point_load_command_metadata() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000180000000000000000000000",
            "2800008018000000",
            "2000000000000000",
            "0020000000000000",
        ))
        .expect("hex fixture is valid");

        let report = probe_public_binary_format(&input).expect("probe succeeds");

        assert_eq!(
            serde_json::to_value(report).expect("probe report serializes"),
            serde_json::json!({
                "format": "mach_o_64_little_endian",
                "status": "recognized_but_unsupported",
                "metadata": {
                    "mach_o": {
                        "file_type": "executable",
                        "load_commands": {
                            "count": 1,
                            "byte_size": 24,
                            "recognized_entry_points": [
                                {
                                    "byte_size": 24,
                                    "entryoff": 32,
                                    "stacksize": 8192
                                }
                            ],
                            "recognized_segments": [],
                            "unsupported_commands": []
                        },
                        "executable_image_conversion": {
                            "status": "not_convertible",
                            "blocker": "missing_segment"
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn accepts_mach_o_entry_point_file_offset_inside_input() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000180000000000000000000000",
            "2800008018000000",
            "2000000000000000",
            "0020000000000000",
        ))
        .expect("hex fixture is valid");

        let report = probe_public_binary_format(&input).expect("probe succeeds");
        let entry_point = report
            .metadata()
            .mach_o_metadata()
            .load_commands()
            .summary()
            .recognized_entry_points()
            .first()
            .expect("entry point is recognized");

        assert_eq!(
            entry_point.metadata().entryoff(),
            MachOEntryPointFileOffset::from_public_entry_point_value(32)
        );
    }

    #[test]
    fn rejects_mach_o_entry_point_file_offset_at_end_of_input() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000180000000000000000000000",
            "2800008018000000",
            "3800000000000000",
            "0020000000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::EntryPointFileOffsetOutOfBounds)
        );
    }

    #[test]
    fn rejects_mach_o_entry_point_file_offset_beyond_input() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000180000000000000000000000",
            "2800008018000000",
            "3900000000000000",
            "0020000000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::EntryPointFileOffsetOutOfBounds)
        );
    }

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
            MachOExecutableImageConversionBlocker::MissingSegment
        );
    }

    #[test]
    fn reports_mach_o_with_entry_point_and_segment_as_not_convertible_until_image_mapping_exists() {
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
                            MachOSegmentName::from_public_fixed_field(
                                b"__TEXT\0\0\0\0\0\0\0\0\0\0",
                            )
                            .expect("test segment name is valid"),
                            MachOSegmentVmAddr::from_public_segment_value(0x1_0000_0000),
                            MachOSegmentFileOffset::from_public_segment_value(0),
                            MachOSegmentFileSize::from_public_segment_value(0x1234),
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
            MachOExecutableImageConversionBlocker::UnsupportedImageMapping
        );
    }

    #[test]
    fn rejects_mach_o_entry_point_command_smaller_than_public_command() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000170000000000000000000000",
            "2800008017000000",
            "3412000000000000",
            "00200000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::LoadCommandTooSmall)
        );
    }

    #[test]
    fn reports_mach_o_without_entry_point_as_not_convertible_to_executable_image() {
        let input = BinaryInput::from_hex(
            "cffaedfe07000001030000000200000000000000000000000000000000000000",
        )
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
            MachOExecutableImageConversionBlocker::MissingEntryPoint
        );
    }

    #[test]
    fn rejects_mach_o_segment_64_command_smaller_than_public_header() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000470000000000000000000000",
            "1900000047000000",
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
            "000000000000000000000000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::LoadCommandTooSmall)
        );
    }

    #[test]
    fn rejects_mach_o_segment_64_name_that_is_not_utf8() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000480000000000000000000000",
            "1900000048000000",
            "ff000000000000000000000000000000",
            "0000000000000000",
            "0000000000000000",
            "0000000000000000",
            "0000000000000000",
            "00000000000000000000000000000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::InvalidMachOSegmentName)
        );
    }

    #[test]
    fn rejects_mach_o_load_command_smaller_than_envelope() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000080000000000000000000000",
            "0100000007000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::LoadCommandTooSmall)
        );
    }

    #[test]
    fn rejects_mach_o_load_command_range_outside_table() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000080000000000000000000000",
            "0100000010000000",
        ))
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::LoadCommandsOutOfBounds)
        );
    }

    #[test]
    fn constructs_binary_input_from_owned_file_bytes() {
        let input =
            BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(vec![
                0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ]));

        assert_eq!(
            probe_public_binary_format(&input),
            Ok(BinaryFormatProbeReport::new(
                BinaryFormat::MachO64LittleEndian,
                BinaryFormatProbeStatus::RecognizedButUnsupported,
                BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                    MachOFileType::Executable,
                    empty_load_commands()
                ))
            ))
        );
    }

    #[test]
    fn rejects_input_shorter_than_magic() {
        let input = BinaryInput::from_hex("cffaed").expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::InputTooShort)
        );
    }

    #[test]
    fn rejects_mach_o_input_shorter_than_header_after_magic() {
        let input = BinaryInput::from_hex("cffaedfe").expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::HeaderTooShort)
        );
    }

    #[test]
    fn reports_unsupported_mach_o_file_type() {
        let input = BinaryInput::from_hex(
            "cffaedfe07000001030000000600000000000000000000000000000000000000",
        )
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::UnsupportedMachOFileType)
        );
    }

    #[test]
    fn rejects_mach_o_load_command_table_outside_input() {
        let input = BinaryInput::from_hex(
            "cffaedfe07000001030000000200000001000000010000000000000000000000",
        )
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::LoadCommandsOutOfBounds)
        );
    }

    #[test]
    fn reports_unknown_magic() {
        let input = BinaryInput::from_hex("00000000").expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::UnknownMagic)
        );
    }
}
