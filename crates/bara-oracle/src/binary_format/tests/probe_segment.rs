use super::*;

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
