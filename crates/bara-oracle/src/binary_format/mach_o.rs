use super::{input::BinaryInput, probe::BinaryFormatProbeError};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOMetadata {
    file_type: MachOFileType,
}

impl MachOMetadata {
    pub const fn new(file_type: MachOFileType) -> Self {
        Self { file_type }
    }

    pub const fn file_type(self) -> MachOFileType {
        self.file_type
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOFileType {
    Executable,
}

pub(crate) fn parse_mach_o_64_little_endian_metadata(
    input: &BinaryInput,
) -> Result<MachOMetadata, BinaryFormatProbeError> {
    if !input.has_len_at_least(MACH_O_64_HEADER_WIDTH) {
        return Err(BinaryFormatProbeError::HeaderTooShort);
    }

    let file_type = input
        .read_little_endian_u32_at(MACH_O_FILETYPE_OFFSET)
        .and_then(MachOFileType::from_public_header_value)
        .ok_or(BinaryFormatProbeError::UnsupportedMachOFileType)?;

    Ok(MachOMetadata::new(file_type))
}

impl MachOFileType {
    const fn from_public_header_value(value: u32) -> Option<Self> {
        match value {
            MACH_O_MH_EXECUTE => Some(Self::Executable),
            _ => None,
        }
    }
}

const MACH_O_64_HEADER_WIDTH: usize = 32;
const MACH_O_FILETYPE_OFFSET: usize = 12;
const MACH_O_MH_EXECUTE: u32 = 0x2;
