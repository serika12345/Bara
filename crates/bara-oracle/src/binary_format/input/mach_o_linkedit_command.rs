use super::{BinaryFormatProbeError, BinaryInput, MachOLoadCommandByteSize};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachOSymbolTableCommand {
    byte_size: MachOLoadCommandByteSize,
    symoff: MachOLinkeditFileOffset,
    nsyms: MachOLinkeditEntryCount,
    stroff: MachOLinkeditFileOffset,
    strsize: MachOLinkeditByteSize,
}

impl RecognizedMachOSymbolTableCommand {
    pub const fn byte_size(&self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn symoff(&self) -> MachOLinkeditFileOffset {
        self.symoff
    }

    pub const fn nsyms(&self) -> MachOLinkeditEntryCount {
        self.nsyms
    }

    pub const fn stroff(&self) -> MachOLinkeditFileOffset {
        self.stroff
    }

    pub const fn strsize(&self) -> MachOLinkeditByteSize {
        self.strsize
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachODynamicSymbolTableCommand {
    byte_size: MachOLoadCommandByteSize,
    ilocalsym: MachOSymbolIndex,
    nlocalsym: MachOLinkeditEntryCount,
    iextdefsym: MachOSymbolIndex,
    nextdefsym: MachOLinkeditEntryCount,
    iundefsym: MachOSymbolIndex,
    nundefsym: MachOLinkeditEntryCount,
    tocoff: MachOLinkeditFileOffset,
    ntoc: MachOLinkeditEntryCount,
    modtaboff: MachOLinkeditFileOffset,
    nmodtab: MachOLinkeditEntryCount,
    extrefsymoff: MachOLinkeditFileOffset,
    nextrefsyms: MachOLinkeditEntryCount,
    indirectsymoff: MachOLinkeditFileOffset,
    nindirectsyms: MachOLinkeditEntryCount,
    extreloff: MachOLinkeditFileOffset,
    nextrel: MachOLinkeditEntryCount,
    locreloff: MachOLinkeditFileOffset,
    nlocrel: MachOLinkeditEntryCount,
}

impl RecognizedMachODynamicSymbolTableCommand {
    pub const fn byte_size(&self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn extreloff(&self) -> MachOLinkeditFileOffset {
        self.extreloff
    }

    pub const fn nextrel(&self) -> MachOLinkeditEntryCount {
        self.nextrel
    }

    pub const fn locreloff(&self) -> MachOLinkeditFileOffset {
        self.locreloff
    }

    pub const fn nlocrel(&self) -> MachOLinkeditEntryCount {
        self.nlocrel
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachODyldInfoCommand {
    command: MachODyldInfoCommandKind,
    byte_size: MachOLoadCommandByteSize,
    rebase: MachOLinkeditDataRange,
    bind: MachOLinkeditDataRange,
    weak_bind: MachOLinkeditDataRange,
    lazy_bind: MachOLinkeditDataRange,
    export: MachOLinkeditDataRange,
}

impl RecognizedMachODyldInfoCommand {
    pub const fn command(&self) -> MachODyldInfoCommandKind {
        self.command
    }

    pub const fn byte_size(&self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn rebase(&self) -> MachOLinkeditDataRange {
        self.rebase
    }

    pub const fn bind(&self) -> MachOLinkeditDataRange {
        self.bind
    }

    pub const fn weak_bind(&self) -> MachOLinkeditDataRange {
        self.weak_bind
    }

    pub const fn lazy_bind(&self) -> MachOLinkeditDataRange {
        self.lazy_bind
    }

    pub const fn export(&self) -> MachOLinkeditDataRange {
        self.export
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachODyldInfoCommandKind {
    DyldInfo,
    DyldInfoOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecognizedMachOLinkeditDataCommand {
    command: MachOLinkeditDataCommandKind,
    byte_size: MachOLoadCommandByteSize,
    dataoff: MachOLinkeditFileOffset,
    datasize: MachOLinkeditByteSize,
}

impl RecognizedMachOLinkeditDataCommand {
    pub const fn command(&self) -> MachOLinkeditDataCommandKind {
        self.command
    }

    pub const fn byte_size(&self) -> MachOLoadCommandByteSize {
        self.byte_size
    }

    pub const fn dataoff(&self) -> MachOLinkeditFileOffset {
        self.dataoff
    }

    pub const fn datasize(&self) -> MachOLinkeditByteSize {
        self.datasize
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOLinkeditDataCommandKind {
    DyldChainedFixups,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOLinkeditDataRange {
    offset: MachOLinkeditFileOffset,
    byte_size: MachOLinkeditByteSize,
}

impl MachOLinkeditDataRange {
    const fn new(offset: MachOLinkeditFileOffset, byte_size: MachOLinkeditByteSize) -> Self {
        Self { offset, byte_size }
    }

    pub const fn offset(self) -> MachOLinkeditFileOffset {
        self.offset
    }

    pub const fn byte_size(self) -> MachOLinkeditByteSize {
        self.byte_size
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOLinkeditFileOffset {
    value: u32,
}

impl MachOLinkeditFileOffset {
    const fn from_public_linkedit_value(value: u32) -> Self {
        Self { value }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOLinkeditByteSize {
    value: u32,
}

impl MachOLinkeditByteSize {
    const fn from_public_linkedit_value(value: u32) -> Self {
        Self { value }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOLinkeditEntryCount {
    value: u32,
}

impl MachOLinkeditEntryCount {
    const fn from_public_linkedit_value(value: u32) -> Self {
        Self { value }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSymbolIndex {
    value: u32,
}

impl MachOSymbolIndex {
    const fn from_public_linkedit_value(value: u32) -> Self {
        Self { value }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

pub(crate) fn validate_symbol_table_command_byte_size(
    byte_size: usize,
) -> Result<(), BinaryFormatProbeError> {
    validate_minimum_byte_size(byte_size, MACH_O_SYMTAB_COMMAND_WIDTH)
}

pub(crate) fn parse_symbol_table_command_metadata(
    input: &BinaryInput,
    command_offset: usize,
    byte_size: MachOLoadCommandByteSize,
) -> Result<RecognizedMachOSymbolTableCommand, BinaryFormatProbeError> {
    Ok(RecognizedMachOSymbolTableCommand {
        byte_size,
        symoff: read_linkedit_u32_as(
            input,
            command_offset,
            MACH_O_SYMTAB_SYMOFF_OFFSET,
            MachOLinkeditFileOffset::from_public_linkedit_value,
        )?,
        nsyms: read_linkedit_u32_as(
            input,
            command_offset,
            MACH_O_SYMTAB_NSYMS_OFFSET,
            MachOLinkeditEntryCount::from_public_linkedit_value,
        )?,
        stroff: read_linkedit_u32_as(
            input,
            command_offset,
            MACH_O_SYMTAB_STROFF_OFFSET,
            MachOLinkeditFileOffset::from_public_linkedit_value,
        )?,
        strsize: read_linkedit_u32_as(
            input,
            command_offset,
            MACH_O_SYMTAB_STRSIZE_OFFSET,
            MachOLinkeditByteSize::from_public_linkedit_value,
        )?,
    })
}

pub(crate) fn validate_dynamic_symbol_table_command_byte_size(
    byte_size: usize,
) -> Result<(), BinaryFormatProbeError> {
    validate_minimum_byte_size(byte_size, MACH_O_DYSYMTAB_COMMAND_WIDTH)
}

pub(crate) fn parse_dynamic_symbol_table_command_metadata(
    input: &BinaryInput,
    command_offset: usize,
    byte_size: MachOLoadCommandByteSize,
) -> Result<RecognizedMachODynamicSymbolTableCommand, BinaryFormatProbeError> {
    Ok(RecognizedMachODynamicSymbolTableCommand {
        byte_size,
        ilocalsym: read_symbol_index(input, command_offset, MACH_O_DYSYMTAB_ILOCALSYM_OFFSET)?,
        nlocalsym: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NLOCALSYM_OFFSET)?,
        iextdefsym: read_symbol_index(input, command_offset, MACH_O_DYSYMTAB_IEXTDEFSYM_OFFSET)?,
        nextdefsym: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NEXTDEFSYM_OFFSET)?,
        iundefsym: read_symbol_index(input, command_offset, MACH_O_DYSYMTAB_IUNDEFSYM_OFFSET)?,
        nundefsym: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NUNDEFSYM_OFFSET)?,
        tocoff: read_file_offset(input, command_offset, MACH_O_DYSYMTAB_TOCOFF_OFFSET)?,
        ntoc: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NTOC_OFFSET)?,
        modtaboff: read_file_offset(input, command_offset, MACH_O_DYSYMTAB_MODTABOFF_OFFSET)?,
        nmodtab: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NMODTAB_OFFSET)?,
        extrefsymoff: read_file_offset(input, command_offset, MACH_O_DYSYMTAB_EXTREFSYMOFF_OFFSET)?,
        nextrefsyms: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NEXTREFSYMS_OFFSET)?,
        indirectsymoff: read_file_offset(
            input,
            command_offset,
            MACH_O_DYSYMTAB_INDIRECTSYMOFF_OFFSET,
        )?,
        nindirectsyms: read_entry_count(
            input,
            command_offset,
            MACH_O_DYSYMTAB_NINDIRECTSYMS_OFFSET,
        )?,
        extreloff: read_file_offset(input, command_offset, MACH_O_DYSYMTAB_EXTRELOFF_OFFSET)?,
        nextrel: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NEXTREL_OFFSET)?,
        locreloff: read_file_offset(input, command_offset, MACH_O_DYSYMTAB_LOCRELOFF_OFFSET)?,
        nlocrel: read_entry_count(input, command_offset, MACH_O_DYSYMTAB_NLOCREL_OFFSET)?,
    })
}

pub(crate) fn validate_dyld_info_command_byte_size(
    byte_size: usize,
) -> Result<(), BinaryFormatProbeError> {
    validate_minimum_byte_size(byte_size, MACH_O_DYLD_INFO_COMMAND_WIDTH)
}

pub(crate) fn parse_dyld_info_command_metadata(
    input: &BinaryInput,
    command_offset: usize,
    command: MachODyldInfoCommandKind,
    byte_size: MachOLoadCommandByteSize,
) -> Result<RecognizedMachODyldInfoCommand, BinaryFormatProbeError> {
    Ok(RecognizedMachODyldInfoCommand {
        command,
        byte_size,
        rebase: read_data_range(
            input,
            command_offset,
            MACH_O_DYLD_INFO_REBASE_OFF_OFFSET,
            MACH_O_DYLD_INFO_REBASE_SIZE_OFFSET,
        )?,
        bind: read_data_range(
            input,
            command_offset,
            MACH_O_DYLD_INFO_BIND_OFF_OFFSET,
            MACH_O_DYLD_INFO_BIND_SIZE_OFFSET,
        )?,
        weak_bind: read_data_range(
            input,
            command_offset,
            MACH_O_DYLD_INFO_WEAK_BIND_OFF_OFFSET,
            MACH_O_DYLD_INFO_WEAK_BIND_SIZE_OFFSET,
        )?,
        lazy_bind: read_data_range(
            input,
            command_offset,
            MACH_O_DYLD_INFO_LAZY_BIND_OFF_OFFSET,
            MACH_O_DYLD_INFO_LAZY_BIND_SIZE_OFFSET,
        )?,
        export: read_data_range(
            input,
            command_offset,
            MACH_O_DYLD_INFO_EXPORT_OFF_OFFSET,
            MACH_O_DYLD_INFO_EXPORT_SIZE_OFFSET,
        )?,
    })
}

pub(crate) fn validate_linkedit_data_command_byte_size(
    byte_size: usize,
) -> Result<(), BinaryFormatProbeError> {
    validate_minimum_byte_size(byte_size, MACH_O_LINKEDIT_DATA_COMMAND_WIDTH)
}

pub(crate) fn parse_linkedit_data_command_metadata(
    input: &BinaryInput,
    command_offset: usize,
    command: MachOLinkeditDataCommandKind,
    byte_size: MachOLoadCommandByteSize,
) -> Result<RecognizedMachOLinkeditDataCommand, BinaryFormatProbeError> {
    Ok(RecognizedMachOLinkeditDataCommand {
        command,
        byte_size,
        dataoff: read_file_offset(input, command_offset, MACH_O_LINKEDIT_DATAOFF_OFFSET)?,
        datasize: read_linkedit_u32_as(
            input,
            command_offset,
            MACH_O_LINKEDIT_DATASIZE_OFFSET,
            MachOLinkeditByteSize::from_public_linkedit_value,
        )?,
    })
}

fn read_data_range(
    input: &BinaryInput,
    command_offset: usize,
    offset_field: usize,
    size_field: usize,
) -> Result<MachOLinkeditDataRange, BinaryFormatProbeError> {
    Ok(MachOLinkeditDataRange::new(
        read_file_offset(input, command_offset, offset_field)?,
        read_linkedit_u32_as(
            input,
            command_offset,
            size_field,
            MachOLinkeditByteSize::from_public_linkedit_value,
        )?,
    ))
}

fn read_file_offset(
    input: &BinaryInput,
    command_offset: usize,
    field_offset: usize,
) -> Result<MachOLinkeditFileOffset, BinaryFormatProbeError> {
    read_linkedit_u32_as(
        input,
        command_offset,
        field_offset,
        MachOLinkeditFileOffset::from_public_linkedit_value,
    )
}

fn read_entry_count(
    input: &BinaryInput,
    command_offset: usize,
    field_offset: usize,
) -> Result<MachOLinkeditEntryCount, BinaryFormatProbeError> {
    read_linkedit_u32_as(
        input,
        command_offset,
        field_offset,
        MachOLinkeditEntryCount::from_public_linkedit_value,
    )
}

fn read_symbol_index(
    input: &BinaryInput,
    command_offset: usize,
    field_offset: usize,
) -> Result<MachOSymbolIndex, BinaryFormatProbeError> {
    read_linkedit_u32_as(
        input,
        command_offset,
        field_offset,
        MachOSymbolIndex::from_public_linkedit_value,
    )
}

fn read_linkedit_u32_as<T>(
    input: &BinaryInput,
    command_offset: usize,
    field_offset: usize,
    convert: impl Fn(u32) -> T,
) -> Result<T, BinaryFormatProbeError> {
    let offset = command_offset
        .checked_add(field_offset)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    input
        .read_little_endian_u32_at(offset)
        .map(convert)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
}

fn validate_minimum_byte_size(
    byte_size: usize,
    minimum: usize,
) -> Result<(), BinaryFormatProbeError> {
    if byte_size < minimum {
        return Err(BinaryFormatProbeError::LoadCommandTooSmall);
    }

    Ok(())
}

const MACH_O_SYMTAB_COMMAND_WIDTH: usize = 24;
const MACH_O_SYMTAB_SYMOFF_OFFSET: usize = 8;
const MACH_O_SYMTAB_NSYMS_OFFSET: usize = 12;
const MACH_O_SYMTAB_STROFF_OFFSET: usize = 16;
const MACH_O_SYMTAB_STRSIZE_OFFSET: usize = 20;

const MACH_O_DYSYMTAB_COMMAND_WIDTH: usize = 80;
const MACH_O_DYSYMTAB_ILOCALSYM_OFFSET: usize = 8;
const MACH_O_DYSYMTAB_NLOCALSYM_OFFSET: usize = 12;
const MACH_O_DYSYMTAB_IEXTDEFSYM_OFFSET: usize = 16;
const MACH_O_DYSYMTAB_NEXTDEFSYM_OFFSET: usize = 20;
const MACH_O_DYSYMTAB_IUNDEFSYM_OFFSET: usize = 24;
const MACH_O_DYSYMTAB_NUNDEFSYM_OFFSET: usize = 28;
const MACH_O_DYSYMTAB_TOCOFF_OFFSET: usize = 32;
const MACH_O_DYSYMTAB_NTOC_OFFSET: usize = 36;
const MACH_O_DYSYMTAB_MODTABOFF_OFFSET: usize = 40;
const MACH_O_DYSYMTAB_NMODTAB_OFFSET: usize = 44;
const MACH_O_DYSYMTAB_EXTREFSYMOFF_OFFSET: usize = 48;
const MACH_O_DYSYMTAB_NEXTREFSYMS_OFFSET: usize = 52;
const MACH_O_DYSYMTAB_INDIRECTSYMOFF_OFFSET: usize = 56;
const MACH_O_DYSYMTAB_NINDIRECTSYMS_OFFSET: usize = 60;
const MACH_O_DYSYMTAB_EXTRELOFF_OFFSET: usize = 64;
const MACH_O_DYSYMTAB_NEXTREL_OFFSET: usize = 68;
const MACH_O_DYSYMTAB_LOCRELOFF_OFFSET: usize = 72;
const MACH_O_DYSYMTAB_NLOCREL_OFFSET: usize = 76;

const MACH_O_DYLD_INFO_COMMAND_WIDTH: usize = 48;
const MACH_O_DYLD_INFO_REBASE_OFF_OFFSET: usize = 8;
const MACH_O_DYLD_INFO_REBASE_SIZE_OFFSET: usize = 12;
const MACH_O_DYLD_INFO_BIND_OFF_OFFSET: usize = 16;
const MACH_O_DYLD_INFO_BIND_SIZE_OFFSET: usize = 20;
const MACH_O_DYLD_INFO_WEAK_BIND_OFF_OFFSET: usize = 24;
const MACH_O_DYLD_INFO_WEAK_BIND_SIZE_OFFSET: usize = 28;
const MACH_O_DYLD_INFO_LAZY_BIND_OFF_OFFSET: usize = 32;
const MACH_O_DYLD_INFO_LAZY_BIND_SIZE_OFFSET: usize = 36;
const MACH_O_DYLD_INFO_EXPORT_OFF_OFFSET: usize = 40;
const MACH_O_DYLD_INFO_EXPORT_SIZE_OFFSET: usize = 44;

const MACH_O_LINKEDIT_DATA_COMMAND_WIDTH: usize = 16;
const MACH_O_LINKEDIT_DATAOFF_OFFSET: usize = 8;
const MACH_O_LINKEDIT_DATASIZE_OFFSET: usize = 12;
