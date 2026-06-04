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
