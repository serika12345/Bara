mod input;
mod probe;

pub use input::{BinaryInput, BinaryInputError};
pub use probe::{
    probe_public_binary_format, BinaryFormat, BinaryFormatProbeError, BinaryFormatProbeReport,
    BinaryFormatProbeStatus,
};

#[cfg(test)]
mod tests {
    use super::{
        probe_public_binary_format, BinaryFormat, BinaryFormatProbeError, BinaryFormatProbeReport,
        BinaryFormatProbeStatus, BinaryInput,
    };

    #[test]
    fn recognizes_mach_o_64_little_endian_as_unsupported_binary() {
        let input = BinaryInput::from_hex(
            "cffaedfe00000000000000000000000000000000000000000000000000000000",
        )
        .expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Ok(BinaryFormatProbeReport::new(
                BinaryFormat::MachO64LittleEndian,
                BinaryFormatProbeStatus::RecognizedButUnsupported
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
    fn reports_unknown_magic() {
        let input = BinaryInput::from_hex("00000000").expect("hex fixture is valid");

        assert_eq!(
            probe_public_binary_format(&input),
            Err(BinaryFormatProbeError::UnknownMagic)
        );
    }
}
