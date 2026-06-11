use super::*;

#[test]
fn reads_mach_o_load_command_header_fields_as_typed_metadata() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000003000000300000000000000000000000",
        "01000000100000000000000000000000",
        "04000000100000000000000000000000",
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
                            MachOLoadCommandType::from_public_command_value(4),
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
fn recognizes_mach_o_linkedit_relocation_and_bind_metadata() {
    let input = BinaryInput::from_hex(concat!(
        "cffaedfe07000001030000000200000004000000a80000000000000000000000",
        "020000001800000000010000030000000002000040000000",
        "0b00000050000000",
        "010000000200000003000000040000000500000006000000",
        "0700000008000000090000000a0000000b0000000c000000",
        "0d0000000e0000000f000000100000001100000012000000",
        "2200008030000000",
        "200000000400000028000000050000003000000006000000",
        "38000000070000004000000008000000",
        "34000080100000004800000009000000",
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
                        "count": 4,
                        "byte_size": 168,
                        "recognized_entry_points": [],
                        "recognized_segments": [],
                        "recognized_symbol_tables": [
                            {
                                "byte_size": 24,
                                "symoff": 256,
                                "nsyms": 3,
                                "stroff": 512,
                                "strsize": 64
                            }
                        ],
                        "recognized_dynamic_symbol_tables": [
                            {
                                "byte_size": 80,
                                "ilocalsym": 1,
                                "nlocalsym": 2,
                                "iextdefsym": 3,
                                "nextdefsym": 4,
                                "iundefsym": 5,
                                "nundefsym": 6,
                                "tocoff": 7,
                                "ntoc": 8,
                                "modtaboff": 9,
                                "nmodtab": 10,
                                "extrefsymoff": 11,
                                "nextrefsyms": 12,
                                "indirectsymoff": 13,
                                "nindirectsyms": 14,
                                "extreloff": 15,
                                "nextrel": 16,
                                "locreloff": 17,
                                "nlocrel": 18
                            }
                        ],
                        "recognized_dyld_info": [
                            {
                                "command": "dyld_info_only",
                                "byte_size": 48,
                                "rebase": {"offset": 32, "byte_size": 4},
                                "bind": {"offset": 40, "byte_size": 5},
                                "weak_bind": {"offset": 48, "byte_size": 6},
                                "lazy_bind": {"offset": 56, "byte_size": 7},
                                "export": {"offset": 64, "byte_size": 8}
                            }
                        ],
                        "recognized_linkedit_data": [
                            {
                                "command": "dyld_chained_fixups",
                                "byte_size": 16,
                                "dataoff": 72,
                                "datasize": 9
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
