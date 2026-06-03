pub mod emit;
pub mod fixup;

pub use emit::{
    emit_program, Arm64MachineCode, EmitError, EmittedFunction, EmittedHostTrapRequests,
};
pub use fixup::{ArmPc, PcMapEntry};
