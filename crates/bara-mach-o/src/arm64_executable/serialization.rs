use super::{MachOArm64ConstData, MachOArm64ExecutablePayload, MachOArm64ExecutableWriterPlan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64FileOffset {
    value: u64,
}

impl MachOArm64FileOffset {
    const fn from_writer_offset(value: u64) -> Self {
        Self { value }
    }

    pub const fn value(self) -> u64 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64ByteSize {
    value: u64,
}

impl MachOArm64ByteSize {
    const fn from_writer_size(value: u64) -> Self {
        Self { value }
    }

    pub const fn value(self) -> u64 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64FileRange {
    offset: MachOArm64FileOffset,
    size: MachOArm64ByteSize,
}

impl MachOArm64FileRange {
    const fn new(offset: MachOArm64FileOffset, size: MachOArm64ByteSize) -> Self {
        Self { offset, size }
    }

    pub const fn offset(self) -> MachOArm64FileOffset {
        self.offset
    }

    pub const fn size(self) -> MachOArm64ByteSize {
        self.size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64SerializedLayout {
    header: MachOArm64FileRange,
    load_commands: MachOArm64FileRange,
    text_section: MachOArm64FileRange,
    const_section: Option<MachOArm64FileRange>,
    total_size: MachOArm64ByteSize,
}

impl MachOArm64SerializedLayout {
    fn from_payload(
        payload: &MachOArm64ExecutablePayload,
    ) -> Result<Self, MachOArm64ExecutableWriterSerializationError> {
        let load_commands_size = MACH_O_SEGMENT_64_COMMAND_BASE_SIZE
            .checked_add(
                payload
                    .section_count()
                    .checked_mul(MACH_O_SECTION_64_SIZE)
                    .ok_or(MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?,
            )
            .and_then(|value| value.checked_add(MACH_O_MAIN_COMMAND_SIZE))
            .ok_or(MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?;
        let text_offset = MACH_O_64_HEADER_SIZE
            .checked_add(load_commands_size)
            .ok_or(MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?;
        let text_size = payload.main().byte_len();
        let const_offset = text_offset
            .checked_add(text_size)
            .ok_or(MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?;
        let const_size = payload
            .const_data()
            .map(MachOArm64ConstData::byte_len)
            .unwrap_or(0);
        let total_size = const_offset
            .checked_add(const_size)
            .ok_or(MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?;

        Ok(Self {
            header: file_range_from_usize(0, MACH_O_64_HEADER_SIZE)?,
            load_commands: file_range_from_usize(MACH_O_64_HEADER_SIZE, load_commands_size)?,
            text_section: file_range_from_usize(text_offset, text_size)?,
            const_section: payload
                .const_data()
                .map(|_| file_range_from_usize(const_offset, const_size))
                .transpose()?,
            total_size: byte_size_from_usize(total_size)?,
        })
    }

    pub const fn header(&self) -> MachOArm64FileRange {
        self.header
    }

    pub const fn load_commands(&self) -> MachOArm64FileRange {
        self.load_commands
    }

    pub const fn text_section(&self) -> MachOArm64FileRange {
        self.text_section
    }

    pub const fn const_section(&self) -> Option<MachOArm64FileRange> {
        self.const_section
    }

    pub const fn total_size(&self) -> MachOArm64ByteSize {
        self.total_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64SerializedExecutable {
    layout: MachOArm64SerializedLayout,
    bytes: Box<[u8]>,
}

impl MachOArm64SerializedExecutable {
    fn new(layout: MachOArm64SerializedLayout, bytes: Vec<u8>) -> Self {
        Self {
            layout,
            bytes: bytes.into_boxed_slice(),
        }
    }

    pub const fn layout(&self) -> &MachOArm64SerializedLayout {
        &self.layout
    }

    pub fn bytes_at(
        &self,
        range: MachOArm64FileRange,
    ) -> Option<MachOArm64SerializedByteSlice<'_>> {
        let offset = usize::try_from(range.offset().value()).ok()?;
        let size = usize::try_from(range.size().value()).ok()?;
        let end = offset.checked_add(size)?;
        self.bytes
            .get(offset..end)
            .map(MachOArm64SerializedByteSlice::new)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64SerializedByteSlice<'a> {
    bytes: &'a [u8],
}

impl<'a> MachOArm64SerializedByteSlice<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}

impl PartialEq<&[u8]> for MachOArm64SerializedByteSlice<'_> {
    fn eq(&self, other: &&[u8]) -> bool {
        self.bytes == *other
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOArm64ExecutableWriterSerializationError {
    PayloadSizeOverflow,
    LayoutSizeOverflow,
}

pub fn serialize_mach_o_arm64_executable(
    plan: &MachOArm64ExecutableWriterPlan,
) -> Result<MachOArm64SerializedExecutable, MachOArm64ExecutableWriterSerializationError> {
    let layout = MachOArm64SerializedLayout::from_payload(plan.payload())?;
    let mut bytes = Vec::with_capacity(usize_from_byte_size(layout.total_size())?);

    push_mach_o_64_header(&mut bytes, &layout)?;
    push_segment_64_command(&mut bytes, plan.payload(), &layout)?;
    push_main_load_command(&mut bytes, &layout);
    bytes.extend_from_slice(plan.payload().main().as_bytes());
    if let Some(const_data) = plan.payload().const_data() {
        bytes.extend_from_slice(const_data.as_bytes());
    }

    Ok(MachOArm64SerializedExecutable::new(layout, bytes))
}

fn push_mach_o_64_header(
    bytes: &mut Vec<u8>,
    layout: &MachOArm64SerializedLayout,
) -> Result<(), MachOArm64ExecutableWriterSerializationError> {
    push_u32(bytes, MACH_O_64_MAGIC);
    push_u32(bytes, MACH_O_CPU_TYPE_ARM64);
    push_u32(bytes, MACH_O_CPU_SUBTYPE_ARM64_ALL);
    push_u32(bytes, MACH_O_FILETYPE_EXECUTE);
    push_u32(bytes, MACH_O_MINIMAL_LOAD_COMMAND_COUNT);
    push_u32(bytes, u32_from_byte_size(layout.load_commands().size())?);
    push_u32(bytes, MACH_O_HEADER_FLAGS);
    push_u32(bytes, MACH_O_HEADER_RESERVED);

    Ok(())
}

fn push_segment_64_command(
    bytes: &mut Vec<u8>,
    payload: &MachOArm64ExecutablePayload,
    layout: &MachOArm64SerializedLayout,
) -> Result<(), MachOArm64ExecutableWriterSerializationError> {
    let section_count = u32::try_from(payload.section_count())
        .map_err(|_| MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?;
    let segment_command_size = layout
        .load_commands()
        .size()
        .value()
        .checked_sub(MACH_O_MAIN_COMMAND_SIZE as u64)
        .ok_or(MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?;

    push_u32(bytes, MACH_O_LC_SEGMENT_64);
    push_u32(bytes, u32_from_u64(segment_command_size)?);
    push_fixed_name(bytes, b"__TEXT");
    push_u64(bytes, MACH_O_TEXT_VMADDR);
    push_u64(bytes, layout.total_size().value());
    push_u64(bytes, 0);
    push_u64(bytes, layout.total_size().value());
    push_u32(bytes, MACH_O_PROT_READ | MACH_O_PROT_EXECUTE);
    push_u32(bytes, MACH_O_PROT_READ | MACH_O_PROT_EXECUTE);
    push_u32(bytes, section_count);
    push_u32(bytes, 0);
    push_text_section(bytes, layout)?;
    if let Some(const_section) = layout.const_section() {
        push_const_section(bytes, const_section)?;
    }

    Ok(())
}

fn push_text_section(
    bytes: &mut Vec<u8>,
    layout: &MachOArm64SerializedLayout,
) -> Result<(), MachOArm64ExecutableWriterSerializationError> {
    push_section_64(
        bytes,
        b"__text",
        layout.text_section(),
        MACH_O_SECTION_ALIGN_4_BYTES,
        MACH_O_SECTION_ATTR_PURE_INSTRUCTIONS | MACH_O_SECTION_ATTR_SOME_INSTRUCTIONS,
    )
}

fn push_const_section(
    bytes: &mut Vec<u8>,
    range: MachOArm64FileRange,
) -> Result<(), MachOArm64ExecutableWriterSerializationError> {
    push_section_64(
        bytes,
        b"__const",
        range,
        MACH_O_SECTION_ALIGN_1_BYTE,
        MACH_O_SECTION_TYPE_REGULAR,
    )
}

fn push_section_64(
    bytes: &mut Vec<u8>,
    section_name: &[u8],
    range: MachOArm64FileRange,
    align: u32,
    flags: u32,
) -> Result<(), MachOArm64ExecutableWriterSerializationError> {
    let section_addr = MACH_O_TEXT_VMADDR
        .checked_add(range.offset().value())
        .ok_or(MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?;

    push_fixed_name(bytes, section_name);
    push_fixed_name(bytes, b"__TEXT");
    push_u64(bytes, section_addr);
    push_u64(bytes, range.size().value());
    push_u32(bytes, u32_from_u64(range.offset().value())?);
    push_u32(bytes, align);
    push_u32(bytes, 0);
    push_u32(bytes, 0);
    push_u32(bytes, flags);
    push_u32(bytes, 0);
    push_u32(bytes, 0);
    push_u32(bytes, 0);

    Ok(())
}

fn push_main_load_command(bytes: &mut Vec<u8>, layout: &MachOArm64SerializedLayout) {
    push_u32(bytes, MACH_O_LC_MAIN);
    push_u32(bytes, MACH_O_MAIN_COMMAND_SIZE as u32);
    push_u64(bytes, layout.text_section().offset().value());
    push_u64(bytes, 0);
}

fn push_fixed_name(bytes: &mut Vec<u8>, name: &[u8]) {
    let mut field = [0_u8; MACH_O_FIXED_NAME_SIZE];
    let len = name.len().min(MACH_O_FIXED_NAME_SIZE);
    field[..len].copy_from_slice(&name[..len]);
    bytes.extend_from_slice(&field);
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn file_range_from_usize(
    offset: usize,
    size: usize,
) -> Result<MachOArm64FileRange, MachOArm64ExecutableWriterSerializationError> {
    Ok(MachOArm64FileRange::new(
        file_offset_from_usize(offset)?,
        byte_size_from_usize(size)?,
    ))
}

fn file_offset_from_usize(
    value: usize,
) -> Result<MachOArm64FileOffset, MachOArm64ExecutableWriterSerializationError> {
    Ok(MachOArm64FileOffset::from_writer_offset(
        u64::try_from(value)
            .map_err(|_| MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)?,
    ))
}

fn byte_size_from_usize(
    value: usize,
) -> Result<MachOArm64ByteSize, MachOArm64ExecutableWriterSerializationError> {
    Ok(MachOArm64ByteSize::from_writer_size(
        u64::try_from(value)
            .map_err(|_| MachOArm64ExecutableWriterSerializationError::PayloadSizeOverflow)?,
    ))
}

fn usize_from_byte_size(
    value: MachOArm64ByteSize,
) -> Result<usize, MachOArm64ExecutableWriterSerializationError> {
    usize::try_from(value.value())
        .map_err(|_| MachOArm64ExecutableWriterSerializationError::PayloadSizeOverflow)
}

fn u32_from_byte_size(
    value: MachOArm64ByteSize,
) -> Result<u32, MachOArm64ExecutableWriterSerializationError> {
    u32_from_u64(value.value())
}

fn u32_from_u64(value: u64) -> Result<u32, MachOArm64ExecutableWriterSerializationError> {
    u32::try_from(value)
        .map_err(|_| MachOArm64ExecutableWriterSerializationError::LayoutSizeOverflow)
}

const MACH_O_64_HEADER_SIZE: usize = 32;
const MACH_O_SEGMENT_64_COMMAND_BASE_SIZE: usize = 72;
const MACH_O_SECTION_64_SIZE: usize = 80;
const MACH_O_MAIN_COMMAND_SIZE: usize = 24;
const MACH_O_FIXED_NAME_SIZE: usize = 16;

const MACH_O_64_MAGIC: u32 = 0xfeedfacf;
const MACH_O_CPU_TYPE_ARM64: u32 = 0x0100000c;
const MACH_O_CPU_SUBTYPE_ARM64_ALL: u32 = 0;
const MACH_O_FILETYPE_EXECUTE: u32 = 0x2;
const MACH_O_MINIMAL_LOAD_COMMAND_COUNT: u32 = 2;
const MACH_O_HEADER_FLAGS: u32 = 0;
const MACH_O_HEADER_RESERVED: u32 = 0;

const MACH_O_LC_SEGMENT_64: u32 = 0x19;
const MACH_O_LC_MAIN: u32 = 0x80000028;
const MACH_O_TEXT_VMADDR: u64 = 0x1_0000_0000;

const MACH_O_PROT_READ: u32 = 0x1;
const MACH_O_PROT_EXECUTE: u32 = 0x4;
const MACH_O_SECTION_ALIGN_1_BYTE: u32 = 0;
const MACH_O_SECTION_ALIGN_4_BYTES: u32 = 2;
const MACH_O_SECTION_TYPE_REGULAR: u32 = 0;
const MACH_O_SECTION_ATTR_SOME_INSTRUCTIONS: u32 = 0x00000400;
const MACH_O_SECTION_ATTR_PURE_INSTRUCTIONS: u32 = 0x80000000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        plan_mach_o_arm64_executable, MachOArm64ConstData, MachOArm64ExecutableWriterRequest,
        MachOArm64MainCode,
    };

    #[test]
    fn serializes_main_only_offsets_sizes_and_payload_bytes() {
        let main_bytes = [0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6];
        let main =
            MachOArm64MainCode::from_emitted_code_bytes(main_bytes).expect("main is non-empty");
        let plan = plan_mach_o_arm64_executable(MachOArm64ExecutableWriterRequest::main_only(main));

        let serialized =
            serialize_mach_o_arm64_executable(&plan).expect("writer serializes main-only plan");
        let layout = serialized.layout();

        assert_eq!(layout.header().offset().value(), 0);
        assert_eq!(layout.header().size().value(), 32);
        assert_eq!(layout.load_commands().offset().value(), 32);
        assert_eq!(layout.load_commands().size().value(), 176);
        assert_eq!(layout.text_section().offset().value(), 208);
        assert_eq!(layout.text_section().size().value(), 8);
        assert_eq!(layout.const_section(), None);
        assert_eq!(layout.total_size().value(), 216);
        assert_eq!(
            serialized
                .bytes_at(layout.text_section())
                .expect("text range is in serialized bytes"),
            &main_bytes[..]
        );
    }

    #[test]
    fn serializes_const_payload_offsets_sizes_and_payload_bytes() {
        let main_bytes = [0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6];
        let const_bytes = *b"hello world\n";
        let main =
            MachOArm64MainCode::from_emitted_code_bytes(main_bytes).expect("main is non-empty");
        let const_data = MachOArm64ConstData::from_read_only_section_bytes(const_bytes)
            .expect("const data is non-empty");
        let plan = plan_mach_o_arm64_executable(
            MachOArm64ExecutableWriterRequest::main_with_const_data(main, const_data),
        );

        let serialized =
            serialize_mach_o_arm64_executable(&plan).expect("writer serializes const plan");
        let layout = serialized.layout();
        let const_section = layout.const_section().expect("const payload has a range");

        assert_eq!(layout.header().offset().value(), 0);
        assert_eq!(layout.header().size().value(), 32);
        assert_eq!(layout.load_commands().offset().value(), 32);
        assert_eq!(layout.load_commands().size().value(), 256);
        assert_eq!(layout.text_section().offset().value(), 288);
        assert_eq!(layout.text_section().size().value(), 8);
        assert_eq!(const_section.offset().value(), 296);
        assert_eq!(const_section.size().value(), 12);
        assert_eq!(layout.total_size().value(), 308);
        assert_eq!(
            serialized
                .bytes_at(layout.text_section())
                .expect("text range is in serialized bytes"),
            &main_bytes[..]
        );
        assert_eq!(
            serialized
                .bytes_at(const_section)
                .expect("const range is in serialized bytes"),
            &const_bytes[..]
        );
    }
}
