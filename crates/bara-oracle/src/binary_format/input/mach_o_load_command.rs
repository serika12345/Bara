use super::{
    mach_o_entry_point_command::{
        parse_entry_point_command_metadata, validate_entry_point_command_byte_size,
        MachOEntryPointCommandMetadata,
    },
    mach_o_section::{parse_segment_64_sections_metadata, MachOSectionMetadata},
    mach_o_segment_command::{
        parse_segment_64_header_metadata, validate_segment_64_command_byte_size,
        MachOSegmentCommandHeaderMetadata,
    },
    probe::BinaryFormatProbeError,
    BinaryInput,
};

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
    const LC_MAIN: Self = Self { value: 0x80000028 };
    const LC_SEGMENT_64: Self = Self { value: 0x19 };

    pub(crate) const fn from_public_command_value(value: u32) -> Self {
        Self { value }
    }

    const fn is_entry_point(self) -> bool {
        self.value == Self::LC_MAIN.value
    }

    const fn is_segment_64(self) -> bool {
        self.value == Self::LC_SEGMENT_64.value
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOLoadCommandSummary {
    #[serde(default)]
    recognized_entry_points: Vec<RecognizedMachOEntryPointCommand>,
    #[serde(default)]
    recognized_segments: Vec<RecognizedMachOSegmentCommand>,
    unsupported_commands: Vec<UnsupportedMachOLoadCommand>,
}

impl MachOLoadCommandSummary {
    pub fn empty() -> Self {
        Self {
            recognized_entry_points: Vec::new(),
            recognized_segments: Vec::new(),
            unsupported_commands: Vec::new(),
        }
    }

    pub(crate) fn new<E, R, U>(
        recognized_entry_points: E,
        recognized_segments: R,
        unsupported_commands: U,
    ) -> Self
    where
        E: Into<Vec<RecognizedMachOEntryPointCommand>>,
        R: Into<Vec<RecognizedMachOSegmentCommand>>,
        U: Into<Vec<UnsupportedMachOLoadCommand>>,
    {
        Self {
            recognized_entry_points: recognized_entry_points.into(),
            recognized_segments: recognized_segments.into(),
            unsupported_commands: unsupported_commands.into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_unsupported_commands<T>(commands: T) -> Self
    where
        T: Into<Vec<UnsupportedMachOLoadCommand>>,
    {
        Self::new(Vec::new(), Vec::new(), commands)
    }

    pub fn recognized_entry_points(&self) -> &[RecognizedMachOEntryPointCommand] {
        &self.recognized_entry_points
    }

    pub fn recognized_segments(&self) -> &[RecognizedMachOSegmentCommand] {
        &self.recognized_segments
    }

    pub fn unsupported_commands(&self) -> &[UnsupportedMachOLoadCommand] {
        &self.unsupported_commands
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachOEntryPointCommand {
    byte_size: MachOLoadCommandByteSize,
    #[serde(flatten)]
    metadata: MachOEntryPointCommandMetadata,
}

impl RecognizedMachOEntryPointCommand {
    pub const fn new(
        byte_size: MachOLoadCommandByteSize,
        metadata: MachOEntryPointCommandMetadata,
    ) -> Self {
        Self {
            byte_size,
            metadata,
        }
    }

    pub const fn byte_size(self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn metadata(self) -> MachOEntryPointCommandMetadata {
        self.metadata
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachOSegmentCommand {
    byte_size: MachOLoadCommandByteSize,
    #[serde(flatten)]
    header: MachOSegmentCommandHeaderMetadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sections: Vec<MachOSectionMetadata>,
}

impl RecognizedMachOSegmentCommand {
    pub fn new(
        byte_size: MachOLoadCommandByteSize,
        header: MachOSegmentCommandHeaderMetadata,
    ) -> Self {
        Self {
            byte_size,
            header,
            sections: Vec::new(),
        }
    }

    pub(crate) fn with_sections(
        byte_size: MachOLoadCommandByteSize,
        header: MachOSegmentCommandHeaderMetadata,
        sections: Vec<MachOSectionMetadata>,
    ) -> Self {
        Self {
            byte_size,
            header,
            sections,
        }
    }

    pub const fn byte_size(&self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn header(&self) -> &MachOSegmentCommandHeaderMetadata {
        &self.header
    }

    pub fn sections(&self) -> &[MachOSectionMetadata] {
        &self.sections
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

    let mut recognized_entry_points = Vec::new();
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

        if command.is_entry_point() {
            validate_entry_point_command_byte_size(byte_size.as_usize())?;
            recognized_entry_points.push(RecognizedMachOEntryPointCommand::new(
                byte_size,
                parse_entry_point_command_metadata(input, command_offset)?,
            ));
        } else if command.is_segment_64() {
            validate_segment_64_command_byte_size(byte_size.as_usize())?;
            recognized_segments.push(RecognizedMachOSegmentCommand::with_sections(
                byte_size,
                parse_segment_64_header_metadata(input, command_offset)?,
                parse_segment_64_sections_metadata(input, command_offset, byte_size.as_usize())?,
            ));
        } else {
            unsupported_commands.push(UnsupportedMachOLoadCommand::new(command, byte_size));
        }
        command_offset = command_end;
    }

    Ok(MachOLoadCommandSummary::new(
        recognized_entry_points,
        recognized_segments,
        unsupported_commands,
    ))
}

const MACH_O_LOAD_COMMAND_ENVELOPE_WIDTH: usize = 8;
const MACH_O_LOAD_COMMAND_CMD_SIZE_OFFSET: usize = 4;
