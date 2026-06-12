use super::{
    BinaryInput, MachOLinkeditDataCommandKind, MachOMetadata, RecognizedMachOLinkeditDataCommand,
};

use serde::Serialize;

pub fn decode_mach_o_chained_fixups_for_target(
    input: &BinaryInput,
    metadata: &MachOMetadata,
    target_address: MachOChainedFixupTargetAddress,
) -> MachOChainedFixupsTargetReport {
    let Some(command) = chained_fixups_command(metadata) else {
        return MachOChainedFixupsTargetReport::blocked(
            target_address,
            None,
            None,
            None,
            MachOChainedFixupsBlocker::MissingChainedFixupsCommand,
        );
    };
    let payload = match MachOChainedFixupsPayloadRange::from_command(input, command) {
        Ok(payload) => payload,
        Err(blocker) => {
            return MachOChainedFixupsTargetReport::blocked(
                target_address,
                None,
                None,
                None,
                blocker,
            );
        }
    };
    let header = match MachOChainedFixupsHeaderReport::parse(input, payload) {
        Ok(header) => header,
        Err(blocker) => {
            return MachOChainedFixupsTargetReport::blocked(
                target_address,
                None,
                None,
                None,
                blocker,
            );
        }
    };

    if header.fixups_version != 0 {
        return MachOChainedFixupsTargetReport::blocked(
            target_address,
            Some(header),
            None,
            None,
            MachOChainedFixupsBlocker::UnsupportedFixupsVersion {
                version: header.fixups_version,
            },
        );
    }
    if header.symbols_format != 0 {
        return MachOChainedFixupsTargetReport::blocked(
            target_address,
            Some(header),
            None,
            None,
            MachOChainedFixupsBlocker::UnsupportedSymbolsFormat {
                symbols_format: header.symbols_format,
            },
        );
    }

    let starts = match MachOChainedStartsInImageReport::parse(input, payload, &header) {
        Ok(starts) => starts,
        Err(blocker) => {
            return MachOChainedFixupsTargetReport::blocked(
                target_address,
                Some(header),
                None,
                None,
                blocker,
            );
        }
    };
    let imports = match MachOChainedImportsReport::parse(input, payload, metadata, &header) {
        Ok(imports) => imports,
        Err(blocker) => {
            return MachOChainedFixupsTargetReport::blocked(
                target_address,
                Some(header),
                Some(starts),
                None,
                blocker,
            );
        }
    };
    let resolution =
        match resolve_target_pointer(input, metadata, target_address, &starts, &imports) {
            Ok(resolution) => resolution,
            Err(blocker) => {
                return MachOChainedFixupsTargetReport::blocked(
                    target_address,
                    Some(header),
                    Some(starts),
                    Some(imports),
                    blocker,
                );
            }
        };

    MachOChainedFixupsTargetReport {
        schema: "mach_o_chained_fixups_target_report_v0",
        status: resolution.status(),
        target_address,
        header: Some(header),
        starts: Some(starts),
        imports: Some(imports),
        target_resolution: Some(resolution),
        blocker: None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOChainedFixupTargetAddress {
    value: u64,
}

impl MachOChainedFixupTargetAddress {
    pub const fn from_mach_o_virtual_address(value: u64) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MachOChainedFixupsTargetReport {
    schema: &'static str,
    status: MachOChainedFixupsTargetStatus,
    target_address: MachOChainedFixupTargetAddress,
    header: Option<MachOChainedFixupsHeaderReport>,
    starts: Option<MachOChainedStartsInImageReport>,
    imports: Option<MachOChainedImportsReport>,
    target_resolution: Option<MachOChainedFixupTargetResolutionReport>,
    blocker: Option<MachOChainedFixupsBlocker>,
}

impl MachOChainedFixupsTargetReport {
    fn blocked(
        target_address: MachOChainedFixupTargetAddress,
        header: Option<MachOChainedFixupsHeaderReport>,
        starts: Option<MachOChainedStartsInImageReport>,
        imports: Option<MachOChainedImportsReport>,
        blocker: MachOChainedFixupsBlocker,
    ) -> Self {
        Self {
            schema: "mach_o_chained_fixups_target_report_v0",
            status: MachOChainedFixupsTargetStatus::Blocked,
            target_address,
            header,
            starts,
            imports,
            target_resolution: None,
            blocker: Some(blocker),
        }
    }

    pub const fn status(&self) -> MachOChainedFixupsTargetStatus {
        self.status
    }

    pub fn resolved_import_identity(&self) -> Option<MachOChainedImportIdentityReport> {
        self.target_resolution.as_ref().and_then(|resolution| {
            resolution
                .import
                .as_ref()
                .map(MachOChainedImportIdentityReport::from_import)
        })
    }

    pub fn resolved_rebase_target(&self) -> Option<MachOChainedRebaseTargetIdentityReport> {
        self.target_resolution
            .as_ref()
            .and_then(|resolution| resolution.rebase)
    }

    pub const fn blocker(&self) -> Option<MachOChainedFixupsBlocker> {
        self.blocker
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOChainedFixupsTargetStatus {
    Blocked,
    ResolvedImport,
    ResolvedRebase,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MachOChainedImportIdentityReport {
    import_index: u32,
    lib_ordinal: u32,
    dylib_path: Option<String>,
    weak_import: bool,
    symbol_name: String,
}

impl MachOChainedImportIdentityReport {
    fn from_import(import: &MachOChainedImportReport) -> Self {
        Self {
            import_index: import.import_index,
            lib_ordinal: import.lib_ordinal,
            dylib_path: import.dylib_path.clone(),
            weak_import: import.weak_import,
            symbol_name: import.symbol_name.clone(),
        }
    }

    pub fn dylib_path(&self) -> Option<&str> {
        self.dylib_path.as_deref()
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub const fn is_weak_import(&self) -> bool {
        self.weak_import
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct MachOChainedRebaseTargetIdentityReport {
    raw_target: u64,
    high8: u8,
    resolved_vm_address: u64,
}

impl MachOChainedRebaseTargetIdentityReport {
    const fn new(raw_target: u64, high8: u8, resolved_vm_address: u64) -> Self {
        Self {
            raw_target,
            high8,
            resolved_vm_address,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedFixupsHeaderReport {
    fixups_version: u32,
    starts_offset: u32,
    imports_offset: u32,
    symbols_offset: u32,
    imports_count: u32,
    imports_format: MachOChainedImportsFormatReport,
    symbols_format: u32,
}

impl MachOChainedFixupsHeaderReport {
    fn parse(
        input: &BinaryInput,
        payload: MachOChainedFixupsPayloadRange,
    ) -> Result<Self, MachOChainedFixupsBlocker> {
        Ok(Self {
            fixups_version: payload.read_u32(input, 0)?,
            starts_offset: payload.read_u32(input, 4)?,
            imports_offset: payload.read_u32(input, 8)?,
            symbols_offset: payload.read_u32(input, 12)?,
            imports_count: payload.read_u32(input, 16)?,
            imports_format: MachOChainedImportsFormatReport::from_public_value(
                payload.read_u32(input, 20)?,
            ),
            symbols_format: payload.read_u32(input, 24)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedImportsFormatReport {
    value: u32,
    kind: MachOChainedImportsFormatKind,
}

impl MachOChainedImportsFormatReport {
    const fn from_public_value(value: u32) -> Self {
        let kind = match value {
            DYLD_CHAINED_IMPORT => MachOChainedImportsFormatKind::DyldChainedImport,
            DYLD_CHAINED_IMPORT_ADDEND => MachOChainedImportsFormatKind::DyldChainedImportAddend,
            DYLD_CHAINED_IMPORT_ADDEND64 => {
                MachOChainedImportsFormatKind::DyldChainedImportAddend64
            }
            _ => MachOChainedImportsFormatKind::Unsupported,
        };

        Self { value, kind }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MachOChainedImportsFormatKind {
    DyldChainedImport,
    DyldChainedImportAddend,
    DyldChainedImportAddend64,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedStartsInImageReport {
    segment_count: u32,
    segments: Vec<MachOChainedSegmentStartsReport>,
}

impl MachOChainedStartsInImageReport {
    fn parse(
        input: &BinaryInput,
        payload: MachOChainedFixupsPayloadRange,
        header: &MachOChainedFixupsHeaderReport,
    ) -> Result<Self, MachOChainedFixupsBlocker> {
        let starts_offset = header.starts_offset;
        let segment_count = payload.read_u32(input, starts_offset)?;
        let offsets_start = starts_offset
            .checked_add(4)
            .ok_or(MachOChainedFixupsBlocker::StartsOutOfBounds)?;
        let offsets_bytes = segment_count
            .checked_mul(4)
            .ok_or(MachOChainedFixupsBlocker::StartsOutOfBounds)?;
        payload.validate_relative_range(offsets_start, offsets_bytes)?;

        let mut segments = Vec::new();
        for segment_index in 0..segment_count {
            let offset_field = checked_relative_add(
                offsets_start,
                checked_relative_mul(segment_index, 4)?,
                MachOChainedFixupsBlocker::StartsOutOfBounds,
            )?;
            let offset = payload.read_u32(input, offset_field)?;
            if offset == 0 {
                segments.push(MachOChainedSegmentStartsReport::empty(segment_index));
                continue;
            }
            segments.push(MachOChainedSegmentStartsReport::parse(
                input,
                payload,
                starts_offset,
                segment_index,
                offset,
            )?);
        }

        Ok(Self {
            segment_count,
            segments,
        })
    }

    fn segment(&self, index: usize) -> Option<&MachOChainedSegmentStartsReport> {
        self.segments
            .iter()
            .find(|segment| usize::try_from(segment.segment_index).ok() == Some(index))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedSegmentStartsReport {
    segment_index: u32,
    status: MachOChainedSegmentStartsStatus,
    size: Option<u32>,
    page_size: Option<u16>,
    pointer_format: Option<MachOChainedPointerFormatReport>,
    segment_offset: Option<u64>,
    max_valid_pointer: Option<u32>,
    page_count: Option<u16>,
    page_starts: Vec<u16>,
}

impl MachOChainedSegmentStartsReport {
    const fn empty(segment_index: u32) -> Self {
        Self {
            segment_index,
            status: MachOChainedSegmentStartsStatus::Absent,
            size: None,
            page_size: None,
            pointer_format: None,
            segment_offset: None,
            max_valid_pointer: None,
            page_count: None,
            page_starts: Vec::new(),
        }
    }

    fn parse(
        input: &BinaryInput,
        payload: MachOChainedFixupsPayloadRange,
        starts_offset: u32,
        segment_index: u32,
        segment_info_offset: u32,
    ) -> Result<Self, MachOChainedFixupsBlocker> {
        let base = starts_offset
            .checked_add(segment_info_offset)
            .ok_or(MachOChainedFixupsBlocker::SegmentStartsOutOfBounds)?;
        let size = payload.read_u32(input, base)?;
        if size < DYLD_CHAINED_STARTS_IN_SEGMENT_MIN_SIZE {
            return Err(MachOChainedFixupsBlocker::SegmentStartsOutOfBounds);
        }
        payload.validate_relative_range(base, size)?;

        let page_size = payload.read_u16(input, checked_segment_relative_add(base, 4)?)?;
        let pointer_format = MachOChainedPointerFormatReport::from_public_value(
            payload.read_u16(input, checked_segment_relative_add(base, 6)?)?,
        );
        let segment_offset = payload.read_u64(input, checked_segment_relative_add(base, 8)?)?;
        let max_valid_pointer = payload.read_u32(input, checked_segment_relative_add(base, 16)?)?;
        let page_count = payload.read_u16(input, checked_segment_relative_add(base, 20)?)?;
        let page_starts_offset = base
            .checked_add(DYLD_CHAINED_STARTS_IN_SEGMENT_MIN_SIZE)
            .ok_or(MachOChainedFixupsBlocker::SegmentStartsOutOfBounds)?;
        let page_starts_bytes = u32::from(page_count)
            .checked_mul(2)
            .ok_or(MachOChainedFixupsBlocker::SegmentStartsOutOfBounds)?;
        let required_size = DYLD_CHAINED_STARTS_IN_SEGMENT_MIN_SIZE
            .checked_add(page_starts_bytes)
            .ok_or(MachOChainedFixupsBlocker::SegmentStartsOutOfBounds)?;
        if required_size > size {
            return Err(MachOChainedFixupsBlocker::SegmentStartsOutOfBounds);
        }
        payload.validate_relative_range(page_starts_offset, page_starts_bytes)?;

        let mut page_starts = Vec::new();
        for page_index in 0..page_count {
            let page_start_offset = checked_relative_add(
                page_starts_offset,
                checked_relative_mul(u32::from(page_index), 2)?,
                MachOChainedFixupsBlocker::SegmentStartsOutOfBounds,
            )?;
            page_starts.push(payload.read_u16(input, page_start_offset)?);
        }

        Ok(Self {
            segment_index,
            status: MachOChainedSegmentStartsStatus::Present,
            size: Some(size),
            page_size: Some(page_size),
            pointer_format: Some(pointer_format),
            segment_offset: Some(segment_offset),
            max_valid_pointer: Some(max_valid_pointer),
            page_count: Some(page_count),
            page_starts,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MachOChainedSegmentStartsStatus {
    Absent,
    Present,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedPointerFormatReport {
    value: u16,
    kind: MachOChainedPointerFormatKind,
}

impl MachOChainedPointerFormatReport {
    const fn from_public_value(value: u16) -> Self {
        let kind = match value {
            DYLD_CHAINED_PTR_64 => MachOChainedPointerFormatKind::Ptr64,
            DYLD_CHAINED_PTR_64_OFFSET => MachOChainedPointerFormatKind::Ptr64Offset,
            _ => MachOChainedPointerFormatKind::Unsupported,
        };

        Self { value, kind }
    }

    const fn supports_64_pointer(self) -> bool {
        matches!(
            self.kind,
            MachOChainedPointerFormatKind::Ptr64 | MachOChainedPointerFormatKind::Ptr64Offset
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MachOChainedPointerFormatKind {
    Ptr64,
    Ptr64Offset,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedImportsReport {
    count: u32,
    format: MachOChainedImportsFormatReport,
    symbols_format: u32,
    imports: Vec<MachOChainedImportReport>,
}

impl MachOChainedImportsReport {
    fn parse(
        input: &BinaryInput,
        payload: MachOChainedFixupsPayloadRange,
        metadata: &MachOMetadata,
        header: &MachOChainedFixupsHeaderReport,
    ) -> Result<Self, MachOChainedFixupsBlocker> {
        if header.imports_format.value != DYLD_CHAINED_IMPORT {
            return Err(MachOChainedFixupsBlocker::UnsupportedImportsFormat {
                imports_format: header.imports_format.value,
            });
        }

        let imports_bytes = header
            .imports_count
            .checked_mul(DYLD_CHAINED_IMPORT_ENTRY_SIZE)
            .ok_or(MachOChainedFixupsBlocker::ImportsOutOfBounds)?;
        payload.validate_relative_range(header.imports_offset, imports_bytes)?;

        let mut imports = Vec::new();
        for import_index in 0..header.imports_count {
            let entry_offset = checked_relative_add(
                header.imports_offset,
                checked_relative_mul(import_index, DYLD_CHAINED_IMPORT_ENTRY_SIZE)?,
                MachOChainedFixupsBlocker::ImportsOutOfBounds,
            )?;
            let raw = payload.read_u32(input, entry_offset)?;
            let lib_ordinal = raw & 0xff;
            let weak_import = ((raw >> 8) & 1) != 0;
            let name_offset = raw >> 9;
            let symbol_name =
                payload.read_symbol_name(input, header.symbols_offset, name_offset)?;
            let dylib_path = dylib_path_for_public_ordinal(metadata, lib_ordinal);
            imports.push(MachOChainedImportReport {
                import_index,
                raw,
                lib_ordinal,
                dylib_path,
                weak_import,
                name_offset,
                symbol_name,
            });
        }

        Ok(Self {
            count: header.imports_count,
            format: header.imports_format,
            symbols_format: header.symbols_format,
            imports,
        })
    }

    fn import(&self, import_index: u32) -> Option<&MachOChainedImportReport> {
        self.imports
            .iter()
            .find(|import| import.import_index == import_index)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedImportReport {
    import_index: u32,
    raw: u32,
    lib_ordinal: u32,
    dylib_path: Option<String>,
    weak_import: bool,
    name_offset: u32,
    symbol_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct MachOChainedFixupTargetResolutionReport {
    segment_index: u32,
    segment_name: String,
    page_index: u64,
    chain_entry_address: MachOChainedFixupTargetAddress,
    chain_entry_file_offset: u64,
    raw_pointer: u64,
    pointer_format: MachOChainedPointerFormatReport,
    pointer_kind: MachOChainedPointerKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    import: Option<MachOChainedImportReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rebase: Option<MachOChainedRebaseTargetIdentityReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    addend: Option<u8>,
    next_stride_count: u16,
}

impl MachOChainedFixupTargetResolutionReport {
    const fn status(&self) -> MachOChainedFixupsTargetStatus {
        match self.pointer_kind {
            MachOChainedPointerKind::Bind => MachOChainedFixupsTargetStatus::ResolvedImport,
            MachOChainedPointerKind::Rebase => MachOChainedFixupsTargetStatus::ResolvedRebase,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MachOChainedPointerKind {
    Bind,
    Rebase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MachOChainedFixupsBlocker {
    ChainDidNotReachTarget,
    ChainEntryOutOfBounds,
    ChainWalkLimitExceeded,
    ImageBaseMissing,
    ImportOrdinalOutOfBounds { import_index: u32 },
    ImportsOutOfBounds,
    MissingChainedFixupsCommand,
    PageStartMultiUnsupported,
    PageStartNone,
    PayloadOutOfBounds,
    RebaseTargetAddressOverflow,
    SegmentIndexMissing { segment_index: u32 },
    SegmentStartsOutOfBounds,
    StartsOutOfBounds,
    SymbolNameOutOfBounds { name_offset: u32 },
    TargetChainEntryIsRebase,
    TargetPageMissing,
    TargetSegmentMissing,
    UnsupportedFixupsVersion { version: u32 },
    UnsupportedImportsFormat { imports_format: u32 },
    UnsupportedPointerFormat { pointer_format: u16 },
    UnsupportedSymbolsFormat { symbols_format: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MachOChainedFixupsPayloadRange {
    base: usize,
    byte_size: usize,
}

impl MachOChainedFixupsPayloadRange {
    fn from_command(
        input: &BinaryInput,
        command: &RecognizedMachOLinkeditDataCommand,
    ) -> Result<Self, MachOChainedFixupsBlocker> {
        let base = usize::try_from(command.dataoff().as_u32())
            .map_err(|_| MachOChainedFixupsBlocker::PayloadOutOfBounds)?;
        let byte_size = usize::try_from(command.datasize().as_u32())
            .map_err(|_| MachOChainedFixupsBlocker::PayloadOutOfBounds)?;
        let end = base
            .checked_add(byte_size)
            .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)?;
        if end > input.byte_len() {
            return Err(MachOChainedFixupsBlocker::PayloadOutOfBounds);
        }

        Ok(Self { base, byte_size })
    }

    fn read_u16(
        self,
        input: &BinaryInput,
        relative_offset: u32,
    ) -> Result<u16, MachOChainedFixupsBlocker> {
        let offset = self.absolute_offset(relative_offset, 2)?;
        input
            .read_bytes_at(offset, 2)
            .map(|bytes| u16::from_le_bytes(bytes.try_into().expect("slice len is 2")))
            .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)
    }

    fn read_u32(
        self,
        input: &BinaryInput,
        relative_offset: u32,
    ) -> Result<u32, MachOChainedFixupsBlocker> {
        let offset = self.absolute_offset(relative_offset, 4)?;
        input
            .read_little_endian_u32_at(offset)
            .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)
    }

    fn read_u64(
        self,
        input: &BinaryInput,
        relative_offset: u32,
    ) -> Result<u64, MachOChainedFixupsBlocker> {
        let offset = self.absolute_offset(relative_offset, 8)?;
        input
            .read_little_endian_u64_at(offset)
            .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)
    }

    fn read_symbol_name(
        self,
        input: &BinaryInput,
        symbols_offset: u32,
        name_offset: u32,
    ) -> Result<String, MachOChainedFixupsBlocker> {
        let string_start = symbols_offset
            .checked_add(name_offset)
            .ok_or(MachOChainedFixupsBlocker::SymbolNameOutOfBounds { name_offset })?;
        let absolute_start = self.absolute_offset(string_start, 1)?;
        let payload_end = self
            .base
            .checked_add(self.byte_size)
            .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)?;
        let bytes = input
            .read_bytes_at(absolute_start, payload_end - absolute_start)
            .ok_or(MachOChainedFixupsBlocker::SymbolNameOutOfBounds { name_offset })?;
        let string_end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(MachOChainedFixupsBlocker::SymbolNameOutOfBounds { name_offset })?;
        let value = std::str::from_utf8(&bytes[..string_end])
            .map_err(|_| MachOChainedFixupsBlocker::SymbolNameOutOfBounds { name_offset })?;

        Ok(value.to_owned())
    }

    fn validate_relative_range(
        self,
        relative_offset: u32,
        byte_size: u32,
    ) -> Result<(), MachOChainedFixupsBlocker> {
        self.absolute_offset(relative_offset, byte_size).map(|_| ())
    }

    fn absolute_offset(
        self,
        relative_offset: u32,
        byte_size: impl TryInto<usize>,
    ) -> Result<usize, MachOChainedFixupsBlocker> {
        let relative = usize::try_from(relative_offset)
            .map_err(|_| MachOChainedFixupsBlocker::PayloadOutOfBounds)?;
        let byte_size = byte_size
            .try_into()
            .map_err(|_| MachOChainedFixupsBlocker::PayloadOutOfBounds)?;
        let relative_end = relative
            .checked_add(byte_size)
            .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)?;
        if relative_end > self.byte_size {
            return Err(MachOChainedFixupsBlocker::PayloadOutOfBounds);
        }

        self.base
            .checked_add(relative)
            .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)
    }
}

fn resolve_target_pointer(
    input: &BinaryInput,
    metadata: &MachOMetadata,
    target_address: MachOChainedFixupTargetAddress,
    starts: &MachOChainedStartsInImageReport,
    imports: &MachOChainedImportsReport,
) -> Result<MachOChainedFixupTargetResolutionReport, MachOChainedFixupsBlocker> {
    let summary = metadata.load_commands().summary();
    let target = target_address.value;
    for (segment_index, segment) in summary.recognized_segments().iter().enumerate() {
        let header = segment.header();
        let segment_start = header.vmaddr().as_u64();
        let Some(segment_end) = segment_start.checked_add(header.filesize().as_u64()) else {
            continue;
        };
        if target < segment_start || target >= segment_end {
            continue;
        }

        let segment_starts = starts.segment(segment_index).ok_or(
            MachOChainedFixupsBlocker::SegmentIndexMissing {
                segment_index: u32::try_from(segment_index).unwrap_or(u32::MAX),
            },
        )?;
        if segment_starts.status == MachOChainedSegmentStartsStatus::Absent {
            return Err(MachOChainedFixupsBlocker::TargetSegmentMissing);
        }
        let page_size = segment_starts
            .page_size
            .ok_or(MachOChainedFixupsBlocker::TargetSegmentMissing)?;
        let pointer_format = segment_starts
            .pointer_format
            .ok_or(MachOChainedFixupsBlocker::TargetSegmentMissing)?;
        if !pointer_format.supports_64_pointer() {
            return Err(MachOChainedFixupsBlocker::UnsupportedPointerFormat {
                pointer_format: pointer_format.value,
            });
        }

        let target_segment_offset = target - segment_start;
        let page_index = target_segment_offset / u64::from(page_size);
        let page_starts_index = usize::try_from(page_index)
            .map_err(|_| MachOChainedFixupsBlocker::TargetPageMissing)?;
        let Some(page_start) = segment_starts.page_starts.get(page_starts_index).copied() else {
            return Err(MachOChainedFixupsBlocker::TargetPageMissing);
        };
        if page_start == DYLD_CHAINED_PTR_START_NONE {
            return Err(MachOChainedFixupsBlocker::PageStartNone);
        }
        if page_start & DYLD_CHAINED_PTR_START_MULTI != 0 {
            return Err(MachOChainedFixupsBlocker::PageStartMultiUnsupported);
        }

        let page_base_offset = page_index
            .checked_mul(u64::from(page_size))
            .ok_or(MachOChainedFixupsBlocker::ChainEntryOutOfBounds)?;
        let mut chain_offset = page_base_offset
            .checked_add(u64::from(page_start))
            .ok_or(MachOChainedFixupsBlocker::ChainEntryOutOfBounds)?;
        for _ in 0..MAX_CHAIN_WALK_ENTRIES {
            let entry_address = segment_start
                .checked_add(chain_offset)
                .ok_or(MachOChainedFixupsBlocker::ChainEntryOutOfBounds)?;
            let file_offset = header
                .fileoff()
                .as_u64()
                .checked_add(chain_offset)
                .ok_or(MachOChainedFixupsBlocker::ChainEntryOutOfBounds)?;
            let raw_pointer = input
                .read_little_endian_u64_at(
                    usize::try_from(file_offset)
                        .map_err(|_| MachOChainedFixupsBlocker::ChainEntryOutOfBounds)?,
                )
                .ok_or(MachOChainedFixupsBlocker::ChainEntryOutOfBounds)?;
            let bind = ((raw_pointer >> 63) & 1) == 1;
            let next_stride_count = ((raw_pointer >> 51) & 0x0fff) as u16;

            if entry_address == target {
                let common = MachOChainedFixupTargetResolutionCommon {
                    segment_index: u32::try_from(segment_index).unwrap_or(u32::MAX),
                    segment_name: header.name().as_str().to_owned(),
                    page_index,
                    target_address,
                    chain_entry_file_offset: file_offset,
                    raw_pointer,
                    pointer_format,
                    next_stride_count,
                };
                if bind {
                    let import_index = (raw_pointer & 0x00ff_ffff) as u32;
                    let import = imports.import(import_index).cloned().ok_or(
                        MachOChainedFixupsBlocker::ImportOrdinalOutOfBounds { import_index },
                    )?;
                    return Ok(
                        common.into_bind_resolution(import, ((raw_pointer >> 24) & 0xff) as u8)
                    );
                }

                return common.into_rebase_resolution(metadata);
            }

            if next_stride_count == 0 {
                return Err(MachOChainedFixupsBlocker::ChainDidNotReachTarget);
            }
            chain_offset = chain_offset
                .checked_add(u64::from(next_stride_count) * DYLD_CHAINED_PTR_64_STRIDE)
                .ok_or(MachOChainedFixupsBlocker::ChainEntryOutOfBounds)?;
        }

        return Err(MachOChainedFixupsBlocker::ChainWalkLimitExceeded);
    }

    Err(MachOChainedFixupsBlocker::TargetSegmentMissing)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MachOChainedFixupTargetResolutionCommon {
    segment_index: u32,
    segment_name: String,
    page_index: u64,
    target_address: MachOChainedFixupTargetAddress,
    chain_entry_file_offset: u64,
    raw_pointer: u64,
    pointer_format: MachOChainedPointerFormatReport,
    next_stride_count: u16,
}

impl MachOChainedFixupTargetResolutionCommon {
    fn into_bind_resolution(
        self,
        import: MachOChainedImportReport,
        addend: u8,
    ) -> MachOChainedFixupTargetResolutionReport {
        MachOChainedFixupTargetResolutionReport {
            segment_index: self.segment_index,
            segment_name: self.segment_name,
            page_index: self.page_index,
            chain_entry_address: self.target_address,
            chain_entry_file_offset: self.chain_entry_file_offset,
            raw_pointer: self.raw_pointer,
            pointer_format: self.pointer_format,
            pointer_kind: MachOChainedPointerKind::Bind,
            import: Some(import),
            rebase: None,
            addend: Some(addend),
            next_stride_count: self.next_stride_count,
        }
    }

    fn into_rebase_resolution(
        self,
        metadata: &MachOMetadata,
    ) -> Result<MachOChainedFixupTargetResolutionReport, MachOChainedFixupsBlocker> {
        let raw_target = self.raw_pointer & DYLD_CHAINED_PTR_64_REBASE_TARGET_MASK;
        let high8 = ((self.raw_pointer >> 36) & 0xff) as u8;
        let target = raw_target | (u64::from(high8) << 56);
        let resolved_vm_address = match self.pointer_format.kind {
            MachOChainedPointerFormatKind::Ptr64 => target,
            MachOChainedPointerFormatKind::Ptr64Offset => mach_o_image_base_vmaddr(metadata)?
                .checked_add(target)
                .ok_or(MachOChainedFixupsBlocker::RebaseTargetAddressOverflow)?,
            MachOChainedPointerFormatKind::Unsupported => {
                return Err(MachOChainedFixupsBlocker::UnsupportedPointerFormat {
                    pointer_format: self.pointer_format.value,
                });
            }
        };

        Ok(MachOChainedFixupTargetResolutionReport {
            segment_index: self.segment_index,
            segment_name: self.segment_name,
            page_index: self.page_index,
            chain_entry_address: self.target_address,
            chain_entry_file_offset: self.chain_entry_file_offset,
            raw_pointer: self.raw_pointer,
            pointer_format: self.pointer_format,
            pointer_kind: MachOChainedPointerKind::Rebase,
            import: None,
            rebase: Some(MachOChainedRebaseTargetIdentityReport::new(
                raw_target,
                high8,
                resolved_vm_address,
            )),
            addend: None,
            next_stride_count: self.next_stride_count,
        })
    }
}

fn mach_o_image_base_vmaddr(metadata: &MachOMetadata) -> Result<u64, MachOChainedFixupsBlocker> {
    let summary = metadata.load_commands().summary();
    summary
        .recognized_segments()
        .iter()
        .find(|segment| {
            segment.header().fileoff().as_u64() == 0 && segment.header().filesize().as_u64() > 0
        })
        .or_else(|| summary.recognized_segments().first())
        .map(|segment| segment.header().vmaddr().as_u64())
        .ok_or(MachOChainedFixupsBlocker::ImageBaseMissing)
}

fn chained_fixups_command(metadata: &MachOMetadata) -> Option<&RecognizedMachOLinkeditDataCommand> {
    metadata
        .load_commands()
        .summary()
        .recognized_linkedit_data()
        .iter()
        .find(|command| command.command() == MachOLinkeditDataCommandKind::DyldChainedFixups)
}

fn dylib_path_for_public_ordinal(metadata: &MachOMetadata, lib_ordinal: u32) -> Option<String> {
    if lib_ordinal == 0 {
        return None;
    }
    let index = usize::try_from(lib_ordinal - 1).ok()?;
    metadata
        .load_commands()
        .summary()
        .recognized_dylib_imports()
        .get(index)
        .map(|command| command.name().as_str().to_owned())
}

fn checked_segment_relative_add(
    base: u32,
    field_offset: u32,
) -> Result<u32, MachOChainedFixupsBlocker> {
    checked_relative_add(
        base,
        field_offset,
        MachOChainedFixupsBlocker::SegmentStartsOutOfBounds,
    )
}

fn checked_relative_add(
    base: u32,
    offset: u32,
    blocker: MachOChainedFixupsBlocker,
) -> Result<u32, MachOChainedFixupsBlocker> {
    base.checked_add(offset).ok_or(blocker)
}

fn checked_relative_mul(value: u32, multiplier: u32) -> Result<u32, MachOChainedFixupsBlocker> {
    value
        .checked_mul(multiplier)
        .ok_or(MachOChainedFixupsBlocker::PayloadOutOfBounds)
}

const DYLD_CHAINED_IMPORT: u32 = 1;
const DYLD_CHAINED_IMPORT_ADDEND: u32 = 2;
const DYLD_CHAINED_IMPORT_ADDEND64: u32 = 3;
const DYLD_CHAINED_IMPORT_ENTRY_SIZE: u32 = 4;
const DYLD_CHAINED_PTR_64: u16 = 2;
const DYLD_CHAINED_PTR_64_OFFSET: u16 = 6;
const DYLD_CHAINED_PTR_64_REBASE_TARGET_MASK: u64 = 0x0000_000f_ffff_ffff;
const DYLD_CHAINED_PTR_64_STRIDE: u64 = 4;
const DYLD_CHAINED_PTR_START_NONE: u16 = 0xffff;
const DYLD_CHAINED_PTR_START_MULTI: u16 = 0x8000;
const DYLD_CHAINED_STARTS_IN_SEGMENT_MIN_SIZE: u32 = 22;
const MAX_CHAIN_WALK_ENTRIES: usize = 4096;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_format::input::{probe_public_binary_format, BinaryFileBytes, BinaryInput};

    #[test]
    fn chained_fixups_resolve_target_bind_to_import_symbol() {
        let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(
            chained_fixups_fixture(),
        ));
        let report = probe_public_binary_format(&input).expect("fixture probes");
        let metadata = report.metadata().mach_o_metadata();
        let chained = decode_mach_o_chained_fixups_for_target(
            &input,
            metadata,
            MachOChainedFixupTargetAddress::from_mach_o_virtual_address(0x1020),
        );

        assert_eq!(
            chained.status(),
            MachOChainedFixupsTargetStatus::ResolvedImport
        );
        let identity = chained
            .resolved_import_identity()
            .expect("resolved import identity");
        assert_eq!(identity.dylib_path(), Some("/usr/lib/libTest.dylib"));
        assert_eq!(identity.symbol_name(), "_symbol");
        assert!(!identity.is_weak_import());
        assert_eq!(
            serde_json::to_value(identity).expect("identity serializes"),
            serde_json::json!({
                "import_index": 0,
                "lib_ordinal": 1,
                "dylib_path": "/usr/lib/libTest.dylib",
                "weak_import": false,
                "symbol_name": "_symbol"
            })
        );
        assert_eq!(
            serde_json::to_value(chained).expect("report serializes"),
            serde_json::json!({
                "schema": "mach_o_chained_fixups_target_report_v0",
                "status": "resolved_import",
                "target_address": 4128,
                "header": {
                    "fixups_version": 0,
                    "starts_offset": 32,
                    "imports_offset": 64,
                    "symbols_offset": 72,
                    "imports_count": 1,
                    "imports_format": {
                        "value": 1,
                        "kind": "dyld_chained_import"
                    },
                    "symbols_format": 0
                },
                "starts": {
                    "segment_count": 1,
                    "segments": [
                        {
                            "segment_index": 0,
                            "status": "present",
                            "size": 24,
                            "page_size": 4096,
                            "pointer_format": {
                                "value": 6,
                                "kind": "ptr64_offset"
                            },
                            "segment_offset": 0,
                            "max_valid_pointer": 0,
                            "page_count": 1,
                            "page_starts": [32]
                        }
                    ]
                },
                "imports": {
                    "count": 1,
                    "format": {
                        "value": 1,
                        "kind": "dyld_chained_import"
                    },
                    "symbols_format": 0,
                    "imports": [
                        {
                            "import_index": 0,
                            "raw": 513,
                            "lib_ordinal": 1,
                            "dylib_path": "/usr/lib/libTest.dylib",
                            "weak_import": false,
                            "name_offset": 1,
                            "symbol_name": "_symbol"
                        }
                    ]
                },
                "target_resolution": {
                    "segment_index": 0,
                    "segment_name": "__DATA_CONST",
                    "page_index": 0,
                    "chain_entry_address": 4128,
                    "chain_entry_file_offset": 544,
                    "raw_pointer": 9223372036854775808u64,
                    "pointer_format": {
                        "value": 6,
                        "kind": "ptr64_offset"
                    },
                    "pointer_kind": "bind",
                    "import": {
                        "import_index": 0,
                        "raw": 513,
                        "lib_ordinal": 1,
                        "dylib_path": "/usr/lib/libTest.dylib",
                        "weak_import": false,
                        "name_offset": 1,
                        "symbol_name": "_symbol"
                    },
                    "addend": 0,
                    "next_stride_count": 0
                },
                "blocker": null
            })
        );
    }

    #[test]
    fn chained_fixups_resolve_target_rebase_to_vm_address() {
        let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(
            chained_fixups_rebase_fixture(),
        ));
        let report = probe_public_binary_format(&input).expect("fixture probes");
        let metadata = report.metadata().mach_o_metadata();
        let chained = decode_mach_o_chained_fixups_for_target(
            &input,
            metadata,
            MachOChainedFixupTargetAddress::from_mach_o_virtual_address(0x1020),
        );

        assert_eq!(
            chained.status(),
            MachOChainedFixupsTargetStatus::ResolvedRebase
        );
        let target = chained
            .resolved_rebase_target()
            .expect("resolved rebase target");
        assert_eq!(target.resolved_vm_address, 0x1080);
        assert_eq!(
            serde_json::to_value(target).expect("target serializes"),
            serde_json::json!({
                "raw_target": 0x80,
                "high8": 0,
                "resolved_vm_address": 0x1080
            })
        );
        assert_eq!(
            serde_json::to_value(chained).expect("report serializes")["target_resolution"]
                ["pointer_kind"],
            serde_json::json!("rebase")
        );
    }

    fn chained_fixups_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 0x300];
        write_u32(&mut bytes, 0, 0xfeedfacf);
        write_u32(&mut bytes, 4, 0x01000007);
        write_u32(&mut bytes, 8, 3);
        write_u32(&mut bytes, 12, 2);
        write_u32(&mut bytes, 16, 3);
        write_u32(&mut bytes, 20, 136);

        write_segment_64_command(&mut bytes, 32);
        write_chained_fixups_command(&mut bytes, 104);
        write_load_dylib_command(&mut bytes, 120);
        write_chained_fixups_payload(&mut bytes, 0x100);
        write_u64(&mut bytes, 0x220, 1 << 63);

        bytes
    }

    fn chained_fixups_rebase_fixture() -> Vec<u8> {
        let mut bytes = chained_fixups_fixture();
        write_u64(&mut bytes, 0x220, 0x80);
        bytes
    }

    fn write_segment_64_command(bytes: &mut [u8], offset: usize) {
        write_u32(bytes, offset, 0x19);
        write_u32(bytes, offset + 4, 72);
        write_fixed_string(bytes, offset + 8, 16, "__DATA_CONST");
        write_u64(bytes, offset + 24, 0x1000);
        write_u64(bytes, offset + 32, 0x100);
        write_u64(bytes, offset + 40, 0x200);
        write_u64(bytes, offset + 48, 0x100);
    }

    fn write_chained_fixups_command(bytes: &mut [u8], offset: usize) {
        write_u32(bytes, offset, 0x80000034);
        write_u32(bytes, offset + 4, 16);
        write_u32(bytes, offset + 8, 0x100);
        write_u32(bytes, offset + 12, 0x80);
    }

    fn write_load_dylib_command(bytes: &mut [u8], offset: usize) {
        write_u32(bytes, offset, 0xc);
        write_u32(bytes, offset + 4, 48);
        write_u32(bytes, offset + 8, 24);
        write_u32(bytes, offset + 12, 1);
        write_u32(bytes, offset + 16, 1);
        write_u32(bytes, offset + 20, 1);
        write_fixed_string(bytes, offset + 24, 24, "/usr/lib/libTest.dylib");
    }

    fn write_chained_fixups_payload(bytes: &mut [u8], offset: usize) {
        write_u32(bytes, offset, 0);
        write_u32(bytes, offset + 4, 0x20);
        write_u32(bytes, offset + 8, 0x40);
        write_u32(bytes, offset + 12, 0x48);
        write_u32(bytes, offset + 16, 1);
        write_u32(bytes, offset + 20, 1);
        write_u32(bytes, offset + 24, 0);

        write_u32(bytes, offset + 0x20, 1);
        write_u32(bytes, offset + 0x24, 8);
        write_u32(bytes, offset + 0x28, 24);
        write_u16(bytes, offset + 0x2c, 0x1000);
        write_u16(bytes, offset + 0x2e, 6);
        write_u64(bytes, offset + 0x30, 0);
        write_u32(bytes, offset + 0x38, 0);
        write_u16(bytes, offset + 0x3c, 1);
        write_u16(bytes, offset + 0x3e, 0x20);

        write_u32(bytes, offset + 0x40, 0x201);
        bytes[offset + 0x48] = 0;
        write_fixed_string(bytes, offset + 0x49, 8, "_symbol");
    }

    fn write_fixed_string(bytes: &mut [u8], offset: usize, width: usize, value: &str) {
        let raw = value.as_bytes();
        bytes[offset..offset + raw.len()].copy_from_slice(raw);
        for byte in &mut bytes[offset + raw.len()..offset + width] {
            *byte = 0;
        }
    }

    fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
