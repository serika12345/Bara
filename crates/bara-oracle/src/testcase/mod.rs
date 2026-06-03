use std::{error::Error, fmt};

use bara_ir::X86Va;
use bara_isa_x86::{DecodeError, X86Bytes};
use serde::Deserialize;

use crate::{CaseId, CaseIdError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestCase {
    case_id: CaseId,
    x86_bytes: X86Bytes,
    abi: TestCaseAbi,
}

impl TestCase {
    pub const fn new(case_id: CaseId, x86_bytes: X86Bytes, abi: TestCaseAbi) -> Self {
        Self {
            case_id,
            x86_bytes,
            abi,
        }
    }

    pub const fn case_id(&self) -> &CaseId {
        &self.case_id
    }

    pub const fn x86_bytes(&self) -> &X86Bytes {
        &self.x86_bytes
    }

    pub const fn abi(&self) -> &TestCaseAbi {
        &self.abi
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestCaseAbi {
    NoArgsU64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestCaseJsonError {
    Json(String),
    CaseId(CaseIdError),
    UnsupportedAbi {
        args: Vec<String>,
        return_value: String,
    },
    OddHexLength {
        hex_len: usize,
    },
    InvalidHexDigit {
        at: usize,
    },
    DecodeInput(DecodeError),
}

impl fmt::Display for TestCaseJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid testcase json: {error}"),
            Self::CaseId(error) => write!(formatter, "invalid testcase case id: {error:?}"),
            Self::UnsupportedAbi { args, return_value } => {
                write!(
                    formatter,
                    "unsupported testcase abi: args={args:?}, return={return_value}"
                )
            }
            Self::OddHexLength { hex_len } => {
                write!(formatter, "hex byte string has odd length: {hex_len}")
            }
            Self::InvalidHexDigit { at } => write!(formatter, "invalid hex digit at index {at}"),
            Self::DecodeInput(error) => write!(formatter, "invalid x86 input bytes: {error:?}"),
        }
    }
}

impl Error for TestCaseJsonError {}

pub fn test_case_from_json(input: &str) -> Result<TestCase, TestCaseJsonError> {
    let dto: TestCaseDto =
        serde_json::from_str(input).map_err(|error| TestCaseJsonError::Json(error.to_string()))?;
    let case_id = CaseId::new(dto.case_id).map_err(TestCaseJsonError::CaseId)?;
    let abi = TestCaseAbi::try_from(dto.abi)?;
    let bytes = decode_hex_bytes(&dto.bytes)?;
    let x86_bytes =
        X86Bytes::new(X86Va::new(dto.entry), bytes).map_err(TestCaseJsonError::DecodeInput)?;

    Ok(TestCase::new(case_id, x86_bytes, abi))
}

#[derive(Deserialize)]
struct TestCaseDto {
    case_id: String,
    entry: u64,
    bytes: String,
    abi: TestCaseAbiDto,
}

#[derive(Deserialize)]
struct TestCaseAbiDto {
    args: Vec<String>,
    #[serde(rename = "return")]
    return_value: String,
}

impl TryFrom<TestCaseAbiDto> for TestCaseAbi {
    type Error = TestCaseJsonError;

    fn try_from(value: TestCaseAbiDto) -> Result<Self, Self::Error> {
        if value.args.is_empty() && value.return_value == "u64" {
            return Ok(Self::NoArgsU64);
        }

        Err(TestCaseJsonError::UnsupportedAbi {
            args: value.args,
            return_value: value.return_value,
        })
    }
}

fn decode_hex_bytes(input: &str) -> Result<Vec<u8>, TestCaseJsonError> {
    if !input.len().is_multiple_of(2) {
        return Err(TestCaseJsonError::OddHexLength {
            hex_len: input.len(),
        });
    }

    input
        .as_bytes()
        .chunks_exact(2)
        .enumerate()
        .map(|(index, chunk)| decode_hex_byte(chunk).map_err(|_| invalid_hex_digit(input, index)))
        .collect()
}

fn decode_hex_byte(chunk: &[u8]) -> Result<u8, ()> {
    let high = decode_hex_nibble(chunk[0])?;
    let low = decode_hex_nibble(chunk[1])?;
    Ok((high << 4) | low)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    }
}

fn invalid_hex_digit(input: &str, byte_index: usize) -> TestCaseJsonError {
    let pair_start = byte_index * 2;
    let pair = &input.as_bytes()[pair_start..pair_start + 2];
    let offset = if decode_hex_nibble(pair[0]).is_err() {
        pair_start
    } else {
        pair_start + 1
    };
    TestCaseJsonError::InvalidHexDigit { at: offset }
}

#[cfg(test)]
mod tests {
    use bara_ir::X86Va;
    use bara_isa_x86::X86Bytes;

    use crate::{test_case_from_json, CaseId, TestCase, TestCaseAbi, TestCaseJsonError};

    #[test]
    fn parses_return_42_test_case() {
        let test_case = test_case_from_json(include_str!("../../../../tests/cases/return_42.json"))
            .expect("fixture parses");

        assert_eq!(
            test_case,
            TestCase::new(
                CaseId::new("return_42").expect("case id is non-empty"),
                X86Bytes::new(X86Va::new(0), vec![0xb8, 0x2a, 0, 0, 0, 0xc3])
                    .expect("fixture bytes are non-empty"),
                TestCaseAbi::NoArgsU64
            )
        );
    }

    #[test]
    fn rejects_odd_length_hex_bytes() {
        let result = test_case_from_json(
            r#"{"case_id":"bad","entry":0,"bytes":"c","abi":{"args":[],"return":"u64"}}"#,
        );

        assert_eq!(result, Err(TestCaseJsonError::OddHexLength { hex_len: 1 }));
    }

    #[test]
    fn rejects_invalid_hex_digit() {
        let result = test_case_from_json(
            r#"{"case_id":"bad","entry":0,"bytes":"cg","abi":{"args":[],"return":"u64"}}"#,
        );

        assert_eq!(result, Err(TestCaseJsonError::InvalidHexDigit { at: 1 }));
    }

    #[test]
    fn rejects_unsupported_abi() {
        let result = test_case_from_json(
            r#"{"case_id":"bad","entry":0,"bytes":"c3","abi":{"args":["u64"],"return":"u64"}}"#,
        );

        assert_eq!(
            result,
            Err(TestCaseJsonError::UnsupportedAbi {
                args: vec![String::from("u64")],
                return_value: String::from("u64")
            })
        );
    }
}
