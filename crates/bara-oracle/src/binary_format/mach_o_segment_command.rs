use super::{input::BinaryInput, probe::BinaryFormatProbeError};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOSegmentCommandHeaderMetadata {
    name: MachOSegmentName,
    vmaddr: MachOSegmentVmAddr,
    fileoff: MachOSegmentFileOffset,
    filesize: MachOSegmentFileSize,
}

impl MachOSegmentCommandHeaderMetadata {
    pub fn new(
        name: MachOSegmentName,
        vmaddr: MachOSegmentVmAddr,
        fileoff: MachOSegmentFileOffset,
        filesize: MachOSegmentFileSize,
    ) -> Self {
        Self {
            name,
            vmaddr,
            fileoff,
            filesize,
        }
    }

    pub const fn name(&self) -> &MachOSegmentName {
        &self.name
    }

    pub const fn vmaddr(&self) -> MachOSegmentVmAddr {
        self.vmaddr
    }

    pub const fn fileoff(&self) -> MachOSegmentFileOffset {
        self.fileoff
    }

    pub const fn filesize(&self) -> MachOSegmentFileSize {
        self.filesize
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSegmentName {
    value: String,
}

impl MachOSegmentName {
    pub(crate) fn from_public_fixed_field(bytes: &[u8]) -> Result<Self, BinaryFormatProbeError> {
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        let value = std::str::from_utf8(&bytes[..end])
            .map_err(|_| BinaryFormatProbeError::InvalidMachOSegmentName)?;

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
pub struct MachOSegmentVmAddr {
    value: u64,
}

impl MachOSegmentVmAddr {
    pub(crate) const fn from_public_segment_value(value: u64) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSegmentFileOffset {
    value: u64,
}

impl MachOSegmentFileOffset {
    pub(crate) const fn from_public_segment_value(value: u64) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOSegmentFileSize {
    value: u64,
}

impl MachOSegmentFileSize {
    pub(crate) const fn from_public_segment_value(value: u64) -> Self {
        Self { value }
    }
}

pub(crate) fn validate_segment_64_command_byte_size(
    byte_size: usize,
) -> Result<(), BinaryFormatProbeError> {
    if byte_size < MACH_O_SEGMENT_64_COMMAND_HEADER_WIDTH {
        return Err(BinaryFormatProbeError::LoadCommandTooSmall);
    }

    Ok(())
}

pub(crate) fn parse_segment_64_header_metadata(
    input: &BinaryInput,
    command_offset: usize,
) -> Result<MachOSegmentCommandHeaderMetadata, BinaryFormatProbeError> {
    let name_offset = command_offset
        .checked_add(MACH_O_SEGMENT_64_NAME_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let vmaddr_offset = command_offset
        .checked_add(MACH_O_SEGMENT_64_VMADDR_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let fileoff_offset = command_offset
        .checked_add(MACH_O_SEGMENT_64_FILEOFF_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let filesize_offset = command_offset
        .checked_add(MACH_O_SEGMENT_64_FILESIZE_OFFSET)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;

    let name = input
        .read_bytes_at(name_offset, MACH_O_SEGMENT_64_NAME_WIDTH)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)
        .and_then(MachOSegmentName::from_public_fixed_field)?;
    let vmaddr = input
        .read_little_endian_u64_at(vmaddr_offset)
        .map(MachOSegmentVmAddr::from_public_segment_value)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let fileoff = input
        .read_little_endian_u64_at(fileoff_offset)
        .map(MachOSegmentFileOffset::from_public_segment_value)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;
    let filesize = input
        .read_little_endian_u64_at(filesize_offset)
        .map(MachOSegmentFileSize::from_public_segment_value)
        .ok_or(BinaryFormatProbeError::LoadCommandsOutOfBounds)?;

    Ok(MachOSegmentCommandHeaderMetadata::new(
        name, vmaddr, fileoff, filesize,
    ))
}

const MACH_O_SEGMENT_64_COMMAND_HEADER_WIDTH: usize = 72;
const MACH_O_SEGMENT_64_NAME_OFFSET: usize = 8;
const MACH_O_SEGMENT_64_NAME_WIDTH: usize = 16;
const MACH_O_SEGMENT_64_VMADDR_OFFSET: usize = 24;
const MACH_O_SEGMENT_64_FILEOFF_OFFSET: usize = 40;
const MACH_O_SEGMENT_64_FILESIZE_OFFSET: usize = 48;
