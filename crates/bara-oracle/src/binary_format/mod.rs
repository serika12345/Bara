mod input;
mod mach_o;
mod probe;

pub use input::{BinaryFileBytes, BinaryInput, BinaryInputError};
pub use mach_o::{
    MachOFileType, MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommands,
    MachOMetadata,
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
        MachOFileType, MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommands,
        MachOMetadata,
    };

    const EMPTY_LOAD_COMMANDS: MachOLoadCommands = MachOLoadCommands::new(
        MachOLoadCommandCount::from_public_header_value(0),
        MachOLoadCommandByteSize::from_public_header_value(0),
    );

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
                    EMPTY_LOAD_COMMANDS
                ))
            ))
        );
    }

    #[test]
    fn reads_mach_o_load_command_header_fields_as_typed_metadata() {
        let input = BinaryInput::from_hex(concat!(
            "cffaedfe07000001030000000200000003000000300000000000000000000000",
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
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
                        MachOLoadCommandByteSize::from_public_header_value(48)
                    )
                ))
            ))
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
                    EMPTY_LOAD_COMMANDS
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
