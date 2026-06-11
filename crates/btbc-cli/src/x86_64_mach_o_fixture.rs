use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{
    observed_result_from_json, CaseId, CaseIdError, FailureKind, ObservedResult, TestCase,
    TestCaseAbi,
};
use serde::{Serialize, Serializer};

const B8_GUI_HELLO_WORLD_CASE_ID: &str = "b8_gui_hello_world";
const B8_GUI_HELLO_WORLD_SOURCE: &str = include_str!("../../../tests/sources/b8_gui_hello_world.m");
const B8_GUI_HELLO_WORLD_STDOUT: &str =
    "{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}\n";
const B8_GUI_HELLO_WORLD_TITLE: &str = "Bara GUI Hello World";
const B8_GUI_HELLO_WORLD_TEXT: &str = "hello world";

#[derive(Debug)]
pub(crate) enum X8664MachOFixtureError {
    UnsupportedHost {
        os: &'static str,
        arch: &'static str,
    },
    UnsupportedRosettaHost {
        os: &'static str,
        arch: &'static str,
    },
    UnsupportedAbi {
        case_id: CaseId,
    },
    UnsupportedHostTrapPlan {
        case_id: CaseId,
    },
    InvalidBuiltInCaseId {
        case_id: &'static str,
        source: CaseIdError,
    },
    TempPath {
        source: std::time::SystemTimeError,
    },
    WriteSource {
        path: PathBuf,
        source: io::Error,
    },
    ClangSpawn {
        source: io::Error,
    },
    ClangFailed {
        status: String,
        stderr: String,
    },
    MissingOutput {
        path: PathBuf,
    },
    RunnerSpawn {
        path: PathBuf,
        source: io::Error,
    },
    RunnerFailed {
        path: PathBuf,
        status: String,
        stdout: String,
        stderr: String,
    },
    InvalidRunnerStdout {
        path: PathBuf,
        stdout: String,
        source: String,
    },
    InvalidGuiLaunchStdout {
        path: PathBuf,
        stdout: String,
    },
}

impl X8664MachOFixtureError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::UnsupportedAbi { .. } | Self::UnsupportedHostTrapPlan { .. } => {
                FailureKind::InvalidTestCase
            }
            Self::UnsupportedHost { .. }
            | Self::InvalidBuiltInCaseId { .. }
            | Self::TempPath { .. }
            | Self::WriteSource { .. }
            | Self::ClangSpawn { .. }
            | Self::ClangFailed { .. }
            | Self::MissingOutput { .. } => FailureKind::EmitError,
            Self::UnsupportedRosettaHost { .. }
            | Self::RunnerSpawn { .. }
            | Self::RunnerFailed { .. }
            | Self::InvalidRunnerStdout { .. }
            | Self::InvalidGuiLaunchStdout { .. } => FailureKind::RunError,
        }
    }
}

impl fmt::Display for X8664MachOFixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedHost { os, arch } => write!(
                formatter,
                "x86_64 Mach-O artifact generation is unsupported on host os={os} arch={arch}"
            ),
            Self::UnsupportedRosettaHost { os, arch } => write!(
                formatter,
                "x86_64 Rosetta oracle execution is unsupported on host os={os} arch={arch}"
            ),
            Self::UnsupportedAbi { case_id } => write!(
                formatter,
                "x86_64 Mach-O artifact generation supports only no-args u64 testcases: {}",
                case_id.as_str()
            ),
            Self::UnsupportedHostTrapPlan { case_id } => write!(
                formatter,
                "x86_64 Mach-O artifact generation does not support host trap testcases yet: {}",
                case_id.as_str()
            ),
            Self::InvalidBuiltInCaseId { case_id, source } => write!(
                formatter,
                "invalid built-in x86_64 GUI fixture case id {case_id:?}: {source:?}"
            ),
            Self::TempPath { source } => {
                write!(formatter, "failed to build temporary x86_64 path: {source}")
            }
            Self::WriteSource { path, source } => {
                write!(
                    formatter,
                    "failed to write temporary x86_64 source {}: {source}",
                    path.display()
                )
            }
            Self::ClangSpawn { source } => {
                write!(formatter, "failed to run clang for x86_64 Mach-O: {source}")
            }
            Self::ClangFailed { status, stderr } => {
                write!(
                    formatter,
                    "clang failed while building x86_64 Mach-O with {status}: {stderr}"
                )
            }
            Self::MissingOutput { path } => write!(
                formatter,
                "clang completed but x86_64 Mach-O output does not exist: {}",
                path.display()
            ),
            Self::RunnerSpawn { path, source } => write!(
                formatter,
                "failed to run x86_64 oracle runner {}: {source}",
                path.display()
            ),
            Self::RunnerFailed {
                path,
                status,
                stdout,
                stderr,
            } => write!(
                formatter,
                "x86_64 oracle runner {} failed with {status}: stdout={stdout:?} stderr={stderr:?}",
                path.display()
            ),
            Self::InvalidRunnerStdout {
                path,
                stdout,
                source,
            } => write!(
                formatter,
                "x86_64 oracle runner {} emitted invalid expected JSON: {source}; stdout={stdout:?}",
                path.display()
            ),
            Self::InvalidGuiLaunchStdout { path, stdout } => write!(
                formatter,
                "x86_64 GUI fixture {} emitted unexpected launch stdout: {stdout:?}",
                path.display()
            ),
        }
    }
}

impl Error for X8664MachOFixtureError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedX8664MachOFixture {
    metadata: X8664MachOFixtureMetadata,
}

impl GeneratedX8664MachOFixture {
    fn from_request(request: X8664MachOFixtureBuildRequest) -> Self {
        Self {
            metadata: X8664MachOFixtureMetadata::new(
                X8664MachOFixtureArtifactKind::MachO,
                request.case_id,
                request.output_path,
            ),
        }
    }

