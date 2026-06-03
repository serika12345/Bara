use super::{input::BinaryInput, probe::BinaryFormatProbeError};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOLoadCommandCount {
    value: u32,
}

impl MachOLoadCommandCount {
    pub(crate) const fn from_public_header_value(value: u32) -> Self {
        Self { value }
    }

    const fn as_u32(self) -> u32 {
        self.value
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

    pub(crate) const fn as_usize(self) -> usize {
        self.value as usize
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOLoadCommandType {
    value: u32,
}

impl MachOLoadCommandType {
    pub(crate) const fn from_public_command_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOLoadCommandSummary {
    unsupported_commands: Vec<UnsupportedMachOLoadCommand>,
}

impl MachOLoadCommandSummary {
    pub fn empty() -> Self {
        Self {
            unsupported_commands: Vec::new(),
        }
    }

    pub(crate) fn from_unsupported_commands<T>(commands: T) -> Self
    where
        T: Into<Vec<UnsupportedMachOLoadCommand>>,
    {
        Self {
            unsupported_commands: commands.into(),
        }
    }

    pub fn unsupported_commands(&self) -> &[UnsupportedMachOLoadCommand] {
        &self.unsupported_commands
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnsupportedMachOLoadCommand {
    command: MachOLoadCommandType,
    byte_size: MachOLoadCommandByteSize,
}

impl UnsupportedMachOLoadCommand {
    pub const fn new(command: MachOLoadCommandType, byte_size: MachOLoadCommandByteSize) -> Self {
        Self { command, byte_size }
    }

    pub const fn command(self) -> MachOLoadCommandType {
        self.command
    }

    pub const fn byte_size(self) -> MachOLoadCommandByteSize {
        self.byte_size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MachOLoadCommandTableRange {
    start: usize,
    end: usize,
}

impl MachOLoadCommandTableRange {
    pub(crate) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

pub(crate) fn parse_mach_o_load_command_summary(
    input: &BinaryInput,
    table_range: MachOLoadCommandTableRange,
    command_count: MachOLoadCommandCount,
) -> Result<MachOLoadCommandSummary, BinaryFormatProbeError> {
    if command_count.as_u32() == 0 {
        return Ok(MachOLoadCommandSummary::empty());
    }

    let mut unsupported_commands = Vec::new();
    let mut command_offset = table_range.start;

    for _ in 0..command_count.as_u32() {
        let envelope_end = command_offset
            .checked_add(MACH_O_LOAD_COMMAND_ENVELOPE_WIDTH)
            .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
        if envelope_end > table_range.end {
            return Err(BinaryFormatProbeError::LoadCommandsOutOfBounds);
        }

        let command = input
            .read_little_endian_u32_at(command_offset)
            .map(MachOLoadCommandType::from_public_command_value)
            .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
        let byte_size = input
            .read_little_endian_u32_at(command_offset + MACH_O_LOAD_COMMAND_CMD_SIZE_OFFSET)
            .map(MachOLoadCommandByteSize::from_public_header_value)
            .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;

        if byte_size.as_usize() < MACH_O_LOAD_COMMAND_ENVELOPE_WIDTH {
            return Err(BinaryFormatProbeError::LoadCommandTooSmall);
        }

        let command_end = command_offset
            .checked_add(byte_size.as_usize())
            .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
        if command_end > table_range.end {
            return Err(BinaryFormatProbeError::LoadCommandsOutOfBounds);
        }

        unsupported_commands.push(UnsupportedMachOLoadCommand::new(command, byte_size));
        command_offset = command_end;
    }

    Ok(MachOLoadCommandSummary::from_unsupported_commands(
        unsupported_commands,
    ))
}

const MACH_O_LOAD_COMMAND_ENVELOPE_WIDTH: usize = 8;
const MACH_O_LOAD_COMMAND_CMD_SIZE_OFFSET: usize = 4;
