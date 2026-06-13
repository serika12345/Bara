use super::{BinaryFormatProbeReport, BinaryInput, MachOSectionMetadata, MachOSymbolIndex};

use serde::{Deserialize, Serialize};

pub fn resolve_mach_o_symbol_stub_for_target(
    input: &BinaryInput,
    report: &BinaryFormatProbeReport,
    target: MachOStubVirtualAddress,
) -> MachOStubSymbolResolution {
    let summary = report
        .metadata()
        .mach_o_metadata()
        .load_commands()
        .summary();
    let Some(stubs_section) = summary
        .recognized_segments()
        .iter()
        .flat_map(|segment| segment.sections())
        .find(|section| section.name().as_str() == "__stubs")
    else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::MissingStubsSection,
        );
    };

    let Some(stub_index) = stub_index_for_target(stubs_section, target) else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::TargetOutsideStubsSection,
        );
    };
    let Some(stub_byte_size) =
        MachOStubByteSize::from_public_section_reserved2(stubs_section.reserved2().as_u32())
    else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::InvalidStubByteSize,
        );
    };

    if !target_is_stub_aligned(stubs_section, target, stub_byte_size) {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::UnalignedStubTarget,
        );
    }

    let Some(dynamic_symbol_table) = summary.recognized_dynamic_symbol_tables().first() else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::MissingDynamicSymbolTable,
        );
    };
    let Some(indirect_symbol_table_slot) =
        MachOIndirectSymbolTableSlot::from_section_reserved1_and_stub_index(
            stubs_section.reserved1().as_u32(),
            stub_index,
        )
    else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::IndirectSymbolTableSlotOverflow,
        );
    };
    if indirect_symbol_table_slot.as_u32() >= dynamic_symbol_table.nindirectsyms().as_u32() {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::IndirectSymbolTableSlotOutOfBounds,
        );
    }
    let Some(indirect_symbol_table_file_offset) =
        MachOIndirectSymbolTableFileOffset::from_table_and_slot(
            dynamic_symbol_table.indirectsymoff().as_u32(),
            indirect_symbol_table_slot,
        )
    else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::IndirectSymbolTableOffsetOverflow,
        );
    };
    let Some(symbol_table_index_value) =
        read_u32_at(input, indirect_symbol_table_file_offset.as_u32())
    else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::IndirectSymbolTableReadOutOfBounds,
        );
    };

    let Some(symbol_table) = summary.recognized_symbol_tables().first() else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::MissingSymbolTable,
        );
    };
    if symbol_table_index_value >= symbol_table.nsyms().as_u32() {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::SymbolTableIndexOutOfBounds,
        );
    }
    let symbol_table_index = MachOSymbolIndex::from_public_linkedit_value(symbol_table_index_value);
    let Some(symbol_name_string_index) = read_nlist_64_string_index(
        input,
        symbol_table.symoff().as_u32(),
        symbol_table_index_value,
    ) else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::SymbolTableReadOutOfBounds,
        );
    };
    if symbol_name_string_index >= symbol_table.strsize().as_u32() {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::StringTableIndexOutOfBounds,
        );
    }
    let Some(symbol_name) = read_symbol_name(
        input,
        symbol_table.stroff().as_u32(),
        symbol_table.strsize().as_u32(),
        symbol_name_string_index,
    ) else {
        return MachOStubSymbolResolution::unresolved(
            MachOStubSymbolResolutionBlocker::SymbolNameReadOutOfBounds,
        );
    };

    MachOStubSymbolResolution::resolved(MachOResolvedStubSymbol {
        section_segment_name: stubs_section.segment_name().as_str().to_owned(),
        section_name: stubs_section.name().as_str().to_owned(),
        stub_address: target,
        stub_byte_size,
        stub_index: MachOStubIndex::from_stub_index(stub_index),
        indirect_symbol_table_slot,
        indirect_symbol_table_file_offset,
        symbol_table_index,
        symbol_name,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOStubSymbolResolution {
    status: MachOStubSymbolResolutionStatus,
    resolved: Option<MachOResolvedStubSymbol>,
    blocker: Option<MachOStubSymbolResolutionBlocker>,
}

impl MachOStubSymbolResolution {
    const fn resolved(resolved: MachOResolvedStubSymbol) -> Self {
        Self {
            status: MachOStubSymbolResolutionStatus::Resolved,
            resolved: Some(resolved),
            blocker: None,
        }
    }

    const fn unresolved(blocker: MachOStubSymbolResolutionBlocker) -> Self {
        Self {
            status: MachOStubSymbolResolutionStatus::Unresolved,
            resolved: None,
            blocker: Some(blocker),
        }
    }

    pub const fn status(&self) -> MachOStubSymbolResolutionStatus {
        self.status
    }

    pub const fn blocker(&self) -> Option<MachOStubSymbolResolutionBlocker> {
        self.blocker
    }

    pub const fn resolved_symbol(&self) -> Option<&MachOResolvedStubSymbol> {
        self.resolved.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOStubSymbolResolutionStatus {
    Resolved,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOStubSymbolResolutionBlocker {
    IndirectSymbolTableOffsetOverflow,
    IndirectSymbolTableReadOutOfBounds,
    IndirectSymbolTableSlotOutOfBounds,
    IndirectSymbolTableSlotOverflow,
    InvalidStubByteSize,
    MissingDynamicSymbolTable,
    MissingStubsSection,
    MissingSymbolTable,
    StringTableIndexOutOfBounds,
    SymbolNameReadOutOfBounds,
    SymbolTableIndexOutOfBounds,
    SymbolTableReadOutOfBounds,
    TargetOutsideStubsSection,
    UnalignedStubTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOResolvedStubSymbol {
    section_segment_name: String,
    section_name: String,
    stub_address: MachOStubVirtualAddress,
    stub_byte_size: MachOStubByteSize,
    stub_index: MachOStubIndex,
    indirect_symbol_table_slot: MachOIndirectSymbolTableSlot,
    indirect_symbol_table_file_offset: MachOIndirectSymbolTableFileOffset,
    symbol_table_index: MachOSymbolIndex,
    symbol_name: MachOStubSymbolName,
}

impl MachOResolvedStubSymbol {
    pub fn section_segment_name(&self) -> &str {
        &self.section_segment_name
    }

    pub fn section_name(&self) -> &str {
        &self.section_name
    }

    pub const fn stub_address(&self) -> MachOStubVirtualAddress {
        self.stub_address
    }

    pub const fn stub_byte_size(&self) -> MachOStubByteSize {
        self.stub_byte_size
    }

    pub const fn stub_index(&self) -> MachOStubIndex {
        self.stub_index
    }

    pub const fn indirect_symbol_table_slot(&self) -> MachOIndirectSymbolTableSlot {
        self.indirect_symbol_table_slot
    }

    pub const fn indirect_symbol_table_file_offset(&self) -> MachOIndirectSymbolTableFileOffset {
        self.indirect_symbol_table_file_offset
    }

    pub const fn symbol_table_index(&self) -> MachOSymbolIndex {
        self.symbol_table_index
    }

    pub fn symbol_name(&self) -> &MachOStubSymbolName {
        &self.symbol_name
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOStubVirtualAddress {
    value: u64,
}

impl MachOStubVirtualAddress {
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    pub const fn as_u64(self) -> u64 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOStubByteSize {
    value: u32,
}

impl MachOStubByteSize {
    const fn from_public_section_reserved2(value: u32) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self { value })
        }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOStubIndex {
    value: u32,
}

impl MachOStubIndex {
    const fn from_stub_index(value: u32) -> Self {
        Self { value }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOIndirectSymbolTableSlot {
    value: u32,
}

impl MachOIndirectSymbolTableSlot {
    const fn from_section_reserved1_and_stub_index(
        reserved1: u32,
        stub_index: u32,
    ) -> Option<Self> {
        match reserved1.checked_add(stub_index) {
            Some(value) => Some(Self { value }),
            None => None,
        }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOIndirectSymbolTableFileOffset {
    value: u32,
}

impl MachOIndirectSymbolTableFileOffset {
    const fn from_table_and_slot(
        indirect_symbol_table_file_offset: u32,
        slot: MachOIndirectSymbolTableSlot,
    ) -> Option<Self> {
        let Some(slot_byte_offset) = slot
            .as_u32()
            .checked_mul(MACH_O_INDIRECT_SYMBOL_WIDTH as u32)
        else {
            return None;
        };
        match indirect_symbol_table_file_offset.checked_add(slot_byte_offset) {
            Some(value) => Some(Self { value }),
            None => None,
        }
    }

    pub const fn as_u32(self) -> u32 {
        self.value
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MachOStubSymbolName {
    value: String,
}

impl MachOStubSymbolName {
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

fn stub_index_for_target(
    stubs_section: &MachOSectionMetadata,
    target: MachOStubVirtualAddress,
) -> Option<u32> {
    let stubs_start = stubs_section.addr().as_u64();
    let stubs_end = stubs_start.checked_add(stubs_section.size().as_u64())?;
    if !(stubs_start..stubs_end).contains(&target.as_u64()) {
        return None;
    }
    let stub_byte_size = u64::from(stubs_section.reserved2().as_u32());
    if stub_byte_size == 0 {
        return None;
    }

    u32::try_from((target.as_u64() - stubs_start) / stub_byte_size).ok()
}

fn target_is_stub_aligned(
    stubs_section: &MachOSectionMetadata,
    target: MachOStubVirtualAddress,
    stub_byte_size: MachOStubByteSize,
) -> bool {
    let stubs_start = stubs_section.addr().as_u64();
    (target.as_u64() - stubs_start).is_multiple_of(u64::from(stub_byte_size.as_u32()))
}

fn read_nlist_64_string_index(
    input: &BinaryInput,
    symbol_table_file_offset: u32,
    symbol_table_index: u32,
) -> Option<u32> {
    let symbol_entry_offset = symbol_table_index.checked_mul(MACH_O_NLIST_64_WIDTH as u32)?;
    let string_index_offset = symbol_table_file_offset.checked_add(symbol_entry_offset)?;
    read_u32_at(input, string_index_offset)
}

fn read_symbol_name(
    input: &BinaryInput,
    string_table_file_offset: u32,
    string_table_byte_size: u32,
    string_index: u32,
) -> Option<MachOStubSymbolName> {
    let string_start = string_table_file_offset.checked_add(string_index)?;
    let string_table_end = string_table_file_offset.checked_add(string_table_byte_size)?;
    let string_start = usize::try_from(string_start).ok()?;
    let string_table_end = usize::try_from(string_table_end).ok()?;
    if string_start >= string_table_end {
        return None;
    }
    let string_table_tail = input.read_bytes_at(string_start, string_table_end - string_start)?;
    let name_end = string_table_tail
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(string_table_tail.len());
    let value = std::str::from_utf8(&string_table_tail[..name_end])
        .ok()?
        .to_owned();

    Some(MachOStubSymbolName { value })
}

fn read_u32_at(input: &BinaryInput, file_offset: u32) -> Option<u32> {
    input.read_little_endian_u32_at(usize::try_from(file_offset).ok()?)
}

const MACH_O_INDIRECT_SYMBOL_WIDTH: usize = 4;
const MACH_O_NLIST_64_WIDTH: usize = 16;
