use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{
    observed_result_from_json, CaseId, FailureKind, ObservedResult, TestCase, TestCaseAbi,
};
use serde::{Serialize, Serializer};

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
}

impl X8664MachOFixtureError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::UnsupportedAbi { .. } | Self::UnsupportedHostTrapPlan { .. } => {
                FailureKind::InvalidTestCase
            }
            Self::UnsupportedHost { .. }
            | Self::TempPath { .. }
            | Self::WriteSource { .. }
            | Self::ClangSpawn { .. }
            | Self::ClangFailed { .. }
            | Self::MissingOutput { .. } => FailureKind::EmitError,
            Self::UnsupportedRosettaHost { .. }
            | Self::RunnerSpawn { .. }
            | Self::RunnerFailed { .. }
            | Self::InvalidRunnerStdout { .. } => FailureKind::RunError,
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
                X8664MachOFixtureArtifactKind::MachOExecutable,
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
                X8664MachOFixtureArtifactKind::OracleRunnerExecutable,
                request.case_id,
                request.output_path,
            ),
        }
    }

    pub(crate) const fn metadata(&self) -> &X8664MachOFixtureMetadata {
        &self.metadata
    }
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
    MachOExecutable,
    OracleRunnerExecutable,
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
}

impl X8664MachOFixtureSourceLanguage {
    const fn as_clang_arg(self) -> &'static str {
        match self {
            Self::Assembler => "assembler",
            Self::C => "c",
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
        RosettaOracleObservation, X8664MachOAssemblySource, X8664MachOFixtureBuildRequest,
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
