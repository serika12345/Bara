use std::{error::Error, fmt};

use bara_ir::X86Va;
use bara_isa_x86::X86Bytes;
use serde::Deserialize;

use crate::{CaseId, CaseIdError, TestCase, TestCaseAbi, TestCaseHostTrapPlan, TestCaseJsonError};

mod host_helper;
mod image;

pub use host_helper::{
    HostHelperImport, HostHelperImportTable, HostHelperImportTableError, HostHelperName,
    HostHelperSignature,
};
pub use image::{CodeSegment, ExecutableEntry, ExecutableImage, ExecutableImageError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableManifest {
    executable_id: CaseId,
    image: ExecutableImage,
    abi: TestCaseAbi,
    host_helper_imports: HostHelperImportTable,
    host_trap_plan: TestCaseHostTrapPlan,
}

impl ExecutableManifest {
    const fn new(
        executable_id: CaseId,
        image: ExecutableImage,
        abi: TestCaseAbi,
        host_helper_imports: HostHelperImportTable,
        host_trap_plan: TestCaseHostTrapPlan,
    ) -> Self {
        Self {
            executable_id,
            image,
            abi,
            host_helper_imports,
            host_trap_plan,
        }
    }

    pub const fn executable_id(&self) -> &CaseId {
        &self.executable_id
    }

    pub const fn image(&self) -> &ExecutableImage {
        &self.image
    }

    pub const fn host_helper_imports(&self) -> &HostHelperImportTable {
        &self.host_helper_imports
    }

    pub fn entry_function(&self) -> Result<TestCase, ExecutableManifestJsonError> {
        let entry_bytes = self
            .image
            .entry_function_bytes()
            .map_err(ExecutableManifestJsonError::Image)?;

        Ok(TestCase::with_host_traps(
            self.executable_id.clone(),
            entry_bytes,
            self.abi.clone(),
            self.host_trap_plan.clone(),
        ))
    }

    pub fn into_entry_function(self) -> Result<TestCase, ExecutableManifestJsonError> {
        self.entry_function()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableManifestJsonError {
    Json(String),
    ExecutableId(CaseIdError),
    UnsupportedFormat { format: String },
    EntryFunction(TestCaseJsonError),
    Image(ExecutableImageError),
    HostHelperImports(HostHelperImportTableError),
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
            Self::Image(error) => write!(formatter, "invalid executable image: {error:?}"),
            Self::HostHelperImports(error) => {
                write!(formatter, "invalid host helper imports: {error:?}")
            }
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
    let abi = TestCaseAbi::try_from_parts(dto.abi, dto.arguments, dto.memory)
        .map_err(ExecutableManifestJsonError::EntryFunction)?;
    let host_trap_plan = crate::testcase::host_trap::host_trap_plan_from_dtos(dto.host_traps)
        .map_err(ExecutableManifestJsonError::EntryFunction)?;
    let host_helper_imports = host_helper::host_helper_import_table_from_dtos(dto.imports)
        .map_err(ExecutableManifestJsonError::HostHelperImports)?;
    host_helper::validate_host_trap_imports(&host_trap_plan, &host_helper_imports)
        .map_err(ExecutableManifestJsonError::HostHelperImports)?;
    let bytes = crate::testcase::decode_hex_bytes(&dto.code)
        .map_err(ExecutableManifestJsonError::EntryFunction)?;
    let code_segment = CodeSegment::from_x86_bytes(
        X86Bytes::new(X86Va::new(0), bytes)
            .map_err(TestCaseJsonError::DecodeInput)
            .map_err(ExecutableManifestJsonError::EntryFunction)?,
    );
    let entry = ExecutableEntry::new(X86Va::new(dto.entry));
    let image =
        ExecutableImage::new(code_segment, entry).map_err(ExecutableManifestJsonError::Image)?;

    Ok(ExecutableManifest::new(
        executable_id,
        image,
        abi,
        host_helper_imports,
        host_trap_plan,
    ))
}

#[derive(Deserialize)]
struct ExecutableManifestDto {
    executable_id: String,
    format: String,
    entry: u64,
    code: String,
    abi: crate::testcase::TestCaseAbiDto,
    #[serde(default)]
    arguments: Vec<u64>,
    memory: Option<crate::testcase::TestCaseMemoryDto>,
    #[serde(default)]
    host_traps: Vec<crate::testcase::host_trap::TestCaseHostTrapDto>,
    #[serde(default)]
    imports: Vec<host_helper::ExecutableImportDto>,
}

#[cfg(test)]
mod tests {
    use crate::{
        executable_manifest_from_json, CaseId, ExecutableImageError, ExecutableManifestJsonError,
        HostHelperImportTableError, HostHelperName, HostHelperSignature, TestCaseAbi,
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
        let entry_function = manifest
            .entry_function()
            .expect("entry function is inside image");
        assert_eq!(
            entry_function.case_id().as_str(),
            "hello_world_executable_manifest"
        );
        assert_eq!(entry_function.abi(), &TestCaseAbi::NoArgsU64);
        assert_eq!(
            entry_function
                .host_trap_plan()
                .stdout_trap()
                .expect("stdout trap exists")
                .text(),
            "hello world\n"
        );
        let write_stdout = manifest
            .host_helper_imports()
            .write_stdout()
            .expect("write_stdout import exists");
        assert_eq!(write_stdout.name(), HostHelperName::WriteStdout);
        assert_eq!(write_stdout.signature(), HostHelperSignature::PtrLenToUnit);
    }

    #[test]
    fn entry_function_starts_at_manifest_entry_offset() {
        let manifest = executable_manifest_from_json(
            r#"{
  "executable_id": "entry_offset",
  "format": "bara-executable-v0",
  "entry": 2,
  "code": "0f0b31c0c3",
  "abi": { "args": [], "return": "u64" }
}"#,
        )
        .expect("entry offset manifest parses");

        let entry_function = manifest
            .entry_function()
            .expect("entry offset is in segment");

        assert_eq!(entry_function.x86_bytes().bytes(), &[0x31, 0xc0, 0xc3]);
    }

    #[test]
    fn rejects_entry_outside_code_segment() {
        let result = executable_manifest_from_json(
            r#"{
  "executable_id": "bad_entry",
  "format": "bara-executable-v0",
  "entry": 1,
  "code": "c3",
  "abi": { "args": [], "return": "u64" }
}"#,
        );

        assert_eq!(
            result,
            Err(ExecutableManifestJsonError::Image(
                ExecutableImageError::EntryOutOfCodeSegment
            ))
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

    #[test]
    fn rejects_stdout_trap_without_write_stdout_import() {
        let result = executable_manifest_from_json(
            r#"{
  "executable_id": "missing_stdout_import",
  "format": "bara-executable-v0",
  "entry": 0,
  "code": "0f0b31c0c3",
  "abi": { "args": [], "return": "u64" },
  "host_traps": [
    { "kind": "stdout", "text": "hello world\n" }
  ]
}"#,
        );

        assert_eq!(
            result,
            Err(ExecutableManifestJsonError::HostHelperImports(
                HostHelperImportTableError::MissingRequiredImport {
                    helper: HostHelperName::WriteStdout
                }
            ))
        );
    }
}
