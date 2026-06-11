use super::{
    mach_o_dylib_command::{
        parse_dylib_import_command_metadata, validate_dylib_command_byte_size,
        MachODylibImportCommandKind, RecognizedMachODylibImportCommand,
    },
    mach_o_entry_point_command::{
        parse_entry_point_command_metadata, validate_entry_point_command_byte_size,
        MachOEntryPointCommandMetadata,
    },
    mach_o_linkedit_command::{
        parse_dyld_info_command_metadata, parse_dynamic_symbol_table_command_metadata,
        parse_linkedit_data_command_metadata, parse_symbol_table_command_metadata,
        validate_dyld_info_command_byte_size, validate_dynamic_symbol_table_command_byte_size,
        validate_linkedit_data_command_byte_size, validate_symbol_table_command_byte_size,
        MachODyldInfoCommandKind, MachOLinkeditDataCommandKind, RecognizedMachODyldInfoCommand,
        RecognizedMachODynamicSymbolTableCommand, RecognizedMachOLinkeditDataCommand,
        RecognizedMachOSymbolTableCommand,
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
    const LC_DYSYMTAB: Self = Self { value: 0xb };
    const LC_DYLD_CHAINED_FIXUPS: Self = Self { value: 0x80000034 };
    const LC_DYLD_INFO: Self = Self { value: 0x22 };
    const LC_DYLD_INFO_ONLY: Self = Self { value: 0x80000022 };
    const LC_LAZY_LOAD_DYLIB: Self = Self { value: 0x20 };
    const LC_LOAD_DYLIB: Self = Self { value: 0xc };
    const LC_MAIN: Self = Self { value: 0x80000028 };
    const LC_REEXPORT_DYLIB: Self = Self { value: 0x8000001f };
    const LC_SEGMENT_64: Self = Self { value: 0x19 };
    const LC_LOAD_UPWARD_DYLIB: Self = Self { value: 0x80000023 };
    const LC_LOAD_WEAK_DYLIB: Self = Self { value: 0x80000018 };
    const LC_SYMTAB: Self = Self { value: 0x2 };

    pub(crate) const fn from_public_command_value(value: u32) -> Self {
        Self { value }
    }

    const fn dylib_import_kind(self) -> Option<MachODylibImportCommandKind> {
        if self.value == Self::LC_LOAD_DYLIB.value {
            Some(MachODylibImportCommandKind::LoadDylib)
        } else if self.value == Self::LC_LOAD_WEAK_DYLIB.value {
            Some(MachODylibImportCommandKind::LoadWeakDylib)
        } else if self.value == Self::LC_REEXPORT_DYLIB.value {
            Some(MachODylibImportCommandKind::ReexportDylib)
        } else if self.value == Self::LC_LAZY_LOAD_DYLIB.value {
            Some(MachODylibImportCommandKind::LazyLoadDylib)
        } else if self.value == Self::LC_LOAD_UPWARD_DYLIB.value {
            Some(MachODylibImportCommandKind::LoadUpwardDylib)
        } else {
            None
        }
    }

    const fn is_entry_point(self) -> bool {
        self.value == Self::LC_MAIN.value
    }

    const fn dyld_info_kind(self) -> Option<MachODyldInfoCommandKind> {
        if self.value == Self::LC_DYLD_INFO.value {
            Some(MachODyldInfoCommandKind::DyldInfo)
        } else if self.value == Self::LC_DYLD_INFO_ONLY.value {
            Some(MachODyldInfoCommandKind::DyldInfoOnly)
        } else {
            None
        }
    }

    const fn is_dynamic_symbol_table(self) -> bool {
        self.value == Self::LC_DYSYMTAB.value
    }

    const fn linkedit_data_kind(self) -> Option<MachOLinkeditDataCommandKind> {
        if self.value == Self::LC_DYLD_CHAINED_FIXUPS.value {
            Some(MachOLinkeditDataCommandKind::DyldChainedFixups)
        } else {
            None
        }
    }

    const fn is_segment_64(self) -> bool {
        self.value == Self::LC_SEGMENT_64.value
    }

    const fn is_symbol_table(self) -> bool {
        self.value == Self::LC_SYMTAB.value
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOLoadCommandSummary {
    #[serde(default)]
    recognized_entry_points: Vec<RecognizedMachOEntryPointCommand>,
    #[serde(default)]
    recognized_segments: Vec<RecognizedMachOSegmentCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    recognized_dylib_imports: Vec<RecognizedMachODylibImportCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    recognized_symbol_tables: Vec<RecognizedMachOSymbolTableCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    recognized_dynamic_symbol_tables: Vec<RecognizedMachODynamicSymbolTableCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    recognized_dyld_info: Vec<RecognizedMachODyldInfoCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    recognized_linkedit_data: Vec<RecognizedMachOLinkeditDataCommand>,
    unsupported_commands: Vec<UnsupportedMachOLoadCommand>,
}

impl MachOLoadCommandSummary {
    pub fn empty() -> Self {
        Self {
            recognized_entry_points: Vec::new(),
            recognized_segments: Vec::new(),
            recognized_dylib_imports: Vec::new(),
            recognized_symbol_tables: Vec::new(),
            recognized_dynamic_symbol_tables: Vec::new(),
            recognized_dyld_info: Vec::new(),
            recognized_linkedit_data: Vec::new(),
            unsupported_commands: Vec::new(),
        }
    }

    #[cfg(test)]
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
            recognized_dylib_imports: Vec::new(),
            recognized_symbol_tables: Vec::new(),
            recognized_dynamic_symbol_tables: Vec::new(),
            recognized_dyld_info: Vec::new(),
            recognized_linkedit_data: Vec::new(),
            unsupported_commands: unsupported_commands.into(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parsed_commands<E, R, D, S, Y, I, L, U>(
        recognized_entry_points: E,
        recognized_segments: R,
        recognized_dylib_imports: D,
        recognized_symbol_tables: S,
        recognized_dynamic_symbol_tables: Y,
        recognized_dyld_info: I,
        recognized_linkedit_data: L,
        unsupported_commands: U,
    ) -> Self
    where
        E: Into<Vec<RecognizedMachOEntryPointCommand>>,
        R: Into<Vec<RecognizedMachOSegmentCommand>>,
        D: Into<Vec<RecognizedMachODylibImportCommand>>,
        S: Into<Vec<RecognizedMachOSymbolTableCommand>>,
        Y: Into<Vec<RecognizedMachODynamicSymbolTableCommand>>,
        I: Into<Vec<RecognizedMachODyldInfoCommand>>,
        L: Into<Vec<RecognizedMachOLinkeditDataCommand>>,
        U: Into<Vec<UnsupportedMachOLoadCommand>>,
    {
        Self {
            recognized_entry_points: recognized_entry_points.into(),
            recognized_segments: recognized_segments.into(),
            recognized_dylib_imports: recognized_dylib_imports.into(),
            recognized_symbol_tables: recognized_symbol_tables.into(),
            recognized_dynamic_symbol_tables: recognized_dynamic_symbol_tables.into(),
            recognized_dyld_info: recognized_dyld_info.into(),
            recognized_linkedit_data: recognized_linkedit_data.into(),
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

    pub fn recognized_dylib_imports(&self) -> &[RecognizedMachODylibImportCommand] {
        &self.recognized_dylib_imports
    }

    pub fn recognized_symbol_tables(&self) -> &[RecognizedMachOSymbolTableCommand] {
        &self.recognized_symbol_tables
    }

    pub fn recognized_dynamic_symbol_tables(&self) -> &[RecognizedMachODynamicSymbolTableCommand] {
        &self.recognized_dynamic_symbol_tables
    }

    pub fn recognized_dyld_info(&self) -> &[RecognizedMachODyldInfoCommand] {
        &self.recognized_dyld_info
    }

    pub fn recognized_linkedit_data(&self) -> &[RecognizedMachOLinkeditDataCommand] {
        &self.recognized_linkedit_data
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
    let mut recognized_dylib_imports = Vec::new();
    let mut recognized_symbol_tables = Vec::new();
    let mut recognized_dynamic_symbol_tables = Vec::new();
    let mut recognized_dyld_info = Vec::new();
    let mut recognized_linkedit_data = Vec::new();
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
        } else if let Some(dylib_import_kind) = command.dylib_import_kind() {
            validate_dylib_command_byte_size(byte_size.as_usize())?;
            recognized_dylib_imports.push(parse_dylib_import_command_metadata(
                input,
                command_offset,
                dylib_import_kind,
                byte_size,
            )?);
        } else if command.is_symbol_table() {
            validate_symbol_table_command_byte_size(byte_size.as_usize())?;
            recognized_symbol_tables.push(parse_symbol_table_command_metadata(
                input,
                command_offset,
                byte_size,
            )?);
        } else if command.is_dynamic_symbol_table() {
            validate_dynamic_symbol_table_command_byte_size(byte_size.as_usize())?;
            recognized_dynamic_symbol_tables.push(parse_dynamic_symbol_table_command_metadata(
                input,
                command_offset,
                byte_size,
            )?);
        } else if let Some(dyld_info_kind) = command.dyld_info_kind() {
            validate_dyld_info_command_byte_size(byte_size.as_usize())?;
            recognized_dyld_info.push(parse_dyld_info_command_metadata(
                input,
                command_offset,
                dyld_info_kind,
                byte_size,
            )?);
        } else if let Some(linkedit_data_kind) = command.linkedit_data_kind() {
            validate_linkedit_data_command_byte_size(byte_size.as_usize())?;
            recognized_linkedit_data.push(parse_linkedit_data_command_metadata(
                input,
                command_offset,
                linkedit_data_kind,
                byte_size,
            )?);
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

    Ok(MachOLoadCommandSummary::from_parsed_commands(
        recognized_entry_points,
        recognized_segments,
        recognized_dylib_imports,
        recognized_symbol_tables,
        recognized_dynamic_symbol_tables,
        recognized_dyld_info,
        recognized_linkedit_data,
        unsupported_commands,
    ))
}

const MACH_O_LOAD_COMMAND_ENVELOPE_WIDTH: usize = 8;
const MACH_O_LOAD_COMMAND_CMD_SIZE_OFFSET: usize = 4;
