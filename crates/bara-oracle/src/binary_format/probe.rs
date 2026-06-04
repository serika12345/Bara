use super::{
    input::{BinaryInput, BinaryMagic},
    mach_o::{parse_mach_o_64_little_endian_metadata, MachOMetadata},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryFormat {
    #[serde(rename = "mach_o_64_little_endian")]
    MachO64LittleEndian,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryFormatProbeStatus {
    RecognizedButUnsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BinaryFormatProbeReport {
    format: BinaryFormat,
    status: BinaryFormatProbeStatus,
    metadata: BinaryFormatProbeMetadata,
}

impl BinaryFormatProbeReport {
    pub const fn new(
        format: BinaryFormat,
        status: BinaryFormatProbeStatus,
        metadata: BinaryFormatProbeMetadata,
    ) -> Self {
        Self {
            format,
            status,
            metadata,
        }
    }

    pub const fn format(&self) -> BinaryFormat {
        self.format
    }

    pub const fn status(&self) -> BinaryFormatProbeStatus {
        self.status
    }

    pub const fn metadata(&self) -> &BinaryFormatProbeMetadata {
        &self.metadata
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BinaryFormatProbeMetadata {
    mach_o: MachOMetadata,
}

impl BinaryFormatProbeMetadata {
    pub const fn mach_o(mach_o: MachOMetadata) -> Self {
        Self { mach_o }
    }

    pub const fn mach_o_metadata(&self) -> &MachOMetadata {
        &self.mach_o
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormatProbeError {
    InputTooShort,
    HeaderTooShort,
    InvalidMachOSegmentName,
    LoadCommandTooSmall,
    LoadCommandsOutOfBounds,
    UnknownMagic,
    UnsupportedMachOFileType,
}

pub fn probe_public_binary_format(
    input: &BinaryInput,
) -> Result<BinaryFormatProbeReport, BinaryFormatProbeError> {
    if !input.has_magic_width() {
        return Err(BinaryFormatProbeError::InputTooShort);
    }

    if input.starts_with_magic(BinaryMagic::MachO64LittleEndian) {
        let metadata = parse_mach_o_64_little_endian_metadata(input)?;

        return Ok(BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
            BinaryFormatProbeMetadata::mach_o(metadata),
        ));
    }

    Err(BinaryFormatProbeError::UnknownMagic)
}
