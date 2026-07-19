use std::{error::Error, fmt, str::FromStr};

use bara_arm64::{
    TranslationSourceHash, TranslationSourceHashParseError, TranslationSourceIdentity,
};
use sha2::{Digest, Sha256};

use super::encode_lower_hex;

const SOURCE_DOMAIN_TAG: &[u8] =
    b"bara.b8-mach-o-x86_64-lc-main-first-block-translation-source.v1\0";

pub(super) fn b8_lc_main_first_block_translation_source_identity(
    source_image_bytes: &[u8],
) -> Result<TranslationSourceIdentity, B8LcMainTranslationSourceIdentityError> {
    let source_image_byte_length = u64::try_from(source_image_bytes.len())
        .map_err(|_| B8LcMainTranslationSourceIdentityError::SourceImageByteLengthOverflow)?;
    let mut hasher = Sha256::new();
    hasher.update(SOURCE_DOMAIN_TAG);
    hasher.update(source_image_byte_length.to_le_bytes());
    hasher.update(source_image_bytes);
    let digest = hasher.finalize();
    let source_hash = TranslationSourceHash::from_str(&encode_lower_hex(&digest))
        .map_err(B8LcMainTranslationSourceIdentityError::SourceHash)?;

    Ok(TranslationSourceIdentity::new(source_hash))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum B8LcMainTranslationSourceIdentityError {
    SourceImageByteLengthOverflow,
    SourceHash(TranslationSourceHashParseError),
}

impl fmt::Display for B8LcMainTranslationSourceIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceImageByteLengthOverflow => {
                formatter.write_str("Mach-O source image byte length exceeds u64")
            }
            Self::SourceHash(error) => {
                write!(formatter, "invalid Mach-O translation source hash: {error}")
            }
        }
    }
}

impl Error for B8LcMainTranslationSourceIdentityError {}

#[cfg(test)]
mod tests {
    use super::b8_lc_main_first_block_translation_source_identity;

    #[test]
    fn source_identity_is_stable_for_the_same_image() {
        let source_image = b"Mach-O source image";

        let first = b8_lc_main_first_block_translation_source_identity(source_image)
            .expect("source image identity should be constructible");
        let second = b8_lc_main_first_block_translation_source_identity(source_image)
            .expect("same source image identity should be constructible");

        assert_eq!(first, second);
    }

    #[test]
    fn source_identity_changes_when_mapped_data_after_the_entry_prefix_changes() {
        let first_source_image = b"same-entry-prefix\0mapped-data-a";
        let second_source_image = b"same-entry-prefix\0mapped-data-b";

        let first = b8_lc_main_first_block_translation_source_identity(first_source_image)
            .expect("first source image identity should be constructible");
        let second = b8_lc_main_first_block_translation_source_identity(second_source_image)
            .expect("second source image identity should be constructible");

        assert_ne!(first, second);
    }

    #[test]
    fn empty_source_image_has_a_domain_separated_identity() {
        let empty = b8_lc_main_first_block_translation_source_identity(&[])
            .expect("empty source image remains a hashable domain input");
        let non_empty = b8_lc_main_first_block_translation_source_identity(&[0])
            .expect("non-empty source image identity should be constructible");

        assert_ne!(empty, non_empty);
    }
}
