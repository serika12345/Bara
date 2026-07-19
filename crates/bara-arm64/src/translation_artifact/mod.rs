use std::{error::Error, fmt, str::FromStr};

use crate::EmittedFunction;

const SOURCE_HASH_BYTE_LENGTH: usize = 32;
const SOURCE_HASH_HEX_LENGTH: usize = SOURCE_HASH_BYTE_LENGTH * 2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TranslationSourceHash {
    bytes: [u8; SOURCE_HASH_BYTE_LENGTH],
}

impl FromStr for TranslationSourceHash {
    type Err = TranslationSourceHashParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != SOURCE_HASH_HEX_LENGTH {
            return Err(TranslationSourceHashParseError::InvalidLength);
        }

        let mut bytes = [0; SOURCE_HASH_BYTE_LENGTH];
        for (destination, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
            let high =
                parse_hex_digit(pair[0]).ok_or(TranslationSourceHashParseError::InvalidHexDigit)?;
            let low =
                parse_hex_digit(pair[1]).ok_or(TranslationSourceHashParseError::InvalidHexDigit)?;
            *destination = (high << 4) | low;
        }

        Ok(Self { bytes })
    }
}

impl fmt::Display for TranslationSourceHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.bytes {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationSourceHashParseError {
    InvalidLength,
    InvalidHexDigit,
}

impl fmt::Display for TranslationSourceHashParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => formatter.write_str("source hash must contain 64 hex digits"),
            Self::InvalidHexDigit => formatter.write_str("source hash contains a non-hex digit"),
        }
    }
}

impl Error for TranslationSourceHashParseError {}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TranslatorVersion(Box<str>);

impl TranslatorVersion {
    pub fn current() -> Self {
        Self(env!("CARGO_PKG_VERSION").into())
    }
}

impl FromStr for TranslatorVersion {
    type Err = TranslatorVersionParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(TranslatorVersionParseError::Empty);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        {
            return Err(TranslatorVersionParseError::InvalidCharacter);
        }

        Ok(Self(value.into()))
    }
}

impl fmt::Display for TranslatorVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslatorVersionParseError {
    Empty,
    InvalidCharacter,
}

impl fmt::Display for TranslatorVersionParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("translator version must not be empty"),
            Self::InvalidCharacter => formatter.write_str(
                "translator version must contain only ASCII letters, digits, '.', '-', or '+'",
            ),
        }
    }
}

impl Error for TranslatorVersionParseError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TranslationTarget {
    Arm64MacOs,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TranslationSourceIdentity {
    source_hash: TranslationSourceHash,
}

impl TranslationSourceIdentity {
    pub const fn new(source_hash: TranslationSourceHash) -> Self {
        Self { source_hash }
    }

    pub const fn source_hash(&self) -> TranslationSourceHash {
        self.source_hash
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TranslationCacheIdentity {
    source_hash: TranslationSourceHash,
    translator_version: TranslatorVersion,
    target: TranslationTarget,
}

impl TranslationCacheIdentity {
    pub const fn new(
        source_hash: TranslationSourceHash,
        translator_version: TranslatorVersion,
        target: TranslationTarget,
    ) -> Self {
        Self {
            source_hash,
            translator_version,
            target,
        }
    }

    pub const fn source_hash(&self) -> TranslationSourceHash {
        self.source_hash
    }

    pub const fn translator_version(&self) -> &TranslatorVersion {
        &self.translator_version
    }

    pub const fn target(&self) -> TranslationTarget {
        self.target
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationArtifact {
    source_identity: TranslationSourceIdentity,
    emitted_function: EmittedFunction,
    cache_identity: TranslationCacheIdentity,
}

impl TranslationArtifact {
    pub fn new(
        source_identity: TranslationSourceIdentity,
        emitted_function: EmittedFunction,
        cache_identity: TranslationCacheIdentity,
    ) -> Result<Self, TranslationArtifactError> {
        if source_identity.source_hash() != cache_identity.source_hash() {
            return Err(TranslationArtifactError::SourceHashMismatch);
        }

        Ok(Self {
            source_identity,
            emitted_function,
            cache_identity,
        })
    }

    pub const fn source_identity(&self) -> &TranslationSourceIdentity {
        &self.source_identity
    }

    pub const fn emitted_function(&self) -> &EmittedFunction {
        &self.emitted_function
    }

    pub const fn cache_identity(&self) -> &TranslationCacheIdentity {
        &self.cache_identity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationArtifactError {
    SourceHashMismatch,
}

impl fmt::Display for TranslationArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceHashMismatch => {
                formatter.write_str("translation source and cache hashes do not match")
            }
        }
    }
}

impl Error for TranslationArtifactError {}

const fn parse_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
