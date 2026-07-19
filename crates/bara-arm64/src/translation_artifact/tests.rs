use std::str::FromStr;

use bara_ir::X86Va;

use crate::{
    Arm64MachineCode, ArmPc, BranchFixup, BranchFixupKind, EmittedFunction,
    EmittedHostTrapRequests, PcMapEntry,
};

use super::{
    TranslationArtifact, TranslationArtifactError, TranslationCacheIdentity, TranslationSourceHash,
    TranslationSourceHashParseError, TranslationSourceIdentity, TranslationTarget,
    TranslatorVersion, TranslatorVersionParseError,
};

const SOURCE_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const DIFFERENT_SOURCE_HASH: &str =
    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

#[test]
fn artifact_preserves_emitted_function_and_typed_identities() {
    let source_identity = source_identity(SOURCE_HASH);
    let cache_identity = cache_identity(SOURCE_HASH);
    let emitted = emitted_function();

    let artifact = TranslationArtifact::new(
        source_identity.clone(),
        emitted.clone(),
        cache_identity.clone(),
    )
    .expect("matching source and cache identities should construct an artifact");

    assert_eq!(artifact.source_identity(), &source_identity);
    assert_eq!(artifact.emitted_function(), &emitted);
    assert_eq!(artifact.cache_identity(), &cache_identity);
    assert_eq!(artifact.emitted_function().pc_map(), emitted.pc_map());
    assert_eq!(
        artifact.emitted_function().branch_fixups(),
        emitted.branch_fixups()
    );
    assert_eq!(
        artifact.emitted_function().host_trap_requests(),
        emitted.host_trap_requests()
    );
    assert_eq!(cache_identity.source_hash(), source_identity.source_hash());
    assert_eq!(cache_identity.target(), TranslationTarget::Arm64MacOs);
    assert_eq!(cache_identity.translator_version().to_string(), "0.1.0");
}

#[test]
fn artifact_rejects_a_cache_identity_for_different_source_bytes() {
    let source_identity = source_identity(SOURCE_HASH);
    let cache_identity = cache_identity(DIFFERENT_SOURCE_HASH);

    let result = TranslationArtifact::new(source_identity, emitted_function(), cache_identity);

    assert_eq!(result, Err(TranslationArtifactError::SourceHashMismatch));
}

#[test]
fn source_hash_requires_a_complete_hex_digest() {
    assert_eq!(
        TranslationSourceHash::from_str("0123"),
        Err(TranslationSourceHashParseError::InvalidLength)
    );
    assert_eq!(
        TranslationSourceHash::from_str(
            "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        Err(TranslationSourceHashParseError::InvalidHexDigit)
    );
}

#[test]
fn translator_version_rejects_empty_or_non_canonical_values() {
    assert_eq!(
        TranslatorVersion::from_str(""),
        Err(TranslatorVersionParseError::Empty)
    );
    assert_eq!(
        TranslatorVersion::from_str(" 0.1.0"),
        Err(TranslatorVersionParseError::InvalidCharacter)
    );
}

#[test]
fn current_translator_version_is_owned_by_the_arm64_backend() {
    assert_eq!(
        TranslatorVersion::current().to_string(),
        env!("CARGO_PKG_VERSION")
    );
}

fn source_identity(hash: &str) -> TranslationSourceIdentity {
    TranslationSourceIdentity::new(
        TranslationSourceHash::from_str(hash).expect("test source hash should be valid"),
    )
}

fn cache_identity(hash: &str) -> TranslationCacheIdentity {
    TranslationCacheIdentity::new(
        TranslationSourceHash::from_str(hash).expect("test source hash should be valid"),
        TranslatorVersion::from_str("0.1.0").expect("test translator version should be valid"),
        TranslationTarget::Arm64MacOs,
    )
}

fn emitted_function() -> EmittedFunction {
    EmittedFunction::with_metadata(
        Arm64MachineCode::new(vec![0xc0, 0x03, 0x5f, 0xd6])
            .expect("test ARM64 machine code should be valid"),
        vec![PcMapEntry::new(X86Va::new(0x1000), ArmPc::new(0))],
        vec![BranchFixup::for_test(
            ArmPc::new(0),
            ArmPc::new(0),
            X86Va::new(0x1000),
            BranchFixupKind::Unconditional,
        )],
        EmittedHostTrapRequests::stdout(),
    )
}
