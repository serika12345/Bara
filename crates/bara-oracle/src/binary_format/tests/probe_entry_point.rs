use super::*;

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
