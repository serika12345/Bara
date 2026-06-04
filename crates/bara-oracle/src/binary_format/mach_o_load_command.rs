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
    const LC_SEGMENT_64: Self = Self { value: 0x19 };

    pub(crate) const fn from_public_command_value(value: u32) -> Self {
        Self { value }
    }

    const fn is_segment_64(self) -> bool {
        self.value == Self::LC_SEGMENT_64.value
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOLoadCommandSummary {
    #[serde(default)]
    recognized_segments: Vec<RecognizedMachOSegmentCommand>,
    unsupported_commands: Vec<UnsupportedMachOLoadCommand>,
}

impl MachOLoadCommandSummary {
    pub fn empty() -> Self {
        Self {
            recognized_segments: Vec::new(),
            unsupported_commands: Vec::new(),
        }
    }

    pub(crate) fn new<R, U>(recognized_segments: R, unsupported_commands: U) -> Self
    where
        R: Into<Vec<RecognizedMachOSegmentCommand>>,
        U: Into<Vec<UnsupportedMachOLoadCommand>>,
    {
        Self {
            recognized_segments: recognized_segments.into(),
            unsupported_commands: unsupported_commands.into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_unsupported_commands<T>(commands: T) -> Self
    where
        T: Into<Vec<UnsupportedMachOLoadCommand>>,
    {
        Self::new(Vec::new(), commands)
    }

    pub fn recognized_segments(&self) -> &[RecognizedMachOSegmentCommand] {
        &self.recognized_segments
    }

    pub fn unsupported_commands(&self) -> &[UnsupportedMachOLoadCommand] {
        &self.unsupported_commands
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachOSegmentCommand {
    byte_size: MachOLoadCommandByteSize,
}

impl RecognizedMachOSegmentCommand {
    pub const fn new(byte_size: MachOLoadCommandByteSize) -> Self {
        Self { byte_size }
    }

    pub const fn byte_size(self) -> MachOLoadCommandByteSize {
        self.byte_size
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

    let mut recognized_segments = Vec::new();
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

        if command.is_segment_64() {
            recognized_segments.push(RecognizedMachOSegmentCommand::new(byte_size));
        } else {
            unsupported_commands.push(UnsupportedMachOLoadCommand::new(command, byte_size));
        }
        command_offset = command_end;
    }

    Ok(MachOLoadCommandSummary::new(
        recognized_segments,
        unsupported_commands,
    ))
}

const MACH_O_LOAD_COMMAND_ENVELOPE_WIDTH: usize = 8;
const MACH_O_LOAD_COMMAND_CMD_SIZE_OFFSET: usize = 4;
