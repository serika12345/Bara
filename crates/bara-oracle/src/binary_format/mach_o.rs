use super::{input::BinaryInput, probe::BinaryFormatProbeError};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOMetadata {
    file_type: MachOFileType,
    load_commands: MachOLoadCommands,
}

impl MachOMetadata {
    pub const fn new(file_type: MachOFileType, load_commands: MachOLoadCommands) -> Self {
        Self {
            file_type,
            load_commands,
        }
    }

    pub const fn file_type(self) -> MachOFileType {
        self.file_type
    }

    pub const fn load_commands(self) -> MachOLoadCommands {
        self.load_commands
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOFileType {
    Executable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOLoadCommands {
    count: MachOLoadCommandCount,
    byte_size: MachOLoadCommandByteSize,
}

impl MachOLoadCommands {
    pub const fn new(count: MachOLoadCommandCount, byte_size: MachOLoadCommandByteSize) -> Self {
        Self { count, byte_size }
    }

    pub const fn count(self) -> MachOLoadCommandCount {
        self.count
    }

    pub const fn byte_size(self) -> MachOLoadCommandByteSize {
        self.byte_size
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOLoadCommandCount {
    value: u32,
}

impl MachOLoadCommandCount {
    pub(crate) const fn from_public_header_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOLoadCommandByteSize {
    value: u32,
}

impl MachOLoadCommandByteSize {
    pub(crate) const fn from_public_header_value(value: u32) -> Self {
        Self { value }
    }
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
    let load_command_count = input
        .read_little_endian_u32_at(MACH_O_NCMDS_OFFSET)
        .map(MachOLoadCommandCount::from_public_header_value)
        .ok_or(BinaryFormatProbeError::HeaderTooShort)?;
    let load_command_byte_size = input
        .read_little_endian_u32_at(MACH_O_SIZEOFCMDS_OFFSET)
        .map(MachOLoadCommandByteSize::from_public_header_value)
        .ok_or(BinaryFormatProbeError::HeaderTooShort)?;

    Ok(MachOMetadata::new(
        file_type,
        MachOLoadCommands::new(load_command_count, load_command_byte_size),
    ))
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
const MACH_O_NCMDS_OFFSET: usize = 16;
const MACH_O_SIZEOFCMDS_OFFSET: usize = 20;
const MACH_O_MH_EXECUTE: u32 = 0x2;
