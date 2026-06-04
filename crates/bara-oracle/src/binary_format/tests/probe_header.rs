use super::*;

#[test]
fn recognizes_mach_o_64_little_endian_executable_as_unsupported_binary_with_metadata() {
    let input =
        BinaryInput::from_hex("cffaedfe07000001030000000200000000000000000000000000000000000000")
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
fn constructs_binary_input_from_owned_file_bytes() {
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(vec![
        0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
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
    let input =
        BinaryInput::from_hex("cffaedfe07000001030000000600000000000000000000000000000000000000")
            .expect("hex fixture is valid");

    assert_eq!(
        probe_public_binary_format(&input),
        Err(BinaryFormatProbeError::UnsupportedMachOFileType)
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
