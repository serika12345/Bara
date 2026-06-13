use bara_ir::X86Va;

use super::*;

#[test]
fn resolves_mach_o_symbol_by_nlist_64_vm_address() {
    let input = mach_o_input_with_one_public_symbol(
        "_OBJC_CLASS_$_BaraGuiHelloWorldDelegate",
        X86Va::new(0x1_0000_5198),
    );
    let probe = probe_public_binary_format(&input).expect("probe succeeds");

    let resolution = resolve_mach_o_symbol_for_x86_va(&input, &probe, X86Va::new(0x1_0000_5198));

    assert_eq!(
        resolution.status(),
        MachOSymbolAddressResolutionStatus::Resolved
    );
    let resolved = resolution
        .resolved_symbol()
        .expect("symbol address resolves");
    assert_eq!(resolved.symbol_table_index().as_u32(), 0);
    assert_eq!(resolved.symbol_vm_address(), X86Va::new(0x1_0000_5198));
    assert_eq!(
        resolved.symbol_name().as_str(),
        "_OBJC_CLASS_$_BaraGuiHelloWorldDelegate"
    );
}

fn mach_o_input_with_one_public_symbol(symbol_name: &str, symbol_address: X86Va) -> BinaryInput {
    const MACH_O_HEADER_WIDTH: u32 = 32;
    const LC_SYMTAB_WIDTH: u32 = 24;
    const NLIST_64_WIDTH: u32 = 16;

    let symoff = MACH_O_HEADER_WIDTH + LC_SYMTAB_WIDTH;
    let stroff = symoff + NLIST_64_WIDTH;
    let string_table = string_table_with_one_symbol(symbol_name);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xfeed_facfu32.to_le_bytes());
    bytes.extend_from_slice(&0x0100_0007u32.to_le_bytes());
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&LC_SYMTAB_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&LC_SYMTAB_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&symoff.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&stroff.to_le_bytes());
    bytes.extend_from_slice(&(string_table.len() as u32).to_le_bytes());

    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.push(0x0f);
    bytes.push(1);
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&symbol_address.value().to_le_bytes());
    bytes.extend_from_slice(&string_table);

    BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes))
}

fn string_table_with_one_symbol(symbol_name: &str) -> Vec<u8> {
    let mut table = Vec::from([0]);
    table.extend_from_slice(symbol_name.as_bytes());
    table.push(0);
    table
}
