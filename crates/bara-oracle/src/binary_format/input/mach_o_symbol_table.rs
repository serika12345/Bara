use super::{BinaryFormatProbeReport, BinaryInput, MachOSymbolIndex};

use bara_ir::X86Va;
use serde::Serialize;

pub fn resolve_mach_o_symbol_for_x86_va(
    input: &BinaryInput,
    report: &BinaryFormatProbeReport,
    address: X86Va,
) -> MachOSymbolAddressResolution {
    let summary = report
        .metadata()
        .mach_o_metadata()
        .load_commands()
        .summary();
    let Some(symbol_table) = summary.recognized_symbol_tables().first() else {
        return MachOSymbolAddressResolution::unresolved(
            MachOSymbolAddressResolutionBlocker::MissingSymbolTable,
        );
    };

    for symbol_table_index in 0..symbol_table.nsyms().as_u32() {
        let symbol = match read_nlist_64_symbol(
            input,
            symbol_table.symoff().as_u32(),
            symbol_table.stroff().as_u32(),
            symbol_table.strsize().as_u32(),
            symbol_table_index,
        ) {
            Ok(symbol) => symbol,
            Err(blocker) => return MachOSymbolAddressResolution::unresolved(blocker),
        };

        if symbol.vm_address == address.value() {
            return MachOSymbolAddressResolution::resolved(MachOResolvedAddressSymbol {
                symbol_table_index: MachOSymbolIndex::from_public_linkedit_value(
                    symbol_table_index,
                ),
                symbol_vm_address: address,
                symbol_name: MachOSymbolName {
                    value: symbol.symbol_name,
                },
            });
        }
    }

    MachOSymbolAddressResolution::unresolved(MachOSymbolAddressResolutionBlocker::SymbolNotFound)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOSymbolAddressResolution {
    status: MachOSymbolAddressResolutionStatus,
    resolved: Option<MachOResolvedAddressSymbol>,
    blocker: Option<MachOSymbolAddressResolutionBlocker>,
}

impl MachOSymbolAddressResolution {
    fn resolved(resolved: MachOResolvedAddressSymbol) -> Self {
        Self {
            status: MachOSymbolAddressResolutionStatus::Resolved,
            resolved: Some(resolved),
            blocker: None,
        }
    }

    const fn unresolved(blocker: MachOSymbolAddressResolutionBlocker) -> Self {
        Self {
            status: MachOSymbolAddressResolutionStatus::Unresolved,
            resolved: None,
            blocker: Some(blocker),
        }
    }

    pub const fn status(&self) -> MachOSymbolAddressResolutionStatus {
        self.status
    }

    pub const fn blocker(&self) -> Option<MachOSymbolAddressResolutionBlocker> {
        self.blocker
    }

    pub const fn resolved_symbol(&self) -> Option<&MachOResolvedAddressSymbol> {
        self.resolved.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOSymbolAddressResolutionStatus {
    Resolved,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOSymbolAddressResolutionBlocker {
    MissingSymbolTable,
    StringTableIndexOutOfBounds,
    SymbolNameReadOutOfBounds,
    SymbolNotFound,
    SymbolTableReadOutOfBounds,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOResolvedAddressSymbol {
    symbol_table_index: MachOSymbolIndex,
    symbol_vm_address: X86Va,
    symbol_name: MachOSymbolName,
}

impl MachOResolvedAddressSymbol {
    pub const fn symbol_table_index(&self) -> MachOSymbolIndex {
        self.symbol_table_index
    }

    pub const fn symbol_vm_address(&self) -> X86Va {
        self.symbol_vm_address
    }

    pub fn symbol_name(&self) -> &MachOSymbolName {
        &self.symbol_name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOSymbolName {
    value: String,
}

impl MachOSymbolName {
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

struct MachONlist64Symbol {
    symbol_name: String,
    vm_address: u64,
}

fn read_nlist_64_symbol(
    input: &BinaryInput,
    symbol_table_file_offset: u32,
    string_table_file_offset: u32,
    string_table_byte_size: u32,
    symbol_table_index: u32,
) -> Result<MachONlist64Symbol, MachOSymbolAddressResolutionBlocker> {
    let symbol_entry_offset = symbol_table_index
        .checked_mul(MACH_O_NLIST_64_WIDTH as u32)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolTableReadOutOfBounds)?;
    let nlist_offset = symbol_table_file_offset
        .checked_add(symbol_entry_offset)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolTableReadOutOfBounds)?;
    let symbol_name_string_index = read_u32_at(input, nlist_offset)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolTableReadOutOfBounds)?;
    if symbol_name_string_index >= string_table_byte_size {
        return Err(MachOSymbolAddressResolutionBlocker::StringTableIndexOutOfBounds);
    }
    let symbol_name = read_symbol_name(
        input,
        string_table_file_offset,
        string_table_byte_size,
        symbol_name_string_index,
    )?;
    let vm_address_offset = nlist_offset
        .checked_add(MACH_O_NLIST_64_N_VALUE_OFFSET as u32)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolTableReadOutOfBounds)?;
    let vm_address = read_u64_at(input, vm_address_offset)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolTableReadOutOfBounds)?;

    Ok(MachONlist64Symbol {
        symbol_name,
        vm_address,
    })
}

fn read_symbol_name(
    input: &BinaryInput,
    string_table_file_offset: u32,
    string_table_byte_size: u32,
    string_index: u32,
) -> Result<String, MachOSymbolAddressResolutionBlocker> {
    let string_start = string_table_file_offset
        .checked_add(string_index)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolNameReadOutOfBounds)?;
    let string_table_end = string_table_file_offset
        .checked_add(string_table_byte_size)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolNameReadOutOfBounds)?;
    let string_start = usize::try_from(string_start)
        .map_err(|_| MachOSymbolAddressResolutionBlocker::SymbolNameReadOutOfBounds)?;
    let string_table_end = usize::try_from(string_table_end)
        .map_err(|_| MachOSymbolAddressResolutionBlocker::SymbolNameReadOutOfBounds)?;
    if string_start >= string_table_end {
        return Err(MachOSymbolAddressResolutionBlocker::StringTableIndexOutOfBounds);
    }
    let string_table_tail = input
        .read_bytes_at(string_start, string_table_end - string_start)
        .ok_or(MachOSymbolAddressResolutionBlocker::SymbolNameReadOutOfBounds)?;
    let name_end = string_table_tail
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(string_table_tail.len());
    let value = std::str::from_utf8(&string_table_tail[..name_end])
        .map_err(|_| MachOSymbolAddressResolutionBlocker::SymbolNameReadOutOfBounds)?
        .to_owned();

    Ok(value)
}

fn read_u32_at(input: &BinaryInput, file_offset: u32) -> Option<u32> {
    input.read_little_endian_u32_at(usize::try_from(file_offset).ok()?)
}

fn read_u64_at(input: &BinaryInput, file_offset: u32) -> Option<u64> {
    input.read_little_endian_u64_at(usize::try_from(file_offset).ok()?)
}

const MACH_O_NLIST_64_WIDTH: usize = 16;
const MACH_O_NLIST_64_N_VALUE_OFFSET: usize = 8;
