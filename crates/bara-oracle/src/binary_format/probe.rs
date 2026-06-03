use super::input::{BinaryInput, BinaryMagic};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormat {
    MachO64LittleEndian,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormatProbeStatus {
    RecognizedButUnsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryFormatProbeReport {
    format: BinaryFormat,
    status: BinaryFormatProbeStatus,
}

impl BinaryFormatProbeReport {
    pub const fn new(format: BinaryFormat, status: BinaryFormatProbeStatus) -> Self {
        Self { format, status }
    }

    pub const fn format(self) -> BinaryFormat {
        self.format
    }

    pub const fn status(self) -> BinaryFormatProbeStatus {
        self.status
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormatProbeError {
    InputTooShort,
    UnknownMagic,
}

pub fn probe_public_binary_format(
    input: &BinaryInput,
) -> Result<BinaryFormatProbeReport, BinaryFormatProbeError> {
    if !input.has_magic_width() {
        return Err(BinaryFormatProbeError::InputTooShort);
    }

    if input.starts_with_magic(BinaryMagic::MachO64LittleEndian) {
        return Ok(BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
        ));
    }

    Err(BinaryFormatProbeError::UnknownMagic)
}
