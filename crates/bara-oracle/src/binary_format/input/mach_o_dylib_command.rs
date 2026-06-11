use super::{BinaryFormatProbeError, BinaryInput, MachOLoadCommandByteSize};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachODylibImportCommand {
    command: MachODylibImportCommandKind,
    byte_size: MachOLoadCommandByteSize,
    name: MachODylibPath,
    timestamp: MachODylibTimestamp,
    current_version: MachODylibVersion,
    compatibility_version: MachODylibVersion,
}

impl RecognizedMachODylibImportCommand {
    pub const fn new(
        command: MachODylibImportCommandKind,
        byte_size: MachOLoadCommandByteSize,
        name: MachODylibPath,
        timestamp: MachODylibTimestamp,
        current_version: MachODylibVersion,
        compatibility_version: MachODylibVersion,
    ) -> Self {
        Self {
            command,
            byte_size,
            name,
            timestamp,
            current_version,
            compatibility_version,
        }
    }

    pub const fn command(&self) -> MachODylibImportCommandKind {
        self.command
    }

    pub const fn byte_size(&self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn name(&self) -> &MachODylibPath {
        &self.name
    }

    pub const fn timestamp(&self) -> MachODylibTimestamp {
        self.timestamp
    }

    pub const fn current_version(&self) -> MachODylibVersion {
        self.current_version
    }

    pub const fn compatibility_version(&self) -> MachODylibVersion {
        self.compatibility_version
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachODylibImportCommandKind {
    LoadDylib,
    LoadWeakDylib,
    ReexportDylib,
    LazyLoadDylib,
    LoadUpwardDylib,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachODylibPath {
    value: String,
}

impl MachODylibPath {
    fn from_public_null_terminated_bytes(bytes: &[u8]) -> Result<Self, BinaryFormatProbeError> {
        let Some(end) = bytes.iter().position(|byte| *byte == 0) else {
            return Err(BinaryFormatProbeError::DylibPathOutOfBounds);
        };
        let value = std::str::from_utf8(&bytes[..end])
            .map_err(|_| BinaryFormatProbeError::InvalidMachODylibPath)?;

        Ok(Self {
            value: value.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachODylibTimestamp {
    value: u32,
}

impl MachODylibTimestamp {
    const fn from_public_dylib_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachODylibVersion {
    value: u32,
}

impl MachODylibVersion {
    const fn from_public_dylib_value(value: u32) -> Self {
        Self { value }
    }
}

pub(crate) fn validate_dylib_command_byte_size(
    byte_size: usize,
) -> Result<(), BinaryFormatProbeError> {
    if byte_size < MACH_O_DYLIB_COMMAND_WIDTH {
        return Err(BinaryFormatProbeError::LoadCommandTooSmall);
    }

    Ok(())
}

pub(crate) fn parse_dylib_import_command_metadata(
    input: &BinaryInput,
    command_offset: usize,
    command_kind: MachODylibImportCommandKind,
    byte_size: MachOLoadCommandByteSize,
) -> Result<RecognizedMachODylibImportCommand, BinaryFormatProbeError> {
    let name_offset = read_dylib_u32(input, command_offset, MACH_O_DYLIB_NAME_OFFSET)?;
    let name = parse_dylib_path(input, command_offset, byte_size.as_usize(), name_offset)?;
    let timestamp = read_dylib_u32(input, command_offset, MACH_O_DYLIB_TIMESTAMP_OFFSET)
        .map(MachODylibTimestamp::from_public_dylib_value)?;
    let current_version =
        read_dylib_u32(input, command_offset, MACH_O_DYLIB_CURRENT_VERSION_OFFSET)
            .map(MachODylibVersion::from_public_dylib_value)?;
    let compatibility_version = read_dylib_u32(
        input,
        command_offset,
        MACH_O_DYLIB_COMPATIBILITY_VERSION_OFFSET,
    )
    .map(MachODylibVersion::from_public_dylib_value)?;

    Ok(RecognizedMachODylibImportCommand::new(
        command_kind,
        byte_size,
        name,
        timestamp,
        current_version,
        compatibility_version,
    ))
}

fn parse_dylib_path(
    input: &BinaryInput,
    command_offset: usize,
    command_byte_size: usize,
    name_offset: u32,
) -> Result<MachODylibPath, BinaryFormatProbeError> {
    let name_offset = name_offset as usize;
    if name_offset < MACH_O_DYLIB_COMMAND_WIDTH || name_offset >= command_byte_size {
        return Err(BinaryFormatProbeError::DylibPathOutOfBounds);
    }

    let name_start = command_offset
        .checked_add(name_offset)
        .ok_or(BinaryFormatProbeError::DylibPathOutOfBounds)?;
    let max_name_len = command_byte_size
        .checked_sub(name_offset)
        .ok_or(BinaryFormatProbeError::DylibPathOutOfBounds)?;
    input
        .read_bytes_at(name_start, max_name_len)
        .ok_or(BinaryFormatProbeError::DylibPathOutOfBounds)
        .and_then(MachODylibPath::from_public_null_terminated_bytes)
}

fn read_dylib_u32(
    input: &BinaryInput,
    command_offset: usize,
    field_offset: usize,
) -> Result<u32, BinaryFormatProbeError> {
    let offset = command_offset
        .checked_add(field_offset)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    input
        .read_little_endian_u32_at(offset)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
}

const MACH_O_DYLIB_COMMAND_WIDTH: usize = 24;
const MACH_O_DYLIB_NAME_OFFSET: usize = 8;
const MACH_O_DYLIB_TIMESTAMP_OFFSET: usize = 12;
const MACH_O_DYLIB_CURRENT_VERSION_OFFSET: usize = 16;
const MACH_O_DYLIB_COMPATIBILITY_VERSION_OFFSET: usize = 20;
