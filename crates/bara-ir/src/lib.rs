pub mod block;
pub mod boundary;
pub mod program;
pub mod validate;

pub use block::{
    BasicBlock, BasicBlockError, BlockId, HostTrapKind, IrOp, Operand, Terminator,
    UnsupportedReason, X86Reg,
};
pub use boundary::{
    BoundaryRequest, ExternalCallRequest, ExternalCallRequestError, ExternalSymbolId,
    HelperRequest, SyscallAbi, SyscallRequest, SyscallRequestError,
};
pub use program::{Program, ProgramError, X86Va};
pub use validate::{validate_program, ValidationIssue, ValidationReport};
