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
        MachOExecutableImageConversionBlocker, MachOExecutableImageConversionStatus, MachOFileType,
        MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary,
        MachOLoadCommandType, MachOLoadCommands, MachOMetadata, UnsupportedMachOLoadCommand,
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
            "0000000000000000",
            "3412000000000000",
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
                                    "name": "__TEXT",
                                    "vmaddr": 4294967296_u64,
                                    "fileoff": 0,
                                    "filesize": 4660
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
    fn recognizes_mach_o_entry_point_load_command_metadata() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000180000000000000000000000",
            "2800008018000000",
            "3412000000000000",
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
                                    "entryoff": 4660,
                                    "stacksize": 8192
                                }
                            ],
                            "recognized_segments": [],
                            "unsupported_commands": []
                        },
                        "executable_image_conversion": {
                            "status": "not_convertible",
                            "blocker": "unsupported_image_mapping"
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn reports_mach_o_with_entry_point_as_not_convertible_until_image_mapping_exists() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000001000000180000000000000000000000",
            "2800008018000000",
            "3412000000000000",
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
