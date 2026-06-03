mod bytes;
mod error;
mod immediate;
mod instruction;
mod parser;

#[cfg(test)]
mod tests;

pub use bytes::X86Bytes;
pub use error::DecodeError;
pub use immediate::{X86Imm32, X86Imm8};
pub use instruction::{DecodedFunction, DecodedInstruction, DecodedInstructionKind};

pub fn decode_function(input: &X86Bytes) -> Result<DecodedFunction, DecodeError> {
    parser::parse_function(input)
}
