mod mach_o;
mod mach_o_dylib_command;
mod mach_o_entry_point_command;
mod mach_o_executable_image_conversion;
mod mach_o_load_command;
mod mach_o_section;
mod mach_o_segment_command;
mod probe;

pub use mach_o::{MachOFileType, MachOLoadCommands, MachOMetadata};
pub use mach_o_dylib_command::{
    MachODylibImportCommandKind, MachODylibPath, MachODylibTimestamp, MachODylibVersion,
    RecognizedMachODylibImportCommand,
};
pub use mach_o_entry_point_command::{
    MachOEntryPointCommandMetadata, MachOEntryPointFileOffset, MachOEntryPointStackSize,
};
pub use mach_o_executable_image_conversion::{
    MachOExecutableImageConversion, MachOExecutableImageConversionBlocker,
    MachOExecutableImageConversionStatus,
};
pub use mach_o_load_command::{
    MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary, MachOLoadCommandType,
    RecognizedMachOEntryPointCommand, RecognizedMachOSegmentCommand, UnsupportedMachOLoadCommand,
};
pub use mach_o_section::{
    MachOSectionAddress, MachOSectionAlignment, MachOSectionByteSize, MachOSectionFileOffset,
    MachOSectionFlags, MachOSectionMetadata, MachOSectionName, MachOSectionRelocationCount,
    MachOSectionRelocationFileOffset,
};
pub use mach_o_segment_command::{
    MachOSegmentCommandHeaderMetadata, MachOSegmentFileOffset, MachOSegmentFileSize,
    MachOSegmentName, MachOSegmentVmAddr,
};
pub use probe::{
    probe_public_binary_format, BinaryFormat, BinaryFormatProbeError, BinaryFormatProbeMetadata,
    BinaryFormatProbeReport, BinaryFormatProbeStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryInput {
    bytes: Box<[u8]>,
}

impl BinaryInput {
    pub fn from_file_bytes(bytes: BinaryFileBytes) -> Self {
        Self { bytes: bytes.bytes }
    }

    pub fn from_hex(input: &str) -> Result<Self, BinaryInputError> {
        let bytes = decode_hex_bytes(input)?;

        Ok(Self {
            bytes: bytes.into_boxed_slice(),
        })
    }

    pub(crate) fn has_magic_width(&self) -> bool {
        self.bytes.len() >= BinaryMagic::WIDTH
    }

    pub(crate) fn starts_with_magic(&self, magic: BinaryMagic) -> bool {
        self.bytes.starts_with(magic.bytes())
    }

    pub(crate) fn has_len_at_least(&self, len: usize) -> bool {
        self.bytes.len() >= len
    }

    pub(crate) fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn read_little_endian_u32_at(&self, offset: usize) -> Option<u32> {
        let end = offset.checked_add(4)?;
        let bytes = self.bytes.get(offset..end)?;
        Some(u32::from_le_bytes(
            bytes.try_into().expect("slice len is 4"),
        ))
    }

    pub(crate) fn read_little_endian_u64_at(&self, offset: usize) -> Option<u64> {
        let end = offset.checked_add(8)?;
        let bytes = self.bytes.get(offset..end)?;
        Some(u64::from_le_bytes(
            bytes.try_into().expect("slice len is 8"),
        ))
    }

    pub(crate) fn read_bytes_at(&self, offset: usize, len: usize) -> Option<&[u8]> {
        let end = offset.checked_add(len)?;
        self.bytes.get(offset..end)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryFileBytes {
    bytes: Box<[u8]>,
}

impl BinaryFileBytes {
    pub fn from_untrusted_file_contents<T>(bytes: T) -> Self
    where
        T: Into<Box<[u8]>>,
    {
        Self {
            bytes: bytes.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryMagic {
    MachO64LittleEndian,
}

impl BinaryMagic {
    const WIDTH: usize = 4;

    const fn bytes(self) -> &'static [u8; Self::WIDTH] {
        match self {
            Self::MachO64LittleEndian => &[0xcf, 0xfa, 0xed, 0xfe],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryInputError {
    OddHexLength,
    InvalidHexDigit,
}

fn decode_hex_bytes(input: &str) -> Result<Vec<u8>, BinaryInputError> {
    if !input.len().is_multiple_of(2) {
        return Err(BinaryInputError::OddHexLength);
    }

    input
        .as_bytes()
        .chunks_exact(2)
        .map(decode_hex_byte)
        .collect()
}

fn decode_hex_byte(chunk: &[u8]) -> Result<u8, BinaryInputError> {
    let high = decode_hex_nibble(chunk[0])?;
    let low = decode_hex_nibble(chunk[1])?;
    Ok((high << 4) | low)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, BinaryInputError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(BinaryInputError::InvalidHexDigit),
    }
}
