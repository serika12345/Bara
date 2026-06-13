use super::{BinaryFormatProbeError, BinaryInput, MachOSegmentName};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOSectionMetadata {
    name: MachOSectionName,
    segment_name: MachOSegmentName,
    addr: MachOSectionAddress,
    size: MachOSectionByteSize,
    offset: MachOSectionFileOffset,
    align: MachOSectionAlignment,
    reloff: MachOSectionRelocationFileOffset,
    nreloc: MachOSectionRelocationCount,
    flags: MachOSectionFlags,
    reserved1: MachOSectionReserved1,
    reserved2: MachOSectionReserved2,
    reserved3: MachOSectionReserved3,
}

impl MachOSectionMetadata {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        name: MachOSectionName,
        segment_name: MachOSegmentName,
        addr: MachOSectionAddress,
        size: MachOSectionByteSize,
        offset: MachOSectionFileOffset,
        align: MachOSectionAlignment,
        reloff: MachOSectionRelocationFileOffset,
        nreloc: MachOSectionRelocationCount,
        flags: MachOSectionFlags,
        reserved1: MachOSectionReserved1,
        reserved2: MachOSectionReserved2,
        reserved3: MachOSectionReserved3,
    ) -> Self {
        Self {
            name,
            segment_name,
            addr,
            size,
            offset,
            align,
            reloff,
            nreloc,
            flags,
            reserved1,
            reserved2,
            reserved3,
        }
    }

    pub const fn name(&self) -> &MachOSectionName {
        &self.name
    }

    pub const fn segment_name(&self) -> &MachOSegmentName {
        &self.segment_name
    }

    pub const fn addr(&self) -> MachOSectionAddress {
        self.addr
    }

    pub const fn size(&self) -> MachOSectionByteSize {
        self.size
    }

    pub const fn offset(&self) -> MachOSectionFileOffset {
        self.offset
    }

    pub const fn align(&self) -> MachOSectionAlignment {
        self.align
    }

    pub const fn reloff(&self) -> MachOSectionRelocationFileOffset {
        self.reloff
    }

    pub const fn nreloc(&self) -> MachOSectionRelocationCount {
        self.nreloc
    }

    pub const fn flags(&self) -> MachOSectionFlags {
        self.flags
    }

    pub const fn reserved1(&self) -> MachOSectionReserved1 {
        self.reserved1
    }

    pub const fn reserved2(&self) -> MachOSectionReserved2 {
        self.reserved2
    }

    pub const fn reserved3(&self) -> MachOSectionReserved3 {
        self.reserved3
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionName {
    value: String,
}

impl MachOSectionName {
    fn from_public_fixed_field(bytes: &[u8]) -> Result<Self, BinaryFormatProbeError> {
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        let value = std::str::from_utf8(&bytes[..end])
            .map_err(|_| BinaryFormatProbeError::InvalidMachOSectionName)?;

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
pub struct MachOSectionAddress {
    value: u64,
}

impl MachOSectionAddress {
    const fn from_public_section_value(value: u64) -> Self {
        Self { value }
    }

    pub(crate) const fn as_u64(self) -> u64 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionByteSize {
    value: u64,
}

impl MachOSectionByteSize {
    const fn from_public_section_value(value: u64) -> Self {
        Self { value }
    }

    pub(crate) const fn as_u64(self) -> u64 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionFileOffset {
    value: u32,
}

impl MachOSectionFileOffset {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionAlignment {
    value: u32,
}

impl MachOSectionAlignment {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionRelocationFileOffset {
    value: u32,
}

impl MachOSectionRelocationFileOffset {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionRelocationCount {
    value: u32,
}

impl MachOSectionRelocationCount {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionFlags {
    value: u32,
}

impl MachOSectionFlags {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionReserved1 {
    value: u32,
}

impl MachOSectionReserved1 {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }

    pub(crate) const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionReserved2 {
    value: u32,
}

impl MachOSectionReserved2 {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }

    pub(crate) const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSectionReserved3 {
    value: u32,
}

impl MachOSectionReserved3 {
    const fn from_public_section_value(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MachOSectionCount {
    value: u32,
}

impl MachOSectionCount {
    const fn from_public_segment_value(value: u32) -> Self {
        Self { value }
    }

    const fn as_u32(self) -> u32 {
        self.value
    }
}

pub(crate) fn parse_segment_64_sections_metadata(
    input: &BinaryInput,
    command_offset: usize,
    command_byte_size: usize,
) -> Result<Vec<MachOSectionMetadata>, BinaryFormatProbeError> {
    let section_count = parse_segment_64_section_count(input, command_offset)?;
    validate_segment_64_section_table_byte_size(command_byte_size, section_count)?;

    let mut sections = Vec::new();
    let mut section_offset = command_offset
        .checked_add(MACH_O_SEGMENT_64_COMMAND_HEADER_WIDTH)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    for _ in 0..section_count.as_u32() {
        sections.push(parse_section_64_metadata(input, section_offset)?);
        section_offset = section_offset
            .checked_add(MACH_O_SECTION_64_WIDTH)
            .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    }

    Ok(sections)
}

fn parse_segment_64_section_count(
    input: &BinaryInput,
    command_offset: usize,
) -> Result<MachOSectionCount, BinaryFormatProbeError> {
    let section_count_offset = command_offset
        .checked_add(MACH_O_SEGMENT_64_NSECTS_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;

    input
        .read_little_endian_u32_at(section_count_offset)
        .map(MachOSectionCount::from_public_segment_value)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
}

fn validate_segment_64_section_table_byte_size(
    command_byte_size: usize,
    section_count: MachOSectionCount,
) -> Result<(), BinaryFormatProbeError> {
    let section_table_byte_size = (section_count.as_u32() as usize)
        .checked_mul(MACH_O_SECTION_64_WIDTH)
        .ok_or(BinaryFormatProbeError::LoadCommandTooSmall)?;
    let minimum_command_byte_size = MACH_O_SEGMENT_64_COMMAND_HEADER_WIDTH
        .checked_add(section_table_byte_size)
        .ok_or(BinaryFormatProbeError::LoadCommandTooSmall)?;

    if command_byte_size < minimum_command_byte_size {
        return Err(BinaryFormatProbeError::LoadCommandTooSmall);
    }

    Ok(())
}

fn parse_section_64_metadata(
    input: &BinaryInput,
    section_offset: usize,
) -> Result<MachOSectionMetadata, BinaryFormatProbeError> {
    let name = input
        .read_bytes_at(section_offset, MACH_O_SECTION_64_NAME_WIDTH)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
        .and_then(MachOSectionName::from_public_fixed_field)?;
    let segment_name_offset = section_offset
        .checked_add(MACH_O_SECTION_64_SEGMENT_NAME_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let segment_name = input
        .read_bytes_at(segment_name_offset, MACH_O_SECTION_64_NAME_WIDTH)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
        .and_then(MachOSegmentName::from_public_fixed_field)?;
    let addr = read_section_u64(
        input,
        section_offset,
        MACH_O_SECTION_64_ADDR_OFFSET,
        MachOSectionAddress::from_public_section_value,
    )?;
    let size = read_section_u64(
        input,
        section_offset,
        MACH_O_SECTION_64_SIZE_OFFSET,
        MachOSectionByteSize::from_public_section_value,
    )?;
    let offset = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_OFFSET_OFFSET,
        MachOSectionFileOffset::from_public_section_value,
    )?;
    let align = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_ALIGN_OFFSET,
        MachOSectionAlignment::from_public_section_value,
    )?;
    let reloff = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_RELOFF_OFFSET,
        MachOSectionRelocationFileOffset::from_public_section_value,
    )?;
    let nreloc = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_NRELOC_OFFSET,
        MachOSectionRelocationCount::from_public_section_value,
    )?;
    let flags = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_FLAGS_OFFSET,
        MachOSectionFlags::from_public_section_value,
    )?;
    let reserved1 = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_RESERVED1_OFFSET,
        MachOSectionReserved1::from_public_section_value,
    )?;
    let reserved2 = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_RESERVED2_OFFSET,
        MachOSectionReserved2::from_public_section_value,
    )?;
    let reserved3 = read_section_u32(
        input,
        section_offset,
        MACH_O_SECTION_64_RESERVED3_OFFSET,
        MachOSectionReserved3::from_public_section_value,
    )?;

    Ok(MachOSectionMetadata::new(
        name,
        segment_name,
        addr,
        size,
        offset,
        align,
        reloff,
        nreloc,
        flags,
        reserved1,
        reserved2,
        reserved3,
    ))
}

fn read_section_u64<T>(
    input: &BinaryInput,
    section_offset: usize,
    field_offset: usize,
    convert: impl Fn(u64) -> T,
) -> Result<T, BinaryFormatProbeError> {
    let offset = section_offset
        .checked_add(field_offset)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    input
        .read_little_endian_u64_at(offset)
        .map(convert)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
}

fn read_section_u32<T>(
    input: &BinaryInput,
    section_offset: usize,
    field_offset: usize,
    convert: impl Fn(u32) -> T,
) -> Result<T, BinaryFormatProbeError> {
    let offset = section_offset
        .checked_add(field_offset)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    input
        .read_little_endian_u32_at(offset)
        .map(convert)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
}

const MACH_O_SEGMENT_64_COMMAND_HEADER_WIDTH: usize = 72;
const MACH_O_SEGMENT_64_NSECTS_OFFSET: usize = 64;
const MACH_O_SECTION_64_WIDTH: usize = 80;
const MACH_O_SECTION_64_NAME_WIDTH: usize = 16;
const MACH_O_SECTION_64_SEGMENT_NAME_OFFSET: usize = 16;
const MACH_O_SECTION_64_ADDR_OFFSET: usize = 32;
const MACH_O_SECTION_64_SIZE_OFFSET: usize = 40;
const MACH_O_SECTION_64_OFFSET_OFFSET: usize = 48;
const MACH_O_SECTION_64_ALIGN_OFFSET: usize = 52;
const MACH_O_SECTION_64_RELOFF_OFFSET: usize = 56;
const MACH_O_SECTION_64_NRELOC_OFFSET: usize = 60;
const MACH_O_SECTION_64_FLAGS_OFFSET: usize = 64;
const MACH_O_SECTION_64_RESERVED1_OFFSET: usize = 68;
const MACH_O_SECTION_64_RESERVED2_OFFSET: usize = 72;
const MACH_O_SECTION_64_RESERVED3_OFFSET: usize = 76;
