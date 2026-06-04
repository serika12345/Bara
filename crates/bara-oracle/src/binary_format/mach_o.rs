use super::{
    input::BinaryInput,
    mach_o_load_command::{
        parse_mach_o_load_command_summary, MachOLoadCommandByteSize, MachOLoadCommandCount,
        MachOLoadCommandSummary, MachOLoadCommandTableRange,
    },
    probe::BinaryFormatProbeError,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOMetadata {
    file_type: MachOFileType,
    load_commands: MachOLoadCommands,
    executable_image_conversion: MachOExecutableImageConversion,
}

impl MachOMetadata {
    pub fn new(file_type: MachOFileType, load_commands: MachOLoadCommands) -> Self {
        let executable_image_conversion = classify_executable_image_conversion(&load_commands);
        Self {
            file_type,
            load_commands,
            executable_image_conversion,
        }
    }

    pub const fn file_type(&self) -> MachOFileType {
        self.file_type
    }

    pub const fn load_commands(&self) -> &MachOLoadCommands {
        &self.load_commands
    }

    pub const fn executable_image_conversion(&self) -> &MachOExecutableImageConversion {
        &self.executable_image_conversion
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOFileType {
    Executable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOExecutableImageConversion {
    status: MachOExecutableImageConversionStatus,
    blocker: MachOExecutableImageConversionBlocker,
}

impl MachOExecutableImageConversion {
    pub const fn not_convertible(blocker: MachOExecutableImageConversionBlocker) -> Self {
        Self {
            status: MachOExecutableImageConversionStatus::NotConvertible,
            blocker,
        }
    }

    pub const fn status(self) -> MachOExecutableImageConversionStatus {
        self.status
    }

    pub const fn blocker(self) -> MachOExecutableImageConversionBlocker {
        self.blocker
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOExecutableImageConversionStatus {
    NotConvertible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOExecutableImageConversionBlocker {
    MissingEntryPoint,
    AmbiguousEntryPoint,
    MissingSegment,
    EntryPointOutsideSegment,
    AmbiguousEntrySegment,
    UnsupportedImageMapping,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOLoadCommands {
    count: MachOLoadCommandCount,
    byte_size: MachOLoadCommandByteSize,
    #[serde(flatten)]
    summary: MachOLoadCommandSummary,
}

impl MachOLoadCommands {
    pub const fn new(
        count: MachOLoadCommandCount,
        byte_size: MachOLoadCommandByteSize,
        summary: MachOLoadCommandSummary,
    ) -> Self {
        Self {
            count,
            byte_size,
            summary,
        }
    }

    pub const fn count(&self) -> MachOLoadCommandCount {
        self.count
    }

    pub const fn byte_size(&self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn summary(&self) -> &MachOLoadCommandSummary {
        &self.summary
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
    let load_command_table_range =
        validate_load_command_table_bounds(input, load_command_byte_size)?;
    let load_command_summary =
        parse_mach_o_load_command_summary(input, load_command_table_range, load_command_count)?;

    Ok(MachOMetadata::new(
        file_type,
        MachOLoadCommands::new(
            load_command_count,
            load_command_byte_size,
            load_command_summary,
        ),
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

fn classify_executable_image_conversion(
    load_commands: &MachOLoadCommands,
) -> MachOExecutableImageConversion {
    let summary = load_commands.summary();
    let blocker = match summary.recognized_entry_points() {
        [] => MachOExecutableImageConversionBlocker::MissingEntryPoint,
        [_, _, ..] => MachOExecutableImageConversionBlocker::AmbiguousEntryPoint,
        [_] if summary.recognized_segments().is_empty() => {
            MachOExecutableImageConversionBlocker::MissingSegment
        }
        [_] => match recognized_segments_containing_entry_point(summary) {
            0 => MachOExecutableImageConversionBlocker::EntryPointOutsideSegment,
            1 => MachOExecutableImageConversionBlocker::UnsupportedImageMapping,
            _ => MachOExecutableImageConversionBlocker::AmbiguousEntrySegment,
        },
    };

    MachOExecutableImageConversion::not_convertible(blocker)
}

fn recognized_segments_containing_entry_point(summary: &MachOLoadCommandSummary) -> usize {
    let Some(entry_point) = summary.recognized_entry_points().first() else {
        return 0;
    };

    summary
        .recognized_segments()
        .iter()
        .filter(|segment| {
            segment
                .header()
                .contains_entry_point_file_offset(entry_point.metadata().entryoff())
        })
        .count()
}

fn validate_load_command_table_bounds(
    input: &BinaryInput,
    byte_size: MachOLoadCommandByteSize,
) -> Result<MachOLoadCommandTableRange, BinaryFormatProbeError> {
    let table_end = MACH_O_64_HEADER_WIDTH
        .checked_add(byte_size.as_usize())
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;

    if input.has_len_at_least(table_end) {
        Ok(MachOLoadCommandTableRange::new(
            MACH_O_64_HEADER_WIDTH,
            table_end,
        ))
    } else {
        Err(BinaryFormatProbeError::LoadCommandsOutOfBounds)
    }
}

const MACH_O_64_HEADER_WIDTH: usize = 32;
const MACH_O_FILETYPE_OFFSET: usize = 12;
const MACH_O_NCMDS_OFFSET: usize = 16;
const MACH_O_SIZEOFCMDS_OFFSET: usize = 20;
const MACH_O_MH_EXECUTE: u32 = 0x2;
