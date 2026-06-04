use super::{input::BinaryInput, probe::BinaryFormatProbeError};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOEntryPointCommandMetadata {
    entryoff: MachOEntryPointFileOffset,
    stacksize: MachOEntryPointStackSize,
}

impl MachOEntryPointCommandMetadata {
    pub const fn new(
        entryoff: MachOEntryPointFileOffset,
        stacksize: MachOEntryPointStackSize,
    ) -> Self {
        Self {
            entryoff,
            stacksize,
        }
    }

    pub const fn entryoff(self) -> MachOEntryPointFileOffset {
        self.entryoff
    }

    pub const fn stacksize(self) -> MachOEntryPointStackSize {
        self.stacksize
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOEntryPointFileOffset {
    value: u64,
}

impl MachOEntryPointFileOffset {
    pub(crate) const fn from_public_entry_point_value(value: u64) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOEntryPointStackSize {
    value: u64,
}

impl MachOEntryPointStackSize {
    pub(crate) const fn from_public_entry_point_value(value: u64) -> Self {
        Self { value }
    }
}

pub(crate) fn validate_entry_point_command_byte_size(
    byte_size: usize,
) -> Result<(), BinaryFormatProbeError> {
    if byte_size < MACH_O_ENTRY_POINT_COMMAND_WIDTH {
        return Err(BinaryFormatProbeError::LoadCommandTooSmall);
    }

    Ok(())
}

pub(crate) fn parse_entry_point_command_metadata(
    input: &BinaryInput,
    command_offset: usize,
) -> Result<MachOEntryPointCommandMetadata, BinaryFormatProbeError> {
    let entryoff_offset = command_offset
        .checked_add(MACH_O_ENTRY_POINT_ENTRYOFF_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let stacksize_offset = command_offset
        .checked_add(MACH_O_ENTRY_POINT_STACKSIZE_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;

    let entryoff = input
        .read_little_endian_u64_at(entryoff_offset)
        .map(MachOEntryPointFileOffset::from_public_entry_point_value)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let stacksize = input
        .read_little_endian_u64_at(stacksize_offset)
        .map(MachOEntryPointStackSize::from_public_entry_point_value)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;

    Ok(MachOEntryPointCommandMetadata::new(entryoff, stacksize))
}

const MACH_O_ENTRY_POINT_COMMAND_WIDTH: usize = 24;
const MACH_O_ENTRY_POINT_ENTRYOFF_OFFSET: usize = 8;
const MACH_O_ENTRY_POINT_STACKSIZE_OFFSET: usize = 16;