    pub(crate) const fn metadata(&self) -> &X8664MachOFixtureMetadata {
        &self.metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedX8664OracleRunner {
    metadata: X8664MachOFixtureMetadata,
}

impl GeneratedX8664OracleRunner {
    fn from_request(request: X8664OracleRunnerBuildRequest) -> Self {
        Self {
            metadata: X8664MachOFixtureMetadata::new(
                X8664MachOFixtureArtifactKind::OracleRunner,
                request.case_id,
                request.output_path,
            ),
        }
    }

    pub(crate) const fn metadata(&self) -> &X8664MachOFixtureMetadata {
        &self.metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedX8664GuiHelloWorldFixture {
    metadata: X8664MachOFixtureMetadata,
}

impl GeneratedX8664GuiHelloWorldFixture {
    fn from_request(request: X8664GuiHelloWorldBuildRequest) -> Self {
        Self {
            metadata: X8664MachOFixtureMetadata::new(
                request.run_mode.artifact_kind(),
                request.case_id,
                request.output_path,
            ),
        }
    }

    pub(crate) const fn metadata(&self) -> &X8664MachOFixtureMetadata {
        &self.metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct X8664GuiHelloWorldExpectedBundle {
    observed_result: ObservedResult,
    launch_metadata: X8664GuiHelloWorldLaunchMetadata,
}

impl X8664GuiHelloWorldExpectedBundle {
    fn new(
        observed_result: ObservedResult,
        launch_metadata: X8664GuiHelloWorldLaunchMetadata,
    ) -> Self {
        Self {
            observed_result,
            launch_metadata,
        }
    }

    pub(crate) const fn observed_result(&self) -> &ObservedResult {
        &self.observed_result
    }

    pub(crate) const fn launch_metadata(&self) -> &X8664GuiHelloWorldLaunchMetadata {
        &self.launch_metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct X8664GuiHelloWorldLaunchMetadata {
    schema: &'static str,
    case_id: CaseId,
    oracle: X8664GuiHelloWorldOracleKind,
    fixture: X8664GuiHelloWorldFixtureLaunchMetadata,
    observed_events: Vec<X8664GuiHelloWorldLaunchEvent>,
}

impl X8664GuiHelloWorldLaunchMetadata {
    fn expected() -> Result<Self, X8664MachOFixtureError> {
        Ok(Self {
            schema: "b8_gui_hello_world_launch_metadata_v0",
            case_id: b8_gui_hello_world_case_id()?,
            oracle: X8664GuiHelloWorldOracleKind::RosettaBlackBox,
            fixture: X8664GuiHelloWorldFixtureLaunchMetadata::new(),
            observed_events: vec![X8664GuiHelloWorldLaunchEvent::window_created()],
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664GuiHelloWorldOracleKind {
    #[serde(rename = "rosetta_black_box")]
    RosettaBlackBox,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct X8664GuiHelloWorldFixtureLaunchMetadata {
    kind: X8664GuiHelloWorldFixtureKind,
    source_isa: X8664GuiHelloWorldSourceIsa,
    binary_format: X8664GuiHelloWorldBinaryFormat,
    target_triple: X8664MachOFixtureTargetTriple,
    gui_framework: X8664GuiHelloWorldFramework,
}

impl X8664GuiHelloWorldFixtureLaunchMetadata {
    const fn new() -> Self {
        Self {
            kind: X8664GuiHelloWorldFixtureKind::SingleMachOExecutable,
            source_isa: X8664GuiHelloWorldSourceIsa::X8664,
            binary_format: X8664GuiHelloWorldBinaryFormat::MachO,
            target_triple: X8664MachOFixtureTargetTriple::X8664AppleMacos13,
            gui_framework: X8664GuiHelloWorldFramework::AppKit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664GuiHelloWorldFixtureKind {
    #[serde(rename = "single_mach_o_executable")]
    SingleMachOExecutable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664GuiHelloWorldSourceIsa {
    #[serde(rename = "x86_64")]
    X8664,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664GuiHelloWorldBinaryFormat {
    #[serde(rename = "mach_o")]
    MachO,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664GuiHelloWorldFramework {
    #[serde(rename = "appkit")]
    AppKit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct X8664GuiHelloWorldLaunchEvent {
    event: X8664GuiHelloWorldLaunchEventKind,
    title: &'static str,
    text: &'static str,
}

impl X8664GuiHelloWorldLaunchEvent {
    const fn window_created() -> Self {
        Self {
            event: X8664GuiHelloWorldLaunchEventKind::GuiWindowCreated,
            title: B8_GUI_HELLO_WORLD_TITLE,
            text: B8_GUI_HELLO_WORLD_TEXT,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664GuiHelloWorldLaunchEventKind {
    #[serde(rename = "gui_window_created")]
    GuiWindowCreated,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct X8664MachOFixtureMetadata {
    artifact_kind: X8664MachOFixtureArtifactKind,
    case_id: CaseId,
    target_triple: X8664MachOFixtureTargetTriple,
    toolchain: X8664MachOFixtureToolchain,
    output_path: X8664MachOFixtureOutputPath,
}

impl X8664MachOFixtureMetadata {
    fn new(
        artifact_kind: X8664MachOFixtureArtifactKind,
        case_id: CaseId,
        output_path: X8664MachOFixtureOutputPath,
    ) -> Self {
        Self {
            artifact_kind,
            case_id,
            target_triple: X8664MachOFixtureTargetTriple::X8664AppleMacos13,
            toolchain: X8664MachOFixtureToolchain::Clang,
            output_path,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum X8664MachOFixtureArtifactKind {
    #[serde(rename = "mach_o_executable")]
    MachO,
    #[serde(rename = "oracle_runner_executable")]
    OracleRunner,
    #[serde(rename = "gui_hello_world_mach_o_executable")]
    GuiHelloWorldMachO,
    #[serde(rename = "gui_hello_world_manual_visible_mach_o_executable")]
    GuiHelloWorldManualVisibleMachO,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664MachOFixtureTargetTriple {
    #[serde(rename = "x86_64-apple-macos13")]
    X8664AppleMacos13,
}

impl X8664MachOFixtureTargetTriple {
    const fn as_str(self) -> &'static str {
        match self {
            Self::X8664AppleMacos13 => "x86_64-apple-macos13",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum X8664MachOFixtureToolchain {
    #[serde(rename = "clang")]
    Clang,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X8664MachOFixtureOutputPath(String);

impl X8664MachOFixtureOutputPath {
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

impl Serialize for X8664MachOFixtureOutputPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X8664MachOAssemblySource {
    source: String,
}

impl X8664MachOAssemblySource {
    fn from_test_case(test_case: &TestCase) -> Result<Self, X8664MachOFixtureError> {
        ensure_initial_no_args_oracle_scope(test_case)?;

        let mut source = String::from(".text\n.globl _main\n.p2align 4\n_main:\n");
        push_byte_directives(&mut source, test_case.x86_bytes().bytes());
        Ok(Self { source })
    }

    fn as_str(&self) -> &str {
        &self.source
    }

    #[cfg(test)]
    fn into_string(self) -> String {
        self.source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X8664OracleRunnerSource {
    source: String,
}

impl X8664OracleRunnerSource {
    fn from_test_case(test_case: &TestCase) -> Result<Self, X8664MachOFixtureError> {
        ensure_initial_no_args_oracle_scope(test_case)?;

        let mut source = String::new();
        source.push_str("#include <stdint.h>\n");
        source.push_str("#include <stdio.h>\n");
        source.push_str("#include <string.h>\n");
        source.push_str("#include <sys/mman.h>\n\n");
        source.push_str("typedef uint64_t (*bara_no_args_u64_fn)(void);\n\n");
        source.push_str("static const unsigned char BARA_TESTCASE_BYTES[] = {\n");
        push_c_byte_initializer(&mut source, test_case.x86_bytes().bytes());
        source.push_str("};\n");
        source.push_str("static const char BARA_CASE_ID_JSON[] = ");
        push_c_string_literal(
            &mut source,
            &json_string_literal(test_case.case_id().as_str()),
        );
        source.push_str(";\n\n");
        source.push_str("int main(void) {\n");
        source.push_str("    void *code = mmap(0, sizeof(BARA_TESTCASE_BYTES), PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);\n");
        source.push_str("    if (code == MAP_FAILED) {\n");
        source.push_str("        fputs(\"mmap failed\\n\", stderr);\n");
        source.push_str("        return 1;\n");
        source.push_str("    }\n");
        source.push_str("    memcpy(code, BARA_TESTCASE_BYTES, sizeof(BARA_TESTCASE_BYTES));\n");
        source.push_str(
            "    if (mprotect(code, sizeof(BARA_TESTCASE_BYTES), PROT_READ | PROT_EXEC) != 0) {\n",
        );
        source.push_str("        fputs(\"mprotect failed\\n\", stderr);\n");
        source.push_str("        return 1;\n");
        source.push_str("    }\n");
        source.push_str("    uint64_t return_value = ((bara_no_args_u64_fn)code)();\n");
        source.push_str("    (void)munmap(code, sizeof(BARA_TESTCASE_BYTES));\n");
        source.push_str("    printf(\"{\\\"case_id\\\":%s,\\\"exit_status\\\":0,\\\"return_value\\\":%llu,\\\"stdout\\\":\\\"\\\",\\\"stderr\\\":\\\"\\\"}\\n\", BARA_CASE_ID_JSON, (unsigned long long)return_value);\n");
        source.push_str("    return 0;\n");
        source.push_str("}\n");
        Ok(Self { source })
    }

    fn as_str(&self) -> &str {
        &self.source
    }

    #[cfg(test)]
    fn into_string(self) -> String {
        self.source
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct X8664GuiHelloWorldSource {
    source: &'static str,
}

impl X8664GuiHelloWorldSource {
    const fn new() -> Self {
        Self {
            source: B8_GUI_HELLO_WORLD_SOURCE,
        }
    }

    const fn as_str(self) -> &'static str {
        self.source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct X8664MachOFixtureBuildRequest {
    case_id: CaseId,
    source: X8664MachOAssemblySource,
    output_path: X8664MachOFixtureOutputPath,
}

impl X8664MachOFixtureBuildRequest {
    fn from_test_case(
        test_case: &TestCase,
        output_path: &Path,
    ) -> Result<Self, X8664MachOFixtureError> {
        Ok(Self {
            case_id: test_case.case_id().clone(),
            source: X8664MachOAssemblySource::from_test_case(test_case)?,
            output_path: X8664MachOFixtureOutputPath::from_path(output_path),
        })
    }

    fn source(&self) -> &X8664MachOAssemblySource {
        &self.source
    }

    fn output_path(&self) -> &X8664MachOFixtureOutputPath {
        &self.output_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct X8664OracleRunnerBuildRequest {
    case_id: CaseId,
    source: X8664OracleRunnerSource,
    output_path: X8664MachOFixtureOutputPath,
}

impl X8664OracleRunnerBuildRequest {
    fn from_test_case(
        test_case: &TestCase,
        output_path: &Path,
    ) -> Result<Self, X8664MachOFixtureError> {
        Ok(Self {
            case_id: test_case.case_id().clone(),
            source: X8664OracleRunnerSource::from_test_case(test_case)?,
            output_path: X8664MachOFixtureOutputPath::from_path(output_path),
        })
    }

    fn source(&self) -> &X8664OracleRunnerSource {
        &self.source
    }

    fn output_path(&self) -> &X8664MachOFixtureOutputPath {
        &self.output_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct X8664GuiHelloWorldBuildRequest {
    case_id: CaseId,
    source: X8664GuiHelloWorldSource,
    run_mode: X8664GuiHelloWorldRunMode,
    output_path: X8664MachOFixtureOutputPath,
}

impl X8664GuiHelloWorldBuildRequest {
    fn new(output_path: &Path) -> Result<Self, X8664MachOFixtureError> {
        Self::with_run_mode(output_path, X8664GuiHelloWorldRunMode::AutomatedOracle)
    }

    fn manual_visible(output_path: &Path) -> Result<Self, X8664MachOFixtureError> {
        Self::with_run_mode(output_path, X8664GuiHelloWorldRunMode::ManualVisible)
    }

    fn with_run_mode(
        output_path: &Path,
        run_mode: X8664GuiHelloWorldRunMode,
    ) -> Result<Self, X8664MachOFixtureError> {
        let case_id = b8_gui_hello_world_case_id()?;
        Ok(Self {
            case_id,
            source: X8664GuiHelloWorldSource::new(),
            run_mode,
            output_path: X8664MachOFixtureOutputPath::from_path(output_path),
        })
    }

    fn source(&self) -> X8664GuiHelloWorldSource {
        self.source
    }

    fn output_path(&self) -> &X8664MachOFixtureOutputPath {
        &self.output_path
    }

    const fn run_mode(&self) -> X8664GuiHelloWorldRunMode {
        self.run_mode
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum X8664GuiHelloWorldRunMode {
    AutomatedOracle,
    ManualVisible,
}

impl X8664GuiHelloWorldRunMode {
    const fn artifact_kind(self) -> X8664MachOFixtureArtifactKind {
        match self {
            Self::AutomatedOracle => X8664MachOFixtureArtifactKind::GuiHelloWorldMachO,
            Self::ManualVisible => X8664MachOFixtureArtifactKind::GuiHelloWorldManualVisibleMachO,
        }
    }

    fn clang_defines(self) -> Vec<String> {
        match self {
            Self::AutomatedOracle => Vec::new(),
            Self::ManualVisible => {
                vec![String::from("-DBARA_GUI_HELLO_WORLD_MANUAL_VISIBLE=1")]
            }
        }
    }
}

pub(crate) trait X8664MachOFixturePackager {
    fn package(
        &self,
        request: X8664MachOFixtureBuildRequest,
    ) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError>;
}

pub(crate) fn package_x86_64_mach_o_fixture(
    packager: &impl X8664MachOFixturePackager,
    request: X8664MachOFixtureBuildRequest,
) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError> {
    packager.package(request)
}

pub(crate) trait X8664OracleRunnerPackager {
    fn package(
        &self,
        request: X8664OracleRunnerBuildRequest,
    ) -> Result<GeneratedX8664OracleRunner, X8664MachOFixtureError>;
}

pub(crate) fn package_x86_64_oracle_runner(
    packager: &impl X8664OracleRunnerPackager,
    request: X8664OracleRunnerBuildRequest,
) -> Result<GeneratedX8664OracleRunner, X8664MachOFixtureError> {
    packager.package(request)
}

pub(crate) trait X8664GuiHelloWorldPackager {
    fn package(
        &self,
        request: X8664GuiHelloWorldBuildRequest,
    ) -> Result<GeneratedX8664GuiHelloWorldFixture, X8664MachOFixtureError>;
}

pub(crate) fn package_x86_64_gui_hello_world_fixture(
    packager: &impl X8664GuiHelloWorldPackager,
    request: X8664GuiHelloWorldBuildRequest,
) -> Result<GeneratedX8664GuiHelloWorldFixture, X8664MachOFixtureError> {
    packager.package(request)
}

struct ClangX8664MachOFixturePackager;

impl X8664MachOFixturePackager for ClangX8664MachOFixturePackager {
    fn package(
        &self,
        request: X8664MachOFixtureBuildRequest,
    ) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError> {
        let source_path = temporary_path("bara-x86-64-macho", "s")?;
        fs::write(&source_path, request.source().as_str()).map_err(|source| {
            X8664MachOFixtureError::WriteSource {
                path: source_path.clone(),
                source,
            }
        })?;

        let toolchain_command = X8664MachOFixtureToolchainCommand::clang_build(
            &source_path,
            X8664MachOFixtureSourceLanguage::Assembler,
            request.output_path(),
        );
        let output = toolchain_command
            .to_command()
            .output()
            .map_err(|source| X8664MachOFixtureError::ClangSpawn { source });

        let _ = fs::remove_file(&source_path);

        let output = output?;
        if !output.status.success() {
            return Err(X8664MachOFixtureError::ClangFailed {
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        if !request.output_path().as_path().exists() {
            return Err(X8664MachOFixtureError::MissingOutput {
                path: request.output_path().to_path_buf(),
            });
        }

        Ok(GeneratedX8664MachOFixture::from_request(request))
    }
}

struct ClangX8664OracleRunnerPackager;

impl X8664OracleRunnerPackager for ClangX8664OracleRunnerPackager {
    fn package(
        &self,
        request: X8664OracleRunnerBuildRequest,
    ) -> Result<GeneratedX8664OracleRunner, X8664MachOFixtureError> {
        let source_path = temporary_path("bara-x86-64-oracle-runner", "c")?;
        fs::write(&source_path, request.source().as_str()).map_err(|source| {
            X8664MachOFixtureError::WriteSource {
                path: source_path.clone(),
                source,
            }
        })?;

        let toolchain_command = X8664MachOFixtureToolchainCommand::clang_build(
            &source_path,
            X8664MachOFixtureSourceLanguage::C,
            request.output_path(),
        );
        let output = toolchain_command
            .to_command()
            .output()
            .map_err(|source| X8664MachOFixtureError::ClangSpawn { source });

        let _ = fs::remove_file(&source_path);

        let output = output?;
        if !output.status.success() {
            return Err(X8664MachOFixtureError::ClangFailed {
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        if !request.output_path().as_path().exists() {
            return Err(X8664MachOFixtureError::MissingOutput {
                path: request.output_path().to_path_buf(),
            });
        }

        Ok(GeneratedX8664OracleRunner::from_request(request))
    }
}

struct ClangX8664GuiHelloWorldPackager;

impl X8664GuiHelloWorldPackager for ClangX8664GuiHelloWorldPackager {
    fn package(
        &self,
        request: X8664GuiHelloWorldBuildRequest,
    ) -> Result<GeneratedX8664GuiHelloWorldFixture, X8664MachOFixtureError> {
        let source_path = temporary_path("bara-x86-64-gui-hello-world", "m")?;
        fs::write(&source_path, request.source().as_str()).map_err(|source| {
            X8664MachOFixtureError::WriteSource {
                path: source_path.clone(),
                source,
            }
        })?;

        let toolchain_command = X8664MachOFixtureToolchainCommand::clang_gui_appkit_build(
            &source_path,
            request.output_path(),
            request.run_mode(),
        );
        let output = toolchain_command
            .to_command()
            .output()
            .map_err(|source| X8664MachOFixtureError::ClangSpawn { source });

        let _ = fs::remove_file(&source_path);

        let output = output?;
        if !output.status.success() {
            return Err(X8664MachOFixtureError::ClangFailed {
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        if !request.output_path().as_path().exists() {
            return Err(X8664MachOFixtureError::MissingOutput {
                path: request.output_path().to_path_buf(),
            });
        }

        Ok(GeneratedX8664GuiHelloWorldFixture::from_request(request))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X8664MachOFixtureToolchainCommand {
    program: &'static str,
    args: Vec<String>,
}

impl X8664MachOFixtureToolchainCommand {
    fn clang_build(
        source_path: &Path,
        source_language: X8664MachOFixtureSourceLanguage,
        output_path: &X8664MachOFixtureOutputPath,
    ) -> Self {
        Self {
            program: "clang",
            args: vec![
                String::from("-target"),
                X8664MachOFixtureTargetTriple::X8664AppleMacos13
                    .as_str()
                    .to_owned(),
                String::from("-x"),
                source_language.as_clang_arg().to_owned(),
                source_path.to_string_lossy().into_owned(),
                String::from("-o"),
                output_path.as_path().to_string_lossy().into_owned(),
            ],
        }
    }

    fn clang_gui_appkit_build(
        source_path: &Path,
        output_path: &X8664MachOFixtureOutputPath,
        run_mode: X8664GuiHelloWorldRunMode,
    ) -> Self {
        let mut args = vec![
            String::from("-target"),
            X8664MachOFixtureTargetTriple::X8664AppleMacos13
                .as_str()
                .to_owned(),
            String::from("-x"),
            X8664MachOFixtureSourceLanguage::ObjectiveC
                .as_clang_arg()
                .to_owned(),
        ];
        args.extend(run_mode.clang_defines());
        args.extend([
            source_path.to_string_lossy().into_owned(),
            String::from("-framework"),
            String::from("AppKit"),
            String::from("-o"),
            output_path.as_path().to_string_lossy().into_owned(),
        ]);

        Self {
            program: "clang",
            args,
        }
    }

    fn clang_host_gui_appkit_helper_build(
        source_path: &Path,
        output_path: &X8664MachOFixtureOutputPath,
        run_mode: X8664GuiHelloWorldRunMode,
    ) -> Self {
        let mut args = vec![
            String::from("-x"),
            X8664MachOFixtureSourceLanguage::ObjectiveC
                .as_clang_arg()
                .to_owned(),
        ];
        args.extend(run_mode.clang_defines());
        args.extend([
            source_path.to_string_lossy().into_owned(),
            String::from("-framework"),
            String::from("AppKit"),
            String::from("-o"),
            output_path.as_path().to_string_lossy().into_owned(),
        ]);

        Self {
            program: "clang",
            args,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum X8664MachOFixtureSourceLanguage {
    Assembler,
    C,
    ObjectiveC,
}

impl X8664MachOFixtureSourceLanguage {
    const fn as_clang_arg(self) -> &'static str {
        match self {
            Self::Assembler => "assembler",
            Self::C => "c",
            Self::ObjectiveC => "objective-c",
        }
    }
}

pub(crate) fn build_x86_64_mach_o_fixture(
    test_case: &TestCase,
    output_path: &Path,
) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError> {
    ensure_supported_host()?;

    let request = X8664MachOFixtureBuildRequest::from_test_case(test_case, output_path)?;
    package_x86_64_mach_o_fixture(&ClangX8664MachOFixturePackager, request)
}

pub(crate) fn build_x86_64_oracle_runner(
    test_case: &TestCase,
    output_path: &Path,
) -> Result<GeneratedX8664OracleRunner, X8664MachOFixtureError> {
    ensure_supported_host()?;

    let request = X8664OracleRunnerBuildRequest::from_test_case(test_case, output_path)?;
    package_x86_64_oracle_runner(&ClangX8664OracleRunnerPackager, request)
}

pub(crate) fn build_x86_64_gui_hello_world_fixture(
    output_path: &Path,
) -> Result<GeneratedX8664GuiHelloWorldFixture, X8664MachOFixtureError> {
    ensure_supported_host()?;

    let request = X8664GuiHelloWorldBuildRequest::new(output_path)?;
    package_x86_64_gui_hello_world_fixture(&ClangX8664GuiHelloWorldPackager, request)
}

pub(crate) fn build_x86_64_gui_hello_world_manual_visible_fixture(
    output_path: &Path,
) -> Result<GeneratedX8664GuiHelloWorldFixture, X8664MachOFixtureError> {
    ensure_supported_host()?;

    let request = X8664GuiHelloWorldBuildRequest::manual_visible(output_path)?;
    package_x86_64_gui_hello_world_fixture(&ClangX8664GuiHelloWorldPackager, request)
}

pub(crate) fn observe_x86_64_oracle_expected(
    test_case: &TestCase,
) -> Result<ObservedResult, X8664MachOFixtureError> {
    ensure_supported_rosetta_host()?;

    let runner_path = temporary_path("bara-x86-64-oracle-runner", "exe")?;
    if let Err(error) = build_x86_64_oracle_runner(test_case, &runner_path) {
        let _ = fs::remove_file(&runner_path);
        return Err(error);
    }

    let output =
        Command::new(&runner_path)
            .output()
            .map_err(|source| X8664MachOFixtureError::RunnerSpawn {
                path: runner_path.clone(),
                source,
            });
    let _ = fs::remove_file(&runner_path);

    RosettaOracleObservation::from_process_output(output?).into_expected_result(&runner_path)
}

pub(crate) fn observe_x86_64_gui_hello_world_expected(
) -> Result<X8664GuiHelloWorldExpectedBundle, X8664MachOFixtureError> {
    ensure_supported_rosetta_host()?;

    let runner_path = temporary_path("bara-x86-64-gui-hello-world", "exe")?;
    if let Err(error) = build_x86_64_gui_hello_world_fixture(&runner_path) {
        let _ = fs::remove_file(&runner_path);
        return Err(error);
    }

    let output =
        Command::new(&runner_path)
            .output()
            .map_err(|source| X8664MachOFixtureError::RunnerSpawn {
                path: runner_path.clone(),
                source,
            });
    let _ = fs::remove_file(&runner_path);

    RosettaGuiHelloWorldObservation::from_process_output(output?).into_expected_bundle(&runner_path)
}

pub(crate) fn observe_appkit_gui_hello_world_helper_actual(
) -> Result<ObservedResult, X8664MachOFixtureError> {
    observe_appkit_gui_hello_world_helper_actual_with_run_mode(
        X8664GuiHelloWorldRunMode::AutomatedOracle,
    )
}

pub(crate) fn observe_appkit_gui_hello_world_manual_visible_helper_actual(
) -> Result<ObservedResult, X8664MachOFixtureError> {
    observe_appkit_gui_hello_world_helper_actual_with_run_mode(
        X8664GuiHelloWorldRunMode::ManualVisible,
    )
}

fn observe_appkit_gui_hello_world_helper_actual_with_run_mode(
    run_mode: X8664GuiHelloWorldRunMode,
) -> Result<ObservedResult, X8664MachOFixtureError> {
    ensure_supported_host()?;

    let helper_path = temporary_path("bara-appkit-gui-hello-world-helper", "exe")?;
    if let Err(error) = build_appkit_gui_hello_world_helper(&helper_path, run_mode) {
        let _ = fs::remove_file(&helper_path);
        return Err(error);
    }

    let output =
        Command::new(&helper_path)
            .output()
            .map_err(|source| X8664MachOFixtureError::RunnerSpawn {
                path: helper_path.clone(),
                source,
            });
    let _ = fs::remove_file(&helper_path);

    AppKitGuiHelloWorldHelperObservation::from_process_output(output?)
        .into_observed_result(&helper_path)
}

fn build_appkit_gui_hello_world_helper(
    output_path: &Path,
    run_mode: X8664GuiHelloWorldRunMode,
) -> Result<(), X8664MachOFixtureError> {
    let source_path = temporary_path("bara-appkit-gui-hello-world-helper", "m")?;
    let source = X8664GuiHelloWorldSource::new();
    fs::write(&source_path, source.as_str()).map_err(|source| {
        X8664MachOFixtureError::WriteSource {
            path: source_path.clone(),
            source,
        }
    })?;

    let output_path = X8664MachOFixtureOutputPath::from_path(output_path);
    let toolchain_command = X8664MachOFixtureToolchainCommand::clang_host_gui_appkit_helper_build(
        &source_path,
        &output_path,
        run_mode,
    );
    let output = toolchain_command
        .to_command()
        .output()
        .map_err(|source| X8664MachOFixtureError::ClangSpawn { source });

    let _ = fs::remove_file(&source_path);

    let output = output?;
    if !output.status.success() {
        return Err(X8664MachOFixtureError::ClangFailed {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    if !output_path.as_path().exists() {
        return Err(X8664MachOFixtureError::MissingOutput {
            path: output_path.to_path_buf(),
        });
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RosettaOracleObservation {
    runner_succeeded: bool,
    runner_status: String,
    stdout: String,
    stderr: String,
}

impl RosettaOracleObservation {
    fn from_process_output(output: Output) -> Self {
        Self {
            runner_succeeded: output.status.success(),
            runner_status: output.status.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    #[cfg(test)]
    const fn from_parts(
        runner_succeeded: bool,
        runner_status: String,
        stdout: String,
        stderr: String,
    ) -> Self {
        Self {
            runner_succeeded,
            runner_status,
            stdout,
            stderr,
        }
    }

    fn into_expected_result(
        self,
        runner_path: &Path,
    ) -> Result<ObservedResult, X8664MachOFixtureError> {
        if !self.runner_succeeded {
            return Err(X8664MachOFixtureError::RunnerFailed {
                path: runner_path.to_path_buf(),
                status: self.runner_status,
                stdout: self.stdout,
                stderr: self.stderr,
            });
        }

        observed_result_from_json(&self.stdout).map_err(|source| {
            X8664MachOFixtureError::InvalidRunnerStdout {
                path: runner_path.to_path_buf(),
                stdout: self.stdout,
                source: source.to_string(),
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RosettaGuiHelloWorldObservation {
    runner_succeeded: bool,
    runner_status: String,
    runner_exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl RosettaGuiHelloWorldObservation {
    fn from_process_output(output: Output) -> Self {
        Self {
            runner_succeeded: output.status.success(),
            runner_status: output.status.to_string(),
            runner_exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    #[cfg(test)]
    const fn from_parts(
        runner_succeeded: bool,
        runner_status: String,
        runner_exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    ) -> Self {
        Self {
            runner_succeeded,
            runner_status,
            runner_exit_code,
            stdout,
            stderr,
        }
    }

    fn into_expected_bundle(
        self,
        runner_path: &Path,
    ) -> Result<X8664GuiHelloWorldExpectedBundle, X8664MachOFixtureError> {
        if !self.runner_succeeded {
            return Err(X8664MachOFixtureError::RunnerFailed {
                path: runner_path.to_path_buf(),
                status: self.runner_status,
                stdout: self.stdout,
                stderr: self.stderr,
            });
        }
        if self.stdout != B8_GUI_HELLO_WORLD_STDOUT {
            return Err(X8664MachOFixtureError::InvalidGuiLaunchStdout {
                path: runner_path.to_path_buf(),
                stdout: self.stdout,
            });
        }

        let observed_result = ObservedResult::new(
            b8_gui_hello_world_case_id()?,
            self.runner_exit_code.unwrap_or(0),
            0,
            self.stdout,
            self.stderr,
        );
        Ok(X8664GuiHelloWorldExpectedBundle::new(
            observed_result,
            X8664GuiHelloWorldLaunchMetadata::expected()?,
        ))
    }
}

struct AppKitGuiHelloWorldHelperObservation {
    helper_succeeded: bool,
    helper_status: String,
    helper_exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl AppKitGuiHelloWorldHelperObservation {
    fn from_process_output(output: Output) -> Self {
        Self {
            helper_succeeded: output.status.success(),
            helper_status: output.status.to_string(),
            helper_exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn into_observed_result(
        self,
        helper_path: &Path,
    ) -> Result<ObservedResult, X8664MachOFixtureError> {
        if !self.helper_succeeded {
            return Err(X8664MachOFixtureError::RunnerFailed {
                path: helper_path.to_path_buf(),
                status: self.helper_status,
                stdout: self.stdout,
                stderr: self.stderr,
            });
        }
        if self.stdout != B8_GUI_HELLO_WORLD_STDOUT {
            return Err(X8664MachOFixtureError::InvalidGuiLaunchStdout {
                path: helper_path.to_path_buf(),
                stdout: self.stdout,
            });
        }

        Ok(ObservedResult::new(
            b8_gui_hello_world_case_id()?,
            self.helper_exit_code.unwrap_or(0),
            0,
            self.stdout,
            self.stderr,
        ))
    }
}

fn ensure_supported_host() -> Result<(), X8664MachOFixtureError> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        Err(X8664MachOFixtureError::UnsupportedHost {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        })
    }
}

fn ensure_supported_rosetta_host() -> Result<(), X8664MachOFixtureError> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Ok(())
    } else {
        Err(X8664MachOFixtureError::UnsupportedRosettaHost {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        })
    }
}

pub(crate) fn b8_gui_hello_world_case_id() -> Result<CaseId, X8664MachOFixtureError> {
    CaseId::new(B8_GUI_HELLO_WORLD_CASE_ID).map_err(|source| {
        X8664MachOFixtureError::InvalidBuiltInCaseId {
            case_id: B8_GUI_HELLO_WORLD_CASE_ID,
            source,
        }
    })
}

fn temporary_path(prefix: &str, extension: &str) -> Result<PathBuf, X8664MachOFixtureError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| X8664MachOFixtureError::TempPath { source })?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("{prefix}-{nanos}.{extension}")))
}

fn ensure_initial_no_args_oracle_scope(test_case: &TestCase) -> Result<(), X8664MachOFixtureError> {
    if !matches!(test_case.abi(), TestCaseAbi::NoArgsU64) {
        return Err(X8664MachOFixtureError::UnsupportedAbi {
            case_id: test_case.case_id().clone(),
        });
    }
    if test_case.host_trap_plan().stdout_trap().is_some() {
        return Err(X8664MachOFixtureError::UnsupportedHostTrapPlan {
            case_id: test_case.case_id().clone(),
        });
    }

    Ok(())
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

fn push_c_byte_initializer(source: &mut String, bytes: &[u8]) {
    for chunk in bytes.chunks(12) {
        source.push_str("    ");
        for (index, byte) in chunk.iter().enumerate() {
            if index > 0 {
                source.push_str(", ");
            }
            source.push_str(&format!("0x{byte:02x}"));
        }
        source.push_str(",\n");
    }
}

fn json_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn push_c_string_literal(source: &mut String, value: &str) {
    source.push('"');
    for character in value.chars() {
        match character {
            '"' => source.push_str("\\\""),
            '\\' => source.push_str("\\\\"),
            '\n' => source.push_str("\\n"),
            '\r' => source.push_str("\\r"),
            '\t' => source.push_str("\\t"),
            character => source.push(character),
        }
    }
    source.push('"');
}

#[cfg(test)]
mod tests {
    use std::{
        fs, io,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bara_oracle::{test_case_from_json, CaseId, ObservedResult};

    use super::{
        json_string_literal, observe_x86_64_oracle_expected, package_x86_64_mach_o_fixture,
        package_x86_64_oracle_runner, GeneratedX8664MachOFixture, GeneratedX8664OracleRunner,
        RosettaGuiHelloWorldObservation, RosettaOracleObservation, X8664GuiHelloWorldRunMode,
        X8664GuiHelloWorldSource, X8664MachOAssemblySource, X8664MachOFixtureBuildRequest,
        X8664MachOFixtureError, X8664MachOFixtureOutputPath, X8664MachOFixturePackager,
        X8664MachOFixtureSourceLanguage, X8664MachOFixtureToolchainCommand,
        X8664OracleRunnerBuildRequest, X8664OracleRunnerPackager, X8664OracleRunnerSource,
    };

    #[test]
    fn assembly_source_uses_testcase_bytes_as_main_body() {
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");

        let source = X8664MachOAssemblySource::from_test_case(&test_case)
            .expect("return_42 can be used as x86_64 main")
            .into_string();

        assert_eq!(
            source,
            ".text\n.globl _main\n.p2align 4\n_main:\n.byte 0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3\n"
        );
    }

    #[test]
    fn assembly_source_rejects_argument_abi_until_runner_harness_exists() {
        let test_case = test_case_from_json(include_str!("../../../tests/cases/identity_u64.json"))
            .expect("identity_u64 testcase parses");

        let error = X8664MachOAssemblySource::from_test_case(&test_case)
            .expect_err("argument ABI requires a future oracle runner harness");

        assert!(matches!(
            error,
            X8664MachOFixtureError::UnsupportedAbi { .. }
        ));
        assert_eq!(
            error.failure_kind(),
            bara_oracle::FailureKind::InvalidTestCase
        );
    }

    #[test]
    fn assembly_source_rejects_host_traps_until_runner_harness_exists() {
        let test_case = test_case_from_json(include_str!(
            "../../../tests/cases/hello_world_stdout_return_0.json"
        ))
        .expect("stdout testcase parses");

        let error = X8664MachOAssemblySource::from_test_case(&test_case)
            .expect_err("host traps require a future oracle runner harness");

        assert!(matches!(
            error,
            X8664MachOFixtureError::UnsupportedHostTrapPlan { .. }
        ));
        assert_eq!(
            error.failure_kind(),
            bara_oracle::FailureKind::InvalidTestCase
        );
    }

    #[test]
    fn oracle_runner_source_embeds_testcase_bytes_and_json_output() {
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");

        let source = X8664OracleRunnerSource::from_test_case(&test_case)
            .expect("return_42 can be used as x86_64 oracle runner")
            .into_string();

        assert!(source.contains("typedef uint64_t (*bara_no_args_u64_fn)(void);"));
        assert!(source.contains("static const unsigned char BARA_TESTCASE_BYTES[] = {\n    0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3,\n};"));
        assert!(source.contains("static const char BARA_CASE_ID_JSON[] = \"\\\"return_42\\\"\";"));
        assert!(source.contains("printf(\"{\\\"case_id\\\":%s,\\\"exit_status\\\":0,\\\"return_value\\\":%llu,\\\"stdout\\\":\\\"\\\",\\\"stderr\\\":\\\"\\\"}\\n\""));
    }

    #[test]
    fn gui_hello_world_source_is_self_authored_appkit_fixture() {
        let source = X8664GuiHelloWorldSource::new().as_str();

        assert!(source.contains("@interface BaraGuiHelloWorldDelegate"));
        assert!(source.contains("[NSApplication sharedApplication]"));
        assert!(source.contains("[_window setTitle:@\"Bara GUI Hello World\"]"));
        assert!(source.contains("[label setStringValue:@\"hello world\"]"));
        assert!(source.contains("#ifndef BARA_GUI_HELLO_WORLD_MANUAL_VISIBLE"));
        assert!(source.contains("[NSApp activateIgnoringOtherApps:YES]"));
        assert!(source.contains("gui_window_created"));
    }

    #[test]
    fn json_string_literal_escapes_case_id_for_runner_json() {
        assert_eq!(
            json_string_literal("quote\"slash\\newline\n"),
            "\"quote\\\"slash\\\\newline\\n\""
        );
    }

    #[test]
    fn rosetta_oracle_observation_parses_only_runner_stdout_json() {
        let observation = RosettaOracleObservation::from_parts(
            true,
            String::from("exit status: 0"),
            String::from(
                "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}\n",
            ),
            String::from("ignored runner diagnostic"),
        );

        let expected = observation
            .into_expected_result(Path::new("/tmp/return_42_oracle"))
            .expect("runner stdout contains observed result JSON");

        assert_eq!(
            expected,
            ObservedResult::new(
                CaseId::new("return_42").expect("case id is non-empty"),
                0,
                42,
                String::new(),
                String::new()
            )
        );
    }

    #[test]
    fn rosetta_oracle_observation_reports_public_runner_failure_fields() {
        let observation = RosettaOracleObservation::from_parts(
            false,
            String::from("signal: 4 (SIGILL)"),
            String::from("partial stdout"),
            String::from("illegal instruction"),
        );

        let error = observation
            .into_expected_result(Path::new("/tmp/return_42_oracle"))
            .expect_err("failed runner process is not a valid oracle result");

        assert_eq!(
            error.to_string(),
            "x86_64 oracle runner /tmp/return_42_oracle failed with signal: 4 (SIGILL): stdout=\"partial stdout\" stderr=\"illegal instruction\""
        );
    }

    #[test]
    fn rosetta_gui_hello_world_observation_builds_expected_and_launch_metadata() {
        let observation = RosettaGuiHelloWorldObservation::from_parts(
            true,
            String::from("exit status: 0"),
            Some(0),
            String::from("{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}\n"),
            String::new(),
        );

        let expected = observation
            .into_expected_bundle(Path::new("/tmp/b8_gui_hello_world"))
            .expect("GUI stdout contains the expected deterministic launch event");

        assert_eq!(
            expected.observed_result(),
            &ObservedResult::new(
                CaseId::new("b8_gui_hello_world").expect("case id is non-empty"),
                0,
                0,
                String::from(
                    "{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}\n"
                ),
                String::new(),
            )
        );
        assert_eq!(
            serde_json::to_string(expected.launch_metadata()).expect("metadata serializes"),
            "{\"schema\":\"b8_gui_hello_world_launch_metadata_v0\",\"case_id\":\"b8_gui_hello_world\",\"oracle\":\"rosetta_black_box\",\"fixture\":{\"kind\":\"single_mach_o_executable\",\"source_isa\":\"x86_64\",\"binary_format\":\"mach_o\",\"target_triple\":\"x86_64-apple-macos13\",\"gui_framework\":\"appkit\"},\"observed_events\":[{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}]}"
        );
    }

    #[test]
    fn clang_assembler_command_targets_x86_64_apple_macos() {
        let output_path = X8664MachOFixtureOutputPath::from_path(Path::new("/tmp/return_42"));
        let command = X8664MachOFixtureToolchainCommand::clang_build(
            Path::new("/tmp/return_42.s"),
            X8664MachOFixtureSourceLanguage::Assembler,
            &output_path,
        );

        assert_eq!(command.program(), "clang");
        assert_eq!(
            command.args(),
            &[
                String::from("-target"),
                String::from("x86_64-apple-macos13"),
                String::from("-x"),
                String::from("assembler"),
                String::from("/tmp/return_42.s"),
                String::from("-o"),
                String::from("/tmp/return_42"),
            ]
        );
    }

    #[test]
    fn clang_c_command_targets_x86_64_apple_macos() {
        let output_path =
            X8664MachOFixtureOutputPath::from_path(Path::new("/tmp/return_42_oracle"));
        let command = X8664MachOFixtureToolchainCommand::clang_build(
            Path::new("/tmp/return_42_oracle.c"),
            X8664MachOFixtureSourceLanguage::C,
            &output_path,
        );

        assert_eq!(command.program(), "clang");
        assert_eq!(
            command.args(),
            &[
                String::from("-target"),
                String::from("x86_64-apple-macos13"),
                String::from("-x"),
                String::from("c"),
                String::from("/tmp/return_42_oracle.c"),
                String::from("-o"),
                String::from("/tmp/return_42_oracle"),
            ]
        );
    }

    #[test]
    fn clang_objective_c_appkit_command_targets_x86_64_apple_macos() {
        let output_path =
            X8664MachOFixtureOutputPath::from_path(Path::new("/tmp/b8_gui_hello_world"));
        let command = X8664MachOFixtureToolchainCommand::clang_gui_appkit_build(
            Path::new("/tmp/b8_gui_hello_world.m"),
            &output_path,
            X8664GuiHelloWorldRunMode::AutomatedOracle,
        );

        assert_eq!(command.program(), "clang");
        assert_eq!(
            command.args(),
            &[
                String::from("-target"),
                String::from("x86_64-apple-macos13"),
                String::from("-x"),
                String::from("objective-c"),
                String::from("/tmp/b8_gui_hello_world.m"),
                String::from("-framework"),
                String::from("AppKit"),
                String::from("-o"),
                String::from("/tmp/b8_gui_hello_world"),
            ]
        );
    }

    #[test]
    fn clang_objective_c_appkit_visible_command_defines_manual_visible_mode() {
        let output_path =
            X8664MachOFixtureOutputPath::from_path(Path::new("/tmp/b8_gui_hello_world_visible"));
        let command = X8664MachOFixtureToolchainCommand::clang_gui_appkit_build(
            Path::new("/tmp/b8_gui_hello_world.m"),
            &output_path,
            X8664GuiHelloWorldRunMode::ManualVisible,
        );

        assert_eq!(command.program(), "clang");
        assert_eq!(
            command.args(),
            &[
                String::from("-target"),
                String::from("x86_64-apple-macos13"),
                String::from("-x"),
                String::from("objective-c"),
                String::from("-DBARA_GUI_HELLO_WORLD_MANUAL_VISIBLE=1"),
                String::from("/tmp/b8_gui_hello_world.m"),
                String::from("-framework"),
                String::from("AppKit"),
                String::from("-o"),
                String::from("/tmp/b8_gui_hello_world_visible"),
            ]
        );
    }

    #[test]
    fn fixture_packaging_boundary_accepts_non_clang_packager() {
        let temp_dir = TestTempDir::new("fixture_packaging_boundary_accepts_non_clang_packager");
        let output_path = temp_dir.path.join("return_42");
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");
        let request = X8664MachOFixtureBuildRequest::from_test_case(&test_case, &output_path)
            .expect("request builds from return_42");

        let fixture = package_x86_64_mach_o_fixture(&FakePackager, request)
            .expect("fake packager can produce a fixture artifact");

        assert_eq!(
            serde_json::to_string(fixture.metadata()).expect("metadata serializes"),
            format!(
                "{{\"artifact_kind\":\"mach_o_executable\",\"case_id\":\"return_42\",\"target_triple\":\"x86_64-apple-macos13\",\"toolchain\":\"clang\",\"output_path\":\"{}\"}}",
                output_path.display()
            )
        );
    }

    #[test]
    fn oracle_runner_packaging_boundary_accepts_non_clang_packager() {
        let temp_dir =
            TestTempDir::new("oracle_runner_packaging_boundary_accepts_non_clang_packager");
        let output_path = temp_dir.path.join("return_42_oracle");
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");
        let request = X8664OracleRunnerBuildRequest::from_test_case(&test_case, &output_path)
            .expect("request builds from return_42");

        let runner = package_x86_64_oracle_runner(&FakeOracleRunnerPackager, request)
            .expect("fake packager can produce an oracle runner artifact");

        assert_eq!(
            serde_json::to_string(runner.metadata()).expect("metadata serializes"),
            format!(
                "{{\"artifact_kind\":\"oracle_runner_executable\",\"case_id\":\"return_42\",\"target_triple\":\"x86_64-apple-macos13\",\"toolchain\":\"clang\",\"output_path\":\"{}\"}}",
                output_path.display()
            )
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn observe_x86_64_oracle_expected_runs_return_42_under_rosetta() {
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");

        let expected = observe_x86_64_oracle_expected(&test_case)
            .expect("return_42 x86_64 oracle runner runs under Rosetta");

        assert_eq!(
            expected,
            ObservedResult::new(
                CaseId::new("return_42").expect("case id is non-empty"),
                0,
                42,
                String::new(),
                String::new()
            )
        );
    }

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    #[test]
    fn observe_x86_64_oracle_expected_reports_unsupported_rosetta_host() {
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");

        let error = observe_x86_64_oracle_expected(&test_case)
            .expect_err("Rosetta oracle execution requires arm64 macOS");

        assert!(matches!(
            error,
            X8664MachOFixtureError::UnsupportedRosettaHost { .. }
        ));
        assert_eq!(error.failure_kind(), bara_oracle::FailureKind::RunError);
    }

    struct FakePackager;

    impl X8664MachOFixturePackager for FakePackager {
        fn package(
            &self,
            request: X8664MachOFixtureBuildRequest,
        ) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError> {
            fs::write(request.output_path().as_path(), b"fake mach-o").map_err(|source| {
                X8664MachOFixtureError::WriteSource {
                    path: request.output_path().to_path_buf(),
                    source,
                }
            })?;
            Ok(GeneratedX8664MachOFixture::from_request(request))
        }
    }

    struct FakeOracleRunnerPackager;

    impl X8664OracleRunnerPackager for FakeOracleRunnerPackager {
        fn package(
            &self,
            request: X8664OracleRunnerBuildRequest,
        ) -> Result<GeneratedX8664OracleRunner, X8664MachOFixtureError> {
            fs::write(request.output_path().as_path(), b"fake oracle runner").map_err(
                |source| X8664MachOFixtureError::WriteSource {
                    path: request.output_path().to_path_buf(),
                    source,
                },
            )?;
            Ok(GeneratedX8664OracleRunner::from_request(request))
        }
    }

    struct TestTempDir {
        path: PathBuf,
    }

    impl TestTempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("bara-{name}-{nanos}"));
            fs::create_dir(&path).expect("test temp dir is created");
            Self { path }
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let result = fs::remove_dir_all(&self.path);
            if let Err(error) = result {
                assert_eq!(error.kind(), io::ErrorKind::NotFound);
            }
        }
    }
}
