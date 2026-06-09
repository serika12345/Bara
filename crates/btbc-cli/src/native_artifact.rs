use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{
    CaseId, FailureKind, JsonError, MachOEntryPointCommandMetadata, MachOExecutableImageConversion,
    MachOExecutableImageConversionBlocker, MachOSegmentCommandHeaderMetadata, ObservedResult,
    TestCaseHostTrapPlan,
};
use serde::{Serialize, Serializer};

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
            Self::UnsupportedHost { os, arch } => {
                write_unsupported_host_report(formatter, os, arch)
            }
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct NativeArtifactUnsupportedHostReport {
    status: NativeArtifactUnsupportedHostStatus,
    failure_kind: FailureKind,
    artifact_kind: NativeArtifactKind,
    target_triple: NativeArtifactTargetTriple,
    host: NativeArtifactUnsupportedHost,
}

impl NativeArtifactUnsupportedHostReport {
    const fn new(os: &'static str, arch: &'static str) -> Self {
        Self {
            status: NativeArtifactUnsupportedHostStatus::UnsupportedHost,
            failure_kind: FailureKind::EmitError,
            artifact_kind: NativeArtifactKind::LinkedExecutable,
            target_triple: NativeArtifactTargetTriple::Arm64AppleMacos,
            host: NativeArtifactUnsupportedHost::new(os, arch),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeArtifactUnsupportedHostStatus {
    UnsupportedHost,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct NativeArtifactUnsupportedHost {
    os: &'static str,
    arch: &'static str,
}

impl NativeArtifactUnsupportedHost {
    const fn new(os: &'static str, arch: &'static str) -> Self {
        Self { os, arch }
    }
}

fn write_unsupported_host_report(
    formatter: &mut fmt::Formatter<'_>,
    os: &'static str,
    arch: &'static str,
) -> fmt::Result {
    let report = NativeArtifactUnsupportedHostReport::new(os, arch);
    match serde_json::to_string(&report) {
        Ok(json) => formatter.write_str(&json),
        Err(_) => Err(fmt::Error),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NativeStdoutMainUnsupported {
    MissingStdoutTrapPlan,
    MissingEmittedStdoutRequest,
    UnsupportedEmissionTarget(NativeStdoutEmissionUnsupported),
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
            Self::UnsupportedEmissionTarget(error) => write!(formatter, "{error}"),
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
        Self::main_from_generated_code(NativeGeneratedCode::from_raw_arm64(body))
    }

    fn main_from_generated_code(generated_code: NativeGeneratedCode<'_>) -> Self {
        let mut source = String::from(".text\n.globl _main\n.p2align 2\n_main:\n");
        push_byte_directives(&mut source, generated_code.body().as_slice());
        Self { source }
    }

    #[cfg(test)]
    fn stdout_main_from_raw_arm64_and_stdout(body: RawArm64Bytes<'_>, stdout_bytes: &[u8]) -> Self {
        Self::stdout_main_from_generated_code_and_stdout(
            NativeGeneratedCode::from_raw_arm64(body),
            NativeStdoutData::from_bytes(stdout_bytes),
        )
        .expect("arm64 apple macos stdout emission is supported")
    }

    fn stdout_main_from_generated_code_and_stdout(
        generated_code: NativeGeneratedCode<'_>,
        stdout: NativeStdoutData<'_>,
    ) -> Result<Self, NativeStdoutEmissionUnsupported> {
        Self::stdout_main_from_generated_code_and_stdout_for_target(
            generated_code,
            stdout,
            NativeArtifactTargetTriple::Arm64AppleMacos,
        )
    }

    fn stdout_main_from_generated_code_and_stdout_for_target(
        generated_code: NativeGeneratedCode<'_>,
        stdout: NativeStdoutData<'_>,
        target_triple: NativeArtifactTargetTriple,
    ) -> Result<Self, NativeStdoutEmissionUnsupported> {
        let stdout_emission = NativeStdoutEmissionStrategy::for_target(target_triple)?;
        let mut source = String::from(".text\n.globl _main\n.p2align 2\n_main:\n");
        stdout_emission.push_prologue(&mut source, stdout);
        push_byte_directives(&mut source, generated_code.body().as_slice());
        source.push_str(".section __TEXT,__const\n.p2align 2\nL_stdout_text:\n");
        push_byte_directives(&mut source, stdout.as_bytes());
        Ok(Self { source })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.source
    }

    #[cfg(test)]
    fn into_string(self) -> String {
        self.source
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeGeneratedCode<'a> {
    body: RawArm64Bytes<'a>,
}

impl<'a> NativeGeneratedCode<'a> {
    fn from_raw_arm64(body: RawArm64Bytes<'a>) -> Self {
        Self { body }
    }

    const fn body(self) -> RawArm64Bytes<'a> {
        self.body
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeStdoutData<'a> {
    bytes: &'a [u8],
}

impl<'a> NativeStdoutData<'a> {
    fn from_text(text: &'a str) -> Self {
        Self::from_bytes(text.as_bytes())
    }

    const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    const fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkedNativeExecutable {
    output_path: NativeArtifactOutputPath,
    metadata: NativeArtifactMetadata,
}

impl LinkedNativeExecutable {
    #[cfg(test)]
    fn from_existing_path(path: PathBuf) -> Self {
        Self::from_existing_output_path_with_metadata(
            NativeArtifactOutputPath::from_path(path.as_path()),
            NativeArtifactHelperRequirements::none(),
            None,
        )
    }

    fn from_existing_output_path_with_metadata(
        output_path: NativeArtifactOutputPath,
        helper_requirements: NativeArtifactHelperRequirements,
        source_image: Option<NativeSourceImageMetadata>,
    ) -> Self {
        let metadata = NativeArtifactMetadata::linked_executable(
            output_path.clone(),
            helper_requirements,
            source_image,
        );
        Self {
            output_path,
            metadata,
        }
    }

    pub(crate) fn path(&self) -> &Path {
        self.output_path.as_path()
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
    #[serde(skip_serializing_if = "Option::is_none")]
    source_image: Option<NativeSourceImageMetadata>,
}

impl NativeArtifactMetadata {
    fn linked_executable(
        output_path: NativeArtifactOutputPath,
        helper_requirements: NativeArtifactHelperRequirements,
        source_image: Option<NativeSourceImageMetadata>,
    ) -> Self {
        Self {
            artifact_kind: NativeArtifactKind::LinkedExecutable,
            target_triple: NativeArtifactTargetTriple::Arm64AppleMacos,
            toolchain: NativeArtifactToolchain::Clang,
            output_path,
            helper_requirements: helper_requirements.into_values(),
            source_image,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum NativeSourceImageMetadata {
    MachOExecutable {
        entry_point: MachOEntryPointCommandMetadata,
        segment: MachOSegmentCommandHeaderMetadata,
    },
}

impl NativeSourceImageMetadata {
    pub(crate) fn from_mach_o_conversion(
        conversion: &MachOExecutableImageConversion,
    ) -> Result<Self, NativeSourceImageMetadataError> {
        let entry_point = conversion.entry_point().ok_or_else(|| {
            NativeSourceImageMetadataError::NotConvertible {
                blocker: conversion.blocker(),
            }
        })?;
        let segment =
            conversion
                .segment()
                .ok_or_else(|| NativeSourceImageMetadataError::NotConvertible {
                    blocker: conversion.blocker(),
                })?;

        Ok(Self::MachOExecutable {
            entry_point: entry_point.metadata(),
            segment: segment.header().clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeSourceImageMetadataError {
    NotConvertible {
        blocker: Option<MachOExecutableImageConversionBlocker>,
    },
}

impl fmt::Display for NativeSourceImageMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConvertible { blocker } => {
                write!(
                    formatter,
                    "Mach-O executable image metadata is not convertible: {blocker:?}"
                )
            }
        }
    }
}

impl Error for NativeSourceImageMetadataError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeArtifactKind {
    LinkedExecutable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum NativeArtifactTargetTriple {
    #[serde(rename = "arm64-apple-macos")]
    Arm64AppleMacos,
    #[serde(rename = "aarch64-unknown-linux-gnu")]
    Aarch64UnknownLinuxGnu,
    #[serde(rename = "aarch64-pc-windows-msvc")]
    Aarch64PcWindowsMsvc,
}

impl NativeArtifactTargetTriple {
    fn unsupported_stdout_emission_targets() -> [Self; 2] {
        [Self::Aarch64UnknownLinuxGnu, Self::Aarch64PcWindowsMsvc]
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Arm64AppleMacos => "arm64-apple-macos",
            Self::Aarch64UnknownLinuxGnu => "aarch64-unknown-linux-gnu",
            Self::Aarch64PcWindowsMsvc => "aarch64-pc-windows-msvc",
        }
    }

    const fn host_os_abi(self) -> NativeHostOsAbi {
        match self {
            Self::Arm64AppleMacos => NativeHostOsAbi::Macos,
            Self::Aarch64UnknownLinuxGnu => NativeHostOsAbi::Linux,
            Self::Aarch64PcWindowsMsvc => NativeHostOsAbi::Windows,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum NativeArtifactToolchain {
    #[serde(rename = "clang")]
    Clang,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeArtifactOutputPath(String);

impl NativeArtifactOutputPath {
    fn from_path(path: &Path) -> Self {
        Self(path.to_string_lossy().into_owned())
    }

    fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    fn to_path_buf(&self) -> PathBuf {
        self.as_path().to_path_buf()
    }
}

impl Serialize for NativeArtifactOutputPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeHostOsAbi {
    Macos,
    Linux,
    Windows,
}

impl NativeHostOsAbi {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeStdoutEmissionUnsupported {
    helper: NativeArtifactHelperRequirement,
    target_triple: NativeArtifactTargetTriple,
    host_os_abi: NativeHostOsAbi,
}

impl NativeStdoutEmissionUnsupported {
    const fn for_target(target_triple: NativeArtifactTargetTriple) -> Self {
        Self {
            helper: NativeArtifactHelperRequirement::WriteStdout,
            target_triple,
            host_os_abi: target_triple.host_os_abi(),
        }
    }

    #[cfg(test)]
    const fn helper(self) -> NativeArtifactHelperRequirement {
        self.helper
    }

    #[cfg(test)]
    const fn target_triple(self) -> NativeArtifactTargetTriple {
        self.target_triple
    }

    #[cfg(test)]
    const fn host_os_abi(self) -> NativeHostOsAbi {
        self.host_os_abi
    }
}

impl fmt::Display for NativeStdoutEmissionUnsupported {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "stdout helper emission is unsupported for target {} with {} OS ABI",
            self.target_triple.as_str(),
            self.host_os_abi.as_str()
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeStdoutEmissionStrategy {
    MacosArm64Write,
}

impl NativeStdoutEmissionStrategy {
    fn for_target(
        target_triple: NativeArtifactTargetTriple,
    ) -> Result<Self, NativeStdoutEmissionUnsupported> {
        let unsupported_targets = NativeArtifactTargetTriple::unsupported_stdout_emission_targets();

        match target_triple {
            NativeArtifactTargetTriple::Arm64AppleMacos => Ok(Self::MacosArm64Write),
            target_triple if unsupported_targets.contains(&target_triple) => {
                Err(NativeStdoutEmissionUnsupported::for_target(target_triple))
            }
            target_triple => Err(NativeStdoutEmissionUnsupported::for_target(target_triple)),
        }
    }

    fn push_prologue(self, source: &mut String, stdout: NativeStdoutData<'_>) {
        match self {
            Self::MacosArm64Write => push_macos_arm64_write_stdout_prologue(source, stdout),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeToolchainCommand {
    program: &'static str,
    args: Vec<String>,
}

impl NativeToolchainCommand {
    fn clang_link(assembly_path: &Path, output_path: &NativeArtifactOutputPath) -> Self {
        Self {
            program: "clang",
            args: vec![
                assembly_path.to_string_lossy().into_owned(),
                String::from("-o"),
                output_path.as_path().to_string_lossy().into_owned(),
            ],
        }
    }

    fn to_command(&self) -> Command {
        let mut command = Command::new(self.program);
        command.args(&self.args);
        command
    }

    #[cfg(test)]
    const fn program(&self) -> &'static str {
        self.program
    }

    #[cfg(test)]
    fn args(&self) -> &[String] {
        &self.args
    }
}

pub(crate) fn native_artifact_metadata_to_json(
    metadata: &NativeArtifactMetadata,
) -> Result<String, JsonError> {
    serde_json::to_string(metadata).map_err(JsonError::new)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeArtifactPackageRequest {
    source: NativeAssemblySource,
    output_path: NativeArtifactOutputPath,
    helper_requirements: NativeArtifactHelperRequirements,
    source_image: Option<NativeSourceImageMetadata>,
}

impl NativeArtifactPackageRequest {
    fn linked_executable(
        source: NativeAssemblySource,
        output_path: NativeArtifactOutputPath,
        helper_requirements: NativeArtifactHelperRequirements,
    ) -> Self {
        Self {
            source,
            output_path,
            helper_requirements,
            source_image: None,
        }
    }

    fn linked_executable_with_source_image(
        source: NativeAssemblySource,
        output_path: NativeArtifactOutputPath,
        helper_requirements: NativeArtifactHelperRequirements,
        source_image: NativeSourceImageMetadata,
    ) -> Self {
        Self {
            source,
            output_path,
            helper_requirements,
            source_image: Some(source_image),
        }
    }

    fn source(&self) -> &NativeAssemblySource {
        &self.source
    }

    fn output_path(&self) -> &NativeArtifactOutputPath {
        &self.output_path
    }

    fn into_linked_executable(self) -> LinkedNativeExecutable {
        LinkedNativeExecutable::from_existing_output_path_with_metadata(
            self.output_path,
            self.helper_requirements,
            self.source_image,
        )
    }
}

pub(crate) trait NativeArtifactPackager {
    fn package(
        &self,
        request: NativeArtifactPackageRequest,
    ) -> Result<LinkedNativeExecutable, NativeArtifactError>;
}

pub(crate) fn package_native_artifact(
    packager: &impl NativeArtifactPackager,
    request: NativeArtifactPackageRequest,
) -> Result<LinkedNativeExecutable, NativeArtifactError> {
    packager.package(request)
}

struct ClangNativeArtifactPackager;

impl NativeArtifactPackager for ClangNativeArtifactPackager {
    fn package(
        &self,
        request: NativeArtifactPackageRequest,
    ) -> Result<LinkedNativeExecutable, NativeArtifactError> {
        let assembly_path = temporary_assembly_path()?;
        fs::write(&assembly_path, request.source().as_str()).map_err(|source| {
            NativeArtifactError::WriteAssembly {
                path: assembly_path.clone(),
                source,
            }
        })?;

        let toolchain_command =
            NativeToolchainCommand::clang_link(&assembly_path, request.output_path());
        let output = toolchain_command
            .to_command()
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
        if !request.output_path().as_path().exists() {
            return Err(NativeArtifactError::MissingLinkedExecutable {
                path: request.output_path().to_path_buf(),
            });
        }

        Ok(request.into_linked_executable())
    }
}

fn native_artifact_package_request(
    source: NativeAssemblySource,
    output_path: NativeArtifactOutputPath,
    helper_requirements: NativeArtifactHelperRequirements,
    source_image: Option<NativeSourceImageMetadata>,
) -> NativeArtifactPackageRequest {
    match source_image {
        Some(source_image) => NativeArtifactPackageRequest::linked_executable_with_source_image(
            source,
            output_path,
            helper_requirements,
            source_image,
        ),
        None => NativeArtifactPackageRequest::linked_executable(
            source,
            output_path,
            helper_requirements,
        ),
    }
}

pub(crate) fn link_arm64_main_executable(
    body: FunctionArm64Bytes<'_>,
    output_path: &Path,
) -> Result<LinkedNativeExecutable, NativeArtifactError> {
    link_arm64_main_executable_with_source_metadata(body, output_path, None)
}

pub(crate) fn link_arm64_main_executable_with_source_metadata(
    body: FunctionArm64Bytes<'_>,
    output_path: &Path,
    source_image: Option<NativeSourceImageMetadata>,
) -> Result<LinkedNativeExecutable, NativeArtifactError> {
    ensure_supported_host()?;

    let output_path = NativeArtifactOutputPath::from_path(output_path);
    let source = arm64_main_assembly_source(body);
    let request = native_artifact_package_request(
        source,
        output_path,
        NativeArtifactHelperRequirements::none(),
        source_image,
    );
    package_native_artifact(&ClangNativeArtifactPackager, request)
}

pub(crate) fn link_arm64_stdout_main_executable(
    body: FunctionArm64Bytes<'_>,
    host_trap_plan: &TestCaseHostTrapPlan,
    stdout_request: FunctionStdoutHostTrapRequest,
    output_path: &Path,
) -> Result<LinkedNativeExecutable, NativeArtifactError> {
    link_arm64_stdout_main_executable_with_source_metadata(
        body,
        host_trap_plan,
        stdout_request,
        output_path,
        None,
    )
}

pub(crate) fn link_arm64_stdout_main_executable_with_source_metadata(
    body: FunctionArm64Bytes<'_>,
    host_trap_plan: &TestCaseHostTrapPlan,
    stdout_request: FunctionStdoutHostTrapRequest,
    output_path: &Path,
    source_image: Option<NativeSourceImageMetadata>,
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

    let output_path = NativeArtifactOutputPath::from_path(output_path);
    let source = arm64_stdout_main_assembly_source(body, stdout.text())?;
    let request = native_artifact_package_request(
        source,
        output_path,
        NativeArtifactHelperRequirements::write_stdout(),
        source_image,
    );
    package_native_artifact(&ClangNativeArtifactPackager, request)
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
) -> Result<NativeAssemblySource, NativeArtifactError> {
    NativeAssemblySource::stdout_main_from_generated_code_and_stdout(
        NativeGeneratedCode::from_raw_arm64(RawArm64Bytes::from_function(body)),
        NativeStdoutData::from_text(stdout_text),
    )
    .map_err(|error| {
        NativeArtifactError::StdoutMainUnsupported(
            NativeStdoutMainUnsupported::UnsupportedEmissionTarget(error),
        )
    })
}

#[cfg(test)]
fn arm64_stdout_main_assembly_source_from_parts(body_bytes: &[u8], stdout_bytes: &[u8]) -> String {
    NativeAssemblySource::stdout_main_from_raw_arm64_and_stdout(
        RawArm64Bytes::from_trusted_bytes(body_bytes),
        stdout_bytes,
    )
    .into_string()
}

fn push_macos_arm64_write_stdout_prologue(source: &mut String, stdout: NativeStdoutData<'_>) {
    source.push_str("stp x29, x30, [sp, #-16]!\n");
    source.push_str("mov x29, sp\n");
    source.push_str("mov x0, #1\n");
    source.push_str("adrp x1, L_stdout_text@PAGE\n");
    source.push_str("add x1, x1, L_stdout_text@PAGEOFF\n");
    source.push_str(&format!("mov x2, #{}\n", stdout.as_bytes().len()));
    source.push_str("bl _write\n");
    source.push_str("ldp x29, x30, [sp], #16\n");
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

    use bara_oracle::{probe_public_binary_format, BinaryFileBytes, BinaryInput, FailureKind};

    use super::{
        arm64_main_assembly_source_from_bytes, arm64_stdout_main_assembly_source_from_parts,
        native_artifact_metadata_to_json, LinkedNativeExecutable, NativeArtifactError,
        NativeArtifactHelperRequirement, NativeArtifactOutputPath, NativeArtifactTargetTriple,
        NativeAssemblySource, NativeGeneratedCode, NativeHostOsAbi, NativeSourceImageMetadata,
        NativeStdoutData, NativeStdoutEmissionStrategy, NativeStdoutMainUnsupported,
        NativeToolchainCommand, RawArm64Bytes,
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
    fn native_artifact_metadata_serializes_mach_o_source_image_metadata() {
        let output_path = NativeArtifactOutputPath::from_path(Path::new("/tmp/mach_o_return_42"));
        let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(
            include_bytes!("../../../tests/binaries/mach_o_return_42.bin").to_vec(),
        ));
        let report = probe_public_binary_format(&input).expect("Mach-O fixture probe succeeds");
        let source_image = NativeSourceImageMetadata::from_mach_o_conversion(
            report
                .metadata()
                .mach_o_metadata()
                .executable_image_conversion(),
        )
        .expect("test Mach-O conversion is convertible");
        let executable = LinkedNativeExecutable::from_existing_output_path_with_metadata(
            output_path,
            super::NativeArtifactHelperRequirements::none(),
            Some(source_image),
        );

        assert_eq!(
            native_artifact_metadata_to_json(executable.metadata())
                .expect("metadata serializes as json"),
            "{\"artifact_kind\":\"linked_executable\",\"target_triple\":\"arm64-apple-macos\",\"toolchain\":\"clang\",\"output_path\":\"/tmp/mach_o_return_42\",\"helper_requirements\":[],\"source_image\":{\"kind\":\"mach_o_executable\",\"entry_point\":{\"entryoff\":130,\"stacksize\":8192},\"segment\":{\"name\":\"__TEXT\",\"vmaddr\":4294967296,\"fileoff\":128,\"filesize\":8}}}"
        );
    }

    #[test]
    fn unsupported_host_error_serializes_as_stable_json_message() {
        let error = NativeArtifactError::UnsupportedHost {
            os: "linux",
            arch: "x86_64",
        };

        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert_eq!(
            error.to_string(),
            "{\"status\":\"unsupported_host\",\"failure_kind\":\"emit_error\",\"artifact_kind\":\"linked_executable\",\"target_triple\":\"arm64-apple-macos\",\"host\":{\"os\":\"linux\",\"arch\":\"x86_64\"}}"
        );
    }

    #[test]
    fn native_artifact_request_types_separate_code_stdout_command_and_output_path() {
        let output_path = NativeArtifactOutputPath::from_path(Path::new("/tmp/hello"));
        let raw =
            RawArm64Bytes::from_trusted_bytes(&[0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]);
        let generated_code = NativeGeneratedCode::from_raw_arm64(raw);
        let stdout = NativeStdoutData::from_text("hello\n");
        let source = NativeAssemblySource::stdout_main_from_generated_code_and_stdout(
            generated_code,
            stdout,
        )
        .expect("arm64 apple macos stdout emission is supported");
        let command = NativeToolchainCommand::clang_link(Path::new("/tmp/hello.s"), &output_path);

        assert_eq!(output_path.as_path(), Path::new("/tmp/hello"));
        assert!(source.as_str().contains("mov x2, #6\n"));
        assert_eq!(command.program(), "clang");
        assert_eq!(
            command.args(),
            [
                String::from("/tmp/hello.s"),
                String::from("-o"),
                String::from("/tmp/hello")
            ]
        );
    }

    #[test]
    fn native_artifact_packaging_boundary_accepts_different_packagers() {
        struct RecordingPackager;

        impl super::NativeArtifactPackager for RecordingPackager {
            fn package(
                &self,
                request: super::NativeArtifactPackageRequest,
            ) -> Result<LinkedNativeExecutable, NativeArtifactError> {
                assert!(request.source().as_str().starts_with(".text\n"));
                Ok(request.into_linked_executable())
            }
        }

        let raw =
            RawArm64Bytes::from_trusted_bytes(&[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]);
        let source = NativeAssemblySource::main_from_generated_code(
            NativeGeneratedCode::from_raw_arm64(raw),
        );
        let request = super::NativeArtifactPackageRequest::linked_executable(
            source,
            NativeArtifactOutputPath::from_path(Path::new("/tmp/return_42")),
            super::NativeArtifactHelperRequirements::none(),
        );

        let executable = super::package_native_artifact(&RecordingPackager, request)
            .expect("fake packager returns a linked executable");

        assert_eq!(executable.path(), Path::new("/tmp/return_42"));
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

    #[test]
    fn stdout_helper_emission_strategy_is_selected_by_target_os_abi() {
        assert_eq!(
            NativeStdoutEmissionStrategy::for_target(NativeArtifactTargetTriple::Arm64AppleMacos),
            Ok(NativeStdoutEmissionStrategy::MacosArm64Write)
        );

        let linux = NativeStdoutEmissionStrategy::for_target(
            NativeArtifactTargetTriple::Aarch64UnknownLinuxGnu,
        )
        .expect_err("linux stdout helper emission is not implemented yet");
        assert_eq!(linux.helper(), NativeArtifactHelperRequirement::WriteStdout);
        assert_eq!(
            linux.target_triple(),
            NativeArtifactTargetTriple::Aarch64UnknownLinuxGnu
        );
        assert_eq!(linux.host_os_abi(), NativeHostOsAbi::Linux);

        let windows = NativeStdoutEmissionStrategy::for_target(
            NativeArtifactTargetTriple::Aarch64PcWindowsMsvc,
        )
        .expect_err("windows stdout helper emission is not implemented yet");
        assert_eq!(
            windows.helper(),
            NativeArtifactHelperRequirement::WriteStdout
        );
        assert_eq!(
            windows.target_triple(),
            NativeArtifactTargetTriple::Aarch64PcWindowsMsvc
        );
        assert_eq!(windows.host_os_abi(), NativeHostOsAbi::Windows);
    }
}
