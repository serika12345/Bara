use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{CaseId, FailureKind, JsonError, ObservedResult, TestCaseHostTrapPlan};
use serde::Serialize;

use crate::function_run::{FunctionArm64Bytes, FunctionStdoutHostTrapRequest};

#[derive(Debug)]
pub(crate) enum NativeArtifactError {
    UnsupportedHost {
        os: &'static str,
        arch: &'static str,
    },
    TempAssemblyPath {
        source: std::time::SystemTimeError,
    },
    WriteAssembly {
        path: PathBuf,
        source: io::Error,
    },
    LinkerSpawn {
        source: io::Error,
    },
    LinkerFailed {
        status: String,
        stderr: String,
    },
    MissingLinkedExecutable {
        path: PathBuf,
    },
    RunArtifact {
        path: PathBuf,
        source: io::Error,
    },
    MissingArtifactExitStatus {
        path: PathBuf,
    },
    NegativeArtifactExitStatus {
        path: PathBuf,
        status: i32,
    },
    StdoutMainUnsupported(NativeStdoutMainUnsupported),
}

impl NativeArtifactError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::UnsupportedHost { .. } => FailureKind::EmitError,
            Self::StdoutMainUnsupported(_) => FailureKind::EmitError,
            Self::RunArtifact { .. }
            | Self::MissingArtifactExitStatus { .. }
            | Self::NegativeArtifactExitStatus { .. } => FailureKind::RunError,
            Self::TempAssemblyPath { .. }
            | Self::WriteAssembly { .. }
            | Self::LinkerSpawn { .. }
            | Self::LinkerFailed { .. }
            | Self::MissingLinkedExecutable { .. } => FailureKind::EmitError,
        }
    }
}

impl fmt::Display for NativeArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedHost { os, arch } => write!(
                formatter,
                "linking ARM64 main executable is unsupported on host {os}/{arch}"
            ),
            Self::TempAssemblyPath { source } => {
                write!(
                    formatter,
                    "failed to build temporary assembly path: {source}"
                )
            }
            Self::WriteAssembly { path, source } => {
                write!(
                    formatter,
                    "failed to write temporary assembly {}: {source}",
                    path.display()
                )
            }
            Self::LinkerSpawn { source } => write!(formatter, "failed to run clang: {source}"),
            Self::LinkerFailed { status, stderr } => {
                write!(formatter, "clang failed with {status}: {stderr}")
            }
            Self::MissingLinkedExecutable { path } => write!(
                formatter,
                "clang completed but output executable does not exist: {}",
                path.display()
            ),
            Self::RunArtifact { path, source } => write!(
                formatter,
                "failed to run native executable artifact {}: {source}",
                path.display()
            ),
            Self::MissingArtifactExitStatus { path } => write!(
                formatter,
                "native executable artifact terminated without exit status: {}",
                path.display()
            ),
            Self::NegativeArtifactExitStatus { path, status } => write!(
                formatter,
                "native executable artifact {} returned negative exit status {status}",
                path.display()
            ),
            Self::StdoutMainUnsupported(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for NativeArtifactError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NativeStdoutMainUnsupported {
    MissingStdoutTrapPlan,
    MissingEmittedStdoutRequest,
}

impl fmt::Display for NativeStdoutMainUnsupported {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStdoutTrapPlan => write!(
                formatter,
                "stdout main executable requires testcase stdout host trap text"
            ),
            Self::MissingEmittedStdoutRequest => write!(
                formatter,
                "stdout main executable requires emitted function stdout request"
            ),
        }
    }
}

impl Error for NativeStdoutMainUnsupported {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RawArm64Bytes<'a> {
    bytes: &'a [u8],
}

impl<'a> RawArm64Bytes<'a> {
    fn from_function(body: FunctionArm64Bytes<'a>) -> Self {
        Self {
            bytes: body.as_slice(),
        }
    }

