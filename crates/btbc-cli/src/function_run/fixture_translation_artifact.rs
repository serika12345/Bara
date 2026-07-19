use std::{error::Error, fmt, str::FromStr};

use bara_arm64::{
    TranslationArtifact, TranslationArtifactError, TranslationCacheIdentity, TranslationSourceHash,
    TranslationSourceHashParseError, TranslationSourceIdentity, TranslationTarget,
    TranslatorVersion,
};
use bara_oracle::TestCase;
use sha2::{Digest, Sha256};

use super::{encode_lower_hex, FunctionCompileResult};

const SOURCE_DOMAIN_TAG: &[u8] = b"bara.fixture-translation-source.v1\0";

pub(super) fn build_fixture_translation_artifact(
    test_case: &TestCase,
    compiled: &FunctionCompileResult,
) -> Result<TranslationArtifact, FixtureTranslationArtifactError> {
    let source_hash = fixture_translation_source_hash(test_case)?;
    let source_identity = TranslationSourceIdentity::new(source_hash);
    let cache_identity = TranslationCacheIdentity::new(
        source_hash,
        TranslatorVersion::current(),
        TranslationTarget::Arm64MacOs,
    );

    TranslationArtifact::new(source_identity, compiled.emitted().clone(), cache_identity)
        .map_err(FixtureTranslationArtifactError::Artifact)
}

fn fixture_translation_source_hash(
    test_case: &TestCase,
) -> Result<TranslationSourceHash, FixtureTranslationArtifactError> {
    let source_bytes = test_case.x86_bytes().bytes();
    let source_byte_length = u64::try_from(source_bytes.len())
        .map_err(|_| FixtureTranslationArtifactError::SourceByteLengthOverflow)?;
    let mut hasher = Sha256::new();
    hasher.update(SOURCE_DOMAIN_TAG);
    hasher.update(test_case.x86_bytes().entry().value().to_le_bytes());
    hasher.update(source_byte_length.to_le_bytes());
    hasher.update(source_bytes);
    let digest = hasher.finalize();

    TranslationSourceHash::from_str(&encode_lower_hex(&digest))
        .map_err(FixtureTranslationArtifactError::SourceHash)
}

#[derive(Debug)]
pub(crate) enum FixtureTranslationArtifactError {
    SourceByteLengthOverflow,
    SourceHash(TranslationSourceHashParseError),
    Artifact(TranslationArtifactError),
}

impl fmt::Display for FixtureTranslationArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceByteLengthOverflow => {
                formatter.write_str("fixture source byte length exceeds u64")
            }
            Self::SourceHash(error) => write!(formatter, "invalid fixture source hash: {error}"),
            Self::Artifact(error) => write!(formatter, "invalid translation artifact: {error}"),
        }
    }
}

impl Error for FixtureTranslationArtifactError {}

#[cfg(test)]
mod tests {
    use bara_oracle::test_case_from_json;

    use super::{build_fixture_translation_artifact, TranslationTarget, TranslatorVersion};
    use crate::function_run::compile_test_case_function;

    #[test]
    fn identities_are_stable_for_the_same_source() {
        let test_case = test_case_from_json(include_str!("../../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");
        let same_source_with_different_case_id = test_case_from_json(
            r#"{"case_id":"same_source","entry":0,"bytes":"b82a000000c3","abi":{"args":[],"return":"u64"}}"#,
        )
        .expect("same-source testcase parses");
        let compiled = compile_test_case_function(&test_case).expect("baseline fixture compiles");
        let same_source_compiled = compile_test_case_function(&same_source_with_different_case_id)
            .expect("same-source fixture compiles");
        let artifact = build_fixture_translation_artifact(&test_case, &compiled)
            .expect("fixture artifact should be constructible");
        let same_source_artifact = build_fixture_translation_artifact(
            &same_source_with_different_case_id,
            &same_source_compiled,
        )
        .expect("same-source fixture artifact should be constructible");

        assert_eq!(
            artifact.source_identity(),
            same_source_artifact.source_identity()
        );
        assert_eq!(
            artifact.cache_identity(),
            same_source_artifact.cache_identity()
        );
        assert_eq!(
            artifact.cache_identity().source_hash(),
            artifact.source_identity().source_hash()
        );
        assert_eq!(
            artifact.cache_identity().target(),
            TranslationTarget::Arm64MacOs
        );
        assert_eq!(
            artifact.cache_identity().translator_version(),
            &TranslatorVersion::current()
        );
    }

    #[test]
    fn identities_change_with_guest_entry_or_source_bytes() {
        let baseline = test_case_from_json(
            r#"{"case_id":"baseline","entry":0,"bytes":"b82a000000c3","abi":{"args":[],"return":"u64"}}"#,
        )
        .expect("baseline testcase parses");
        let different_entry = test_case_from_json(
            r#"{"case_id":"different_entry","entry":4096,"bytes":"b82a000000c3","abi":{"args":[],"return":"u64"}}"#,
        )
        .expect("different-entry testcase parses");
        let different_bytes = test_case_from_json(
            r#"{"case_id":"different_bytes","entry":0,"bytes":"b82b000000c3","abi":{"args":[],"return":"u64"}}"#,
        )
        .expect("different-bytes testcase parses");

        let baseline_compiled =
            compile_test_case_function(&baseline).expect("baseline fixture compiles");
        let different_entry_compiled =
            compile_test_case_function(&different_entry).expect("different-entry fixture compiles");
        let different_bytes_compiled =
            compile_test_case_function(&different_bytes).expect("different-bytes fixture compiles");
        let baseline_artifact = build_fixture_translation_artifact(&baseline, &baseline_compiled)
            .expect("baseline fixture artifact should be constructible");
        let different_entry_artifact =
            build_fixture_translation_artifact(&different_entry, &different_entry_compiled)
                .expect("different-entry fixture artifact should be constructible");
        let different_bytes_artifact =
            build_fixture_translation_artifact(&different_bytes, &different_bytes_compiled)
                .expect("different-bytes fixture artifact should be constructible");

        assert_ne!(
            different_entry_artifact.source_identity(),
            baseline_artifact.source_identity()
        );
        assert_ne!(
            different_entry_artifact.cache_identity(),
            baseline_artifact.cache_identity()
        );
        assert_ne!(
            different_bytes_artifact.source_identity(),
            baseline_artifact.source_identity()
        );
        assert_ne!(
            different_bytes_artifact.cache_identity(),
            baseline_artifact.cache_identity()
        );
    }
}
