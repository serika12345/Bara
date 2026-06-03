use std::{error::Error, fmt};

use serde::Deserialize;
use serde_json::json;

use crate::{test_case_from_json, CaseId, CaseIdError, TestCase, TestCaseJsonError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableManifest {
    executable_id: CaseId,
    entry_function: TestCase,
}

impl ExecutableManifest {
    const fn new(executable_id: CaseId, entry_function: TestCase) -> Self {
        Self {
            executable_id,
            entry_function,
        }
    }

    pub const fn executable_id(&self) -> &CaseId {
        &self.executable_id
    }

    pub const fn entry_function(&self) -> &TestCase {
        &self.entry_function
    }

    pub fn into_entry_function(self) -> TestCase {
        self.entry_function
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableManifestJsonError {
    Json(String),
    ExecutableId(CaseIdError),
    UnsupportedFormat { format: String },
    EntryFunction(TestCaseJsonError),
}

impl fmt::Display for ExecutableManifestJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid executable manifest json: {error}"),
            Self::ExecutableId(error) => {
                write!(formatter, "invalid executable manifest id: {error:?}")
            }
            Self::UnsupportedFormat { format } => {
                write!(
                    formatter,
                    "unsupported executable manifest format: {format}"
                )
            }
            Self::EntryFunction(error) => write!(formatter, "invalid entry function: {error}"),
        }
    }
}

impl Error for ExecutableManifestJsonError {}

pub fn executable_manifest_from_json(
    input: &str,
) -> Result<ExecutableManifest, ExecutableManifestJsonError> {
    let dto: ExecutableManifestDto = serde_json::from_str(input)
        .map_err(|error| ExecutableManifestJsonError::Json(error.to_string()))?;

    if dto.format != "bara-executable-v0" {
        return Err(ExecutableManifestJsonError::UnsupportedFormat { format: dto.format });
    }

    let executable_id =
        CaseId::new(dto.executable_id).map_err(ExecutableManifestJsonError::ExecutableId)?;
    let entry_function_json = json!({
        "case_id": executable_id.as_str(),
        "entry": dto.entry,
        "bytes": dto.code,
        "abi": dto.abi,
        "host_traps": dto.host_traps,
    })
    .to_string();
    let entry_function = test_case_from_json(&entry_function_json)
        .map_err(ExecutableManifestJsonError::EntryFunction)?;

    Ok(ExecutableManifest::new(executable_id, entry_function))
}

#[derive(Deserialize)]
struct ExecutableManifestDto {
    executable_id: String,
    format: String,
    entry: u64,
    code: String,
    abi: serde_json::Value,
    #[serde(default)]
    host_traps: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use crate::{
        executable_manifest_from_json, CaseId, ExecutableManifestJsonError, TestCaseAbi,
        TestCaseJsonError,
    };

    #[test]
    fn parses_bara_executable_manifest_v0_into_entry_function() {
        let manifest = executable_manifest_from_json(include_str!(
            "../../../../tests/executables/hello_world_executable_manifest.json"
        ))
        .expect("executable manifest parses");

        assert_eq!(
            manifest.executable_id(),
            &CaseId::new("hello_world_executable_manifest").expect("id is non-empty")
        );
        assert_eq!(
            manifest.entry_function().case_id().as_str(),
            "hello_world_executable_manifest"
        );
        assert_eq!(manifest.entry_function().abi(), &TestCaseAbi::NoArgsU64);
        assert_eq!(
            manifest
                .entry_function()
                .host_trap_plan()
                .stdout_trap()
                .expect("stdout trap exists")
                .text(),
            "hello world\n"
        );
    }

    #[test]
    fn rejects_unsupported_manifest_format() {
        let result = executable_manifest_from_json(
            r#"{
  "executable_id": "bad_format",
  "format": "elf",
  "entry": 0,
  "code": "c3",
  "abi": { "args": [], "return": "u64" }
}"#,
        );

        assert_eq!(
            result,
            Err(ExecutableManifestJsonError::UnsupportedFormat {
                format: String::from("elf")
            })
        );
    }

    #[test]
    fn reports_entry_function_parse_errors() {
        let result = executable_manifest_from_json(
            r#"{
  "executable_id": "bad_code",
  "format": "bara-executable-v0",
  "entry": 0,
  "code": "0",
  "abi": { "args": [], "return": "u64" }
}"#,
        );

        assert_eq!(
            result,
            Err(ExecutableManifestJsonError::EntryFunction(
                TestCaseJsonError::OddHexLength { hex_len: 1 }
            ))
        );
    }
}