    #[cfg(test)]
    fn from_trusted_bytes(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    const fn as_slice(self) -> &'a [u8] {
        self.bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeAssemblySource {
    source: String,
}

impl NativeAssemblySource {
    fn main_from_raw_arm64(body: RawArm64Bytes<'_>) -> Self {
        let mut source = String::from(".text\n.globl _main\n.p2align 2\n_main:\n");
        push_byte_directives(&mut source, body.as_slice());
        Self { source }
    }

    fn stdout_main_from_raw_arm64_and_stdout(body: RawArm64Bytes<'_>, stdout_bytes: &[u8]) -> Self {
        let mut source = String::from(".text\n.globl _main\n.p2align 2\n_main:\n");
        source.push_str("stp x29, x30, [sp, #-16]!\n");
        source.push_str("mov x29, sp\n");
        source.push_str("mov x0, #1\n");
        source.push_str("adrp x1, L_stdout_text@PAGE\n");
        source.push_str("add x1, x1, L_stdout_text@PAGEOFF\n");
        source.push_str(&format!("mov x2, #{}\n", stdout_bytes.len()));
        source.push_str("bl _write\n");
        source.push_str("ldp x29, x30, [sp], #16\n");
        push_byte_directives(&mut source, body.as_slice());
        source.push_str(".section __TEXT,__const\n.p2align 2\nL_stdout_text:\n");
        push_byte_directives(&mut source, stdout_bytes);
        Self { source }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.source
    }

    #[cfg(test)]
    fn into_string(self) -> String {
        self.source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkedNativeExecutable {
    path: PathBuf,
    metadata: NativeArtifactMetadata,
}

impl LinkedNativeExecutable {
    #[cfg(test)]
    fn from_existing_path(path: PathBuf) -> Self {
        Self::from_existing_path_with_helper_requirements(
            path,
            NativeArtifactHelperRequirements::none(),
        )
    }

    fn from_existing_path_with_helper_requirements(
        path: PathBuf,
        helper_requirements: NativeArtifactHelperRequirements,
    ) -> Self {
        let metadata =
            NativeArtifactMetadata::linked_executable(path.as_path(), helper_requirements);
        Self { path, metadata }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) const fn metadata(&self) -> &NativeArtifactMetadata {
        &self.metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct NativeArtifactMetadata {
    artifact_kind: NativeArtifactKind,
    target_triple: NativeArtifactTargetTriple,
    toolchain: NativeArtifactToolchain,
    output_path: NativeArtifactOutputPath,
    helper_requirements: Vec<NativeArtifactHelperRequirement>,
}

impl NativeArtifactMetadata {
    fn linked_executable(
        output_path: &Path,
        helper_requirements: NativeArtifactHelperRequirements,
    ) -> Self {
        Self {
            artifact_kind: NativeArtifactKind::LinkedExecutable,
            target_triple: NativeArtifactTargetTriple::Arm64AppleMacos,
            toolchain: NativeArtifactToolchain::Clang,
            output_path: NativeArtifactOutputPath::from_path(output_path),
            helper_requirements: helper_requirements.into_values(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeArtifactKind {
    LinkedExecutable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum NativeArtifactTargetTriple {
    #[serde(rename = "arm64-apple-macos")]
    Arm64AppleMacos,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum NativeArtifactToolchain {
    #[serde(rename = "clang")]
    Clang,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
struct NativeArtifactOutputPath(String);

impl NativeArtifactOutputPath {
    fn from_path(path: &Path) -> Self {
        Self(path.to_string_lossy().into_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeArtifactHelperRequirements {
    values: Vec<NativeArtifactHelperRequirement>,
}

impl NativeArtifactHelperRequirements {
    fn none() -> Self {
        Self { values: Vec::new() }
    }

    fn write_stdout() -> Self {
        Self {
            values: vec![NativeArtifactHelperRequirement::WriteStdout],
        }
    }

    fn into_values(self) -> Vec<NativeArtifactHelperRequirement> {
        self.values
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeArtifactHelperRequirement {
    WriteStdout,
}

pub(crate) fn native_artifact_metadata_to_json(
    metadata: &NativeArtifactMetadata,
) -> Result<String, JsonError> {
    serde_json::to_string(metadata).map_err(JsonError::new)
}

pub(crate) fn link_arm64_main_executable(
    body: FunctionArm64Bytes<'_>,
    output_path: &Path,
) -> Result<LinkedNativeExecutable, NativeArtifactError> {
    ensure_supported_host()?;

    let source = arm64_main_assembly_source(body);
    link_assembly_source(
        &source,
        output_path,
        NativeArtifactHelperRequirements::none(),
    )
}

pub(crate) fn link_arm64_stdout_main_executable(
    body: FunctionArm64Bytes<'_>,
    host_trap_plan: &TestCaseHostTrapPlan,
    stdout_request: FunctionStdoutHostTrapRequest,
    output_path: &Path,
) -> Result<LinkedNativeExecutable, NativeArtifactError> {
    let Some(stdout) = host_trap_plan.stdout_trap() else {
        return Err(NativeArtifactError::StdoutMainUnsupported(
            NativeStdoutMainUnsupported::MissingStdoutTrapPlan,
        ));
    };
    if !stdout_request.is_requested() {
        return Err(NativeArtifactError::StdoutMainUnsupported(
            NativeStdoutMainUnsupported::MissingEmittedStdoutRequest,
        ));
    }

    ensure_supported_host()?;

    let source = arm64_stdout_main_assembly_source(body, stdout.text());
    link_assembly_source(
        &source,
        output_path,
        NativeArtifactHelperRequirements::write_stdout(),
    )
}

pub(crate) fn observe_native_executable_artifact(
    case_id: CaseId,
    executable: &LinkedNativeExecutable,
) -> Result<ObservedResult, NativeArtifactError> {
    let executable_path = executable.path();
    let output = Command::new(executable_path).output().map_err(|source| {
        NativeArtifactError::RunArtifact {
            path: executable_path.to_path_buf(),
            source,
        }
    })?;
    let exit_status =
        output
            .status
            .code()
            .ok_or_else(|| NativeArtifactError::MissingArtifactExitStatus {
                path: executable_path.to_path_buf(),
            })?;
    let return_value = u64::try_from(exit_status).map_err(|_| {
        NativeArtifactError::NegativeArtifactExitStatus {
            path: executable_path.to_path_buf(),
            status: exit_status,
        }
    })?;

    Ok(ObservedResult::new(
        case_id,
        exit_status,
        return_value,
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

fn link_assembly_source(
    source: &NativeAssemblySource,
    output_path: &Path,
    helper_requirements: NativeArtifactHelperRequirements,
) -> Result<LinkedNativeExecutable, NativeArtifactError> {
    let assembly_path = temporary_assembly_path()?;
    fs::write(&assembly_path, source.as_str()).map_err(|source| {
        NativeArtifactError::WriteAssembly {
            path: assembly_path.clone(),
            source,
        }
    })?;

    let output = Command::new("clang")
        .arg(&assembly_path)
        .arg("-o")
        .arg(output_path)
        .output()
        .map_err(|source| NativeArtifactError::LinkerSpawn { source });

    let _ = fs::remove_file(&assembly_path);

    let output = output?;
    if !output.status.success() {
        return Err(NativeArtifactError::LinkerFailed {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    if !output_path.exists() {
        return Err(NativeArtifactError::MissingLinkedExecutable {
            path: output_path.to_path_buf(),
        });
    }

    Ok(
        LinkedNativeExecutable::from_existing_path_with_helper_requirements(
            output_path.to_path_buf(),
            helper_requirements,
        ),
    )
}

fn ensure_supported_host() -> Result<(), NativeArtifactError> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Ok(())
    } else {
        Err(NativeArtifactError::UnsupportedHost {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        })
    }
}

fn temporary_assembly_path() -> Result<PathBuf, NativeArtifactError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| NativeArtifactError::TempAssemblyPath { source })?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("bara-arm64-main-{nanos}.s")))
}

fn arm64_main_assembly_source(body: FunctionArm64Bytes<'_>) -> NativeAssemblySource {
    NativeAssemblySource::main_from_raw_arm64(RawArm64Bytes::from_function(body))
}

#[cfg(test)]
fn arm64_main_assembly_source_from_bytes(bytes: &[u8]) -> String {
    NativeAssemblySource::main_from_raw_arm64(RawArm64Bytes::from_trusted_bytes(bytes))
        .into_string()
}

fn arm64_stdout_main_assembly_source(
    body: FunctionArm64Bytes<'_>,
    stdout_text: &str,
) -> NativeAssemblySource {
    NativeAssemblySource::stdout_main_from_raw_arm64_and_stdout(
        RawArm64Bytes::from_function(body),
        stdout_text.as_bytes(),
    )
}

#[cfg(test)]
fn arm64_stdout_main_assembly_source_from_parts(body_bytes: &[u8], stdout_bytes: &[u8]) -> String {
    NativeAssemblySource::stdout_main_from_raw_arm64_and_stdout(
        RawArm64Bytes::from_trusted_bytes(body_bytes),
        stdout_bytes,
    )
    .into_string()
}

fn push_byte_directives(source: &mut String, bytes: &[u8]) {
    for chunk in bytes.chunks(12) {
        source.push_str(".byte ");
        for (index, byte) in chunk.iter().enumerate() {
            if index > 0 {
                source.push_str(", ");
            }
            source.push_str(&format!("0x{byte:02x}"));
        }
        source.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bara_oracle::FailureKind;

    use super::{
        arm64_main_assembly_source_from_bytes, arm64_stdout_main_assembly_source_from_parts,
        native_artifact_metadata_to_json, LinkedNativeExecutable, NativeArtifactError,
        NativeAssemblySource, NativeStdoutMainUnsupported, RawArm64Bytes,
    };

    #[test]
    fn packaging_and_toolchain_failures_are_emit_errors() {
        let temp_path = PathBuf::from("/tmp/bara-native-artifact-test");
        let time_error = UNIX_EPOCH
            .duration_since(SystemTime::now())
            .expect_err("current time is after Unix epoch");
        let errors = [
            NativeArtifactError::UnsupportedHost {
                os: "test-os",
                arch: "test-arch",
            },
            NativeArtifactError::TempAssemblyPath { source: time_error },
            NativeArtifactError::WriteAssembly {
                path: temp_path.clone(),
                source: io::Error::other("write failed"),
            },
            NativeArtifactError::LinkerSpawn {
                source: io::Error::other("spawn failed"),
            },
            NativeArtifactError::LinkerFailed {
                status: "exit status: 1".to_owned(),
                stderr: "link failed".to_owned(),
            },
            NativeArtifactError::MissingLinkedExecutable {
                path: temp_path.clone(),
            },
            NativeArtifactError::StdoutMainUnsupported(
                NativeStdoutMainUnsupported::MissingStdoutTrapPlan,
            ),
        ];

        for error in errors {
            assert_eq!(error.failure_kind(), FailureKind::EmitError);
        }
    }

    #[test]
    fn native_artifact_execution_failures_are_run_errors() {
        let temp_path = PathBuf::from("/tmp/bara-native-artifact-test");
        let errors = [
            NativeArtifactError::RunArtifact {
                path: temp_path.clone(),
                source: io::Error::other("run failed"),
            },
            NativeArtifactError::MissingArtifactExitStatus {
                path: temp_path.clone(),
            },
            NativeArtifactError::NegativeArtifactExitStatus {
                path: temp_path,
                status: -1,
            },
        ];

        for error in errors {
            assert_eq!(error.failure_kind(), FailureKind::RunError);
        }
    }

    #[test]
    fn native_artifact_types_separate_raw_source_and_linked_executable() {
        let raw =
            RawArm64Bytes::from_trusted_bytes(&[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]);
        let source = NativeAssemblySource::main_from_raw_arm64(raw);
        let executable =
            LinkedNativeExecutable::from_existing_path(PathBuf::from("/tmp/return_42"));

        assert_eq!(
            source.as_str(),
            ".text\n.globl _main\n.p2align 2\n_main:\n.byte 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6\n"
        );
        assert_eq!(executable.path(), Path::new("/tmp/return_42"));
    }

    #[test]
    fn native_artifact_metadata_serializes_as_stable_json() {
        let executable =
            LinkedNativeExecutable::from_existing_path(PathBuf::from("/tmp/return_42"));

        assert_eq!(
            native_artifact_metadata_to_json(executable.metadata())
                .expect("metadata serializes as json"),
            "{\"artifact_kind\":\"linked_executable\",\"target_triple\":\"arm64-apple-macos\",\"toolchain\":\"clang\",\"output_path\":\"/tmp/return_42\",\"helper_requirements\":[]}"
        );
    }

    #[test]
    fn assembly_source_embeds_arm64_main_body_bytes() {
        let source = arm64_main_assembly_source_from_bytes(&[
            0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6,
        ]);

        assert_eq!(
            source,
            ".text\n.globl _main\n.p2align 2\n_main:\n.byte 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6\n"
        );
    }

    #[test]
    fn stdout_main_assembly_source_embeds_write_prologue_and_stdout_bytes() {
        let source = arm64_stdout_main_assembly_source_from_parts(
            &[0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6],
            b"hello world\n",
        );

        assert!(source.contains("stp x29, x30, [sp, #-16]!\n"));
        assert!(source.contains("mov x0, #1\n"));
        assert!(source.contains("adrp x1, L_stdout_text@PAGE\n"));
        assert!(source.contains("add x1, x1, L_stdout_text@PAGEOFF\n"));
        assert!(source.contains("mov x2, #12\n"));
        assert!(source.contains("bl _write\n"));
        assert!(source.contains("ldp x29, x30, [sp], #16\n"));
        assert!(source.contains(
            ".byte 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x0a\n"
        ));
    }
}
