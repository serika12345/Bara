use super::*;

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
fn recognizes_mach_o_load_dylib_command_as_import_metadata() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000001000000300000000000000000000000",
        "0c00000030000000",
        "18000000",
        "01000000",
        "02000100",
        "00000100",
        "2f7573722f6c69622f6c6962412e64796c696200",
        "00000000",
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
                        "byte_size": 48,
                        "recognized_entry_points": [],
                        "recognized_segments": [],
                        "recognized_dylib_imports": [
                            {
                                "command": "load_dylib",
                                "byte_size": 48,
                                "name": "/usr/lib/libA.dylib",
                                "timestamp": 1,
                                "current_version": 65538,
                                "compatibility_version": 65536
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
fn rejects_mach_o_load_command_table_outside_input() {
    let input =
        BinaryInput::from_hex("cffaedfe07000001030000000200000001000000010000000000000000000000")
            .expect("hex fixture is valid");

    assert_eq!(
        probe_public_binary_format(&input),
        Err(BinaryFormatProbeError::LoadCommandsOutOfBounds)
    );
}
