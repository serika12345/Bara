use bara_ir::X86Va;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeError {
    EmptyFunction { entry: X86Va },
    AddressOverflow { at: X86Va, byte_len: u64 },
    TruncatedInstruction { at: X86Va, opcode: u8 },
}
