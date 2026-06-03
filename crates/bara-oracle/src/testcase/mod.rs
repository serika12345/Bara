use std::{error::Error, fmt};

use bara_ir::X86Va;
use bara_isa_x86::{DecodeError, X86Bytes};
use serde::Deserialize;

use crate::{CaseId, CaseIdError};

mod host_trap;

pub use host_trap::{TestCaseHostTrapPlan, TestCaseStdoutTrap, TestCaseStdoutTrapError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestCase {
    case_id: CaseId,
    x86_bytes: X86Bytes,
    abi: TestCaseAbi,
    host_trap_plan: TestCaseHostTrapPlan,
}

impl TestCase {
    pub const fn new(case_id: CaseId, x86_bytes: X86Bytes, abi: TestCaseAbi) -> Self {
        Self::with_host_traps(case_id, x86_bytes, abi, TestCaseHostTrapPlan::none())
    }

    pub const fn with_host_traps(
        case_id: CaseId,
        x86_bytes: X86Bytes,
        abi: TestCaseAbi,
        host_trap_plan: TestCaseHostTrapPlan,
    ) -> Self {
        Self {
            case_id,
            x86_bytes,
            abi,
            host_trap_plan,
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

    pub const fn host_trap_plan(&self) -> &TestCaseHostTrapPlan {
        &self.host_trap_plan
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestCaseU64(u64);

impl TestCaseU64 {
    const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestCaseInputMemory {
    bytes: Vec<u8>,
}

impl TestCaseInputMemory {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, TestCaseInputMemoryError> {
        if bytes.is_empty() {
            return Err(TestCaseInputMemoryError::Empty);
        }

        Ok(Self { bytes })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestCaseInputMemoryError {
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestCaseAbi {
    NoArgsU64,
    OneU64ArgReturnsU64 { argument: TestCaseU64 },
    OneInputMemoryPtrReturnsU64 { memory: TestCaseInputMemory },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestCaseJsonError {
    Json(String),
    CaseId(CaseIdError),
    UnsupportedAbi {
        args: Vec<String>,
        return_value: String,
    },
    UnsupportedArguments {
        expected: String,
        actual_len: usize,
    },
    MissingInputMemory,
    EmptyInputMemory,
    DuplicateStdoutTrap,
    StdoutTrap(TestCaseStdoutTrapError),
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
            Self::UnsupportedArguments {
                expected,
                actual_len,
            } => {
                write!(
                    formatter,
                    "unsupported testcase arguments: expected {expected}, got {actual_len} value(s)"
                )
            }
            Self::MissingInputMemory => write!(formatter, "missing testcase input memory"),
            Self::EmptyInputMemory => write!(formatter, "testcase input memory is empty"),
            Self::DuplicateStdoutTrap => write!(formatter, "duplicate testcase stdout trap"),
            Self::StdoutTrap(error) => write!(formatter, "invalid testcase stdout trap: {error:?}"),
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
    let abi = TestCaseAbi::try_from_parts(dto.abi, dto.arguments, dto.memory)?;
    let host_trap_plan = host_trap::host_trap_plan_from_dtos(dto.host_traps)?;
    let bytes = decode_hex_bytes(&dto.bytes)?;
    let x86_bytes =
        X86Bytes::new(X86Va::new(dto.entry), bytes).map_err(TestCaseJsonError::DecodeInput)?;

    Ok(TestCase::with_host_traps(
        case_id,
        x86_bytes,
        abi,
        host_trap_plan,
    ))
}

#[derive(Deserialize)]
struct TestCaseDto {
    case_id: String,
    entry: u64,
    bytes: String,
    abi: TestCaseAbiDto,
    #[serde(default)]
    arguments: Vec<u64>,
    memory: Option<TestCaseMemoryDto>,
    #[serde(default)]
    host_traps: Vec<host_trap::TestCaseHostTrapDto>,
}

#[derive(Deserialize)]
struct TestCaseAbiDto {
    args: Vec<String>,
    #[serde(rename = "return")]
    return_value: String,
}

#[derive(Deserialize)]
struct TestCaseMemoryDto {
    input: String,
}

impl TestCaseAbi {
    fn try_from_parts(
        abi: TestCaseAbiDto,
        arguments: Vec<u64>,
        memory: Option<TestCaseMemoryDto>,
    ) -> Result<Self, TestCaseJsonError> {
        if abi.args.is_empty() && abi.return_value == "u64" {
            if !arguments.is_empty() {
                return Err(TestCaseJsonError::UnsupportedArguments {
                    expected: String::from("no arguments"),
                    actual_len: arguments.len(),
                });
            }
            return Ok(Self::NoArgsU64);
        }

        if abi.args == ["u64"] && abi.return_value == "u64" {
            let [argument] = arguments.as_slice() else {
                return Err(TestCaseJsonError::UnsupportedArguments {
                    expected: String::from("one u64 argument"),
                    actual_len: arguments.len(),
                });
            };
            return Ok(Self::OneU64ArgReturnsU64 {
                argument: TestCaseU64::new(*argument),
            });
        }

        if abi.args == ["ptr"] && abi.return_value == "u64" {
            if !arguments.is_empty() {
                return Err(TestCaseJsonError::UnsupportedArguments {
                    expected: String::from("no integer arguments for input memory pointer"),
                    actual_len: arguments.len(),
                });
            }
            let memory = memory.ok_or(TestCaseJsonError::MissingInputMemory)?;
            let bytes = decode_hex_bytes(&memory.input)?;
            let memory = TestCaseInputMemory::from_bytes(bytes)
                .map_err(|_| TestCaseJsonError::EmptyInputMemory)?;
            return Ok(Self::OneInputMemoryPtrReturnsU64 { memory });
        }

        Err(TestCaseJsonError::UnsupportedAbi {
            args: abi.args,
            return_value: abi.return_value,
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

    use super::{TestCaseInputMemory, TestCaseStdoutTrap, TestCaseU64};
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
    fn parses_identity_u64_test_case() {
        let test_case =
            test_case_from_json(include_str!("../../../../tests/cases/identity_u64.json"))
                .expect("fixture parses");

        assert_eq!(
            test_case,
            TestCase::new(
                CaseId::new("identity_u64").expect("case id is non-empty"),
                X86Bytes::new(X86Va::new(0), vec![0x48, 0x89, 0xf8, 0xc3])
                    .expect("fixture bytes are non-empty"),
                TestCaseAbi::OneU64ArgReturnsU64 {
                    argument: TestCaseU64::new(123)
                }
            )
        );
    }

    #[test]
    fn parses_ptr_input_memory_test_case() {
        let test_case = test_case_from_json(include_str!(
            "../../../../tests/cases/load_u8_from_rdi_return_72.json"
        ))
        .expect("fixture parses");

        assert_eq!(
            test_case,
            TestCase::new(
                CaseId::new("load_u8_from_rdi_return_72").expect("case id is non-empty"),
                X86Bytes::new(X86Va::new(0), vec![0x0f, 0xb6, 0x07, 0xc3])
                    .expect("fixture bytes are non-empty"),
                TestCaseAbi::OneInputMemoryPtrReturnsU64 {
                    memory: TestCaseInputMemory::from_bytes(vec![0x48])
                        .expect("input memory is non-empty")
                }
            )
        );
    }

    #[test]
    fn parses_stdout_trap_test_case() {
        let test_case = test_case_from_json(include_str!(
            "../../../../tests/cases/stdout_trap_return_0.json"
        ))
        .expect("fixture parses");

        assert_eq!(
            test_case,
            TestCase::with_host_traps(
                CaseId::new("stdout_trap_return_0").expect("case id is non-empty"),
                X86Bytes::new(X86Va::new(0), vec![0x31, 0xc0, 0xc3])
                    .expect("fixture bytes are non-empty"),
                TestCaseAbi::NoArgsU64,
                super::TestCaseHostTrapPlan::stdout(
                    TestCaseStdoutTrap::from_text(String::from("hello trap\n"))
                        .expect("stdout trap text is valid")
                )
            )
        );
    }

    #[test]
    fn rejects_unsupported_abi() {
        let result = test_case_from_json(
            r#"{"case_id":"bad","entry":0,"bytes":"c3","abi":{"args":["u64","u64"],"return":"u64"}}"#,
        );

        assert_eq!(
            result,
            Err(TestCaseJsonError::UnsupportedAbi {
                args: vec![String::from("u64"), String::from("u64")],
                return_value: String::from("u64")
            })
        );
    }
}
