pub mod emit;
pub mod fixup;
pub mod translation_artifact;
pub mod verify;

pub use emit::{
    emit_program, Arm64MachineCode, BranchFixup, BranchFixupKind, EmitError, EmittedFunction,
    EmittedHostTrapRequests,
};
pub use fixup::{ArmPc, PcMapEntry};
pub use translation_artifact::{
    TranslationArtifact, TranslationArtifactError, TranslationCacheIdentity, TranslationSourceHash,
    TranslationSourceHashParseError, TranslationSourceIdentity, TranslationTarget,
    TranslatorVersion, TranslatorVersionParseError,
};
pub use verify::{
    verify_emitted_function, EmittedFunctionVerificationIssue, EmittedFunctionVerificationReport,
};
