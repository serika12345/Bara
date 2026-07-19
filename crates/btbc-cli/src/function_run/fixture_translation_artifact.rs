use std::{error::Error, fmt, str::FromStr};

use bara_arm64::{
    TranslationSourceHash, TranslationSourceHashParseError, TranslationSourceIdentity,
};
use bara_oracle::TestCase;
use sha2::{Digest, Sha256};

use super::encode_lower_hex;

const SOURCE_DOMAIN_TAG: &[u8] = b"bara.fixture-translation-source.v1\0";

pub(super) fn fixture_translation_source_identity(
    test_case: &TestCase,
) -> Result<TranslationSourceIdentity, FixtureTranslationSourceIdentityError> {
    let source_hash = fixture_translation_source_hash(test_case)?;
    Ok(TranslationSourceIdentity::new(source_hash))
}

fn fixture_translation_source_hash(
    test_case: &TestCase,
) -> Result<TranslationSourceHash, FixtureTranslationSourceIdentityError> {
    let source_bytes = test_case.x86_bytes().bytes();
    let source_byte_length = u64::try_from(source_bytes.len())
        .map_err(|_| FixtureTranslationSourceIdentityError::SourceByteLengthOverflow)?;
    let mut hasher = Sha256::new();
    hasher.update(SOURCE_DOMAIN_TAG);
    hasher.update(test_case.x86_bytes().entry().value().to_le_bytes());
    hasher.update(source_byte_length.to_le_bytes());
    hasher.update(source_bytes);
    let digest = hasher.finalize();

    TranslationSourceHash::from_str(&encode_lower_hex(&digest))
        .map_err(FixtureTranslationSourceIdentityError::SourceHash)
}

#[derive(Debug)]
pub(crate) enum FixtureTranslationSourceIdentityError {
    SourceByteLengthOverflow,
    SourceHash(TranslationSourceHashParseError),
}

impl fmt::Display for FixtureTranslationSourceIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceByteLengthOverflow => {
                formatter.write_str("fixture source byte length exceeds u64")
            }
            Self::SourceHash(error) => write!(formatter, "invalid fixture source hash: {error}"),
        }
    }
}

impl Error for FixtureTranslationSourceIdentityError {}

#[cfg(test)]
mod tests {
    use bara_oracle::test_case_from_json;

    use super::fixture_translation_source_identity;

    #[test]
    fn identities_are_stable_for_the_same_source() {
        let test_case = test_case_from_json(include_str!("../../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");
        let same_source_with_different_case_id = test_case_from_json(
            r#"{"case_id":"same_source","entry":0,"bytes":"b82a000000c3","abi":{"args":[],"return":"u64"}}"#,
        )
        .expect("same-source testcase parses");
        let identity = fixture_translation_source_identity(&test_case)
            .expect("fixture identity should be constructible");
        let same_source_identity =
            fixture_translation_source_identity(&same_source_with_different_case_id)
                .expect("same-source fixture identity should be constructible");

        assert_eq!(identity, same_source_identity);
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

        let baseline_identity = fixture_translation_source_identity(&baseline)
            .expect("baseline identity is constructible");
        let different_entry_identity = fixture_translation_source_identity(&different_entry)
            .expect("different-entry identity is constructible");
        let different_bytes_identity = fixture_translation_source_identity(&different_bytes)
            .expect("different-bytes identity is constructible");

        assert_ne!(different_entry_identity, baseline_identity);
        assert_ne!(different_bytes_identity, baseline_identity);
    }
}
