use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{CaseId, FailureKind, TestCase, TestCaseAbi};
use serde::{Serialize, Serializer};

#[derive(Debug)]
pub(crate) enum X8664MachOFixtureError {
    UnsupportedHost {
        os: &'static str,
        arch: &'static str,
    },
    UnsupportedAbi {
        case_id: CaseId,
    },
    UnsupportedHostTrapPlan {
        case_id: CaseId,
    },
    TempAssemblyPath {
        source: std::time::SystemTimeError,
    },
    WriteAssembly {
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
}

impl X8664MachOFixtureError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::UnsupportedAbi { .. } | Self::UnsupportedHostTrapPlan { .. } => {
                FailureKind::InvalidTestCase
            }
            Self::UnsupportedHost { .. }
            | Self::TempAssemblyPath { .. }
            | Self::WriteAssembly { .. }
            | Self::ClangSpawn { .. }
            | Self::ClangFailed { .. }
            | Self::MissingOutput { .. } => FailureKind::EmitError,
        }
    }
}

impl fmt::Display for X8664MachOFixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedHost { os, arch } => write!(
                formatter,
                "x86_64 Mach-O fixture generation is unsupported on host os={os} arch={arch}"
            ),
            Self::UnsupportedAbi { case_id } => write!(
                formatter,
                "x86_64 Mach-O fixture generation supports only no-args u64 testcases: {}",
                case_id.as_str()
            ),
            Self::UnsupportedHostTrapPlan { case_id } => write!(
                formatter,
                "x86_64 Mach-O fixture generation does not support host trap testcases yet: {}",
                case_id.as_str()
            ),
            Self::TempAssemblyPath { source } => {
                write!(
                    formatter,
                    "failed to build temporary x86_64 assembly path: {source}"
                )
            }
            Self::WriteAssembly { path, source } => {
                write!(
                    formatter,
                    "failed to write temporary x86_64 assembly {}: {source}",
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
            metadata: X8664MachOFixtureMetadata::new(request.case_id, request.output_path),
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
    fn new(case_id: CaseId, output_path: X8664MachOFixtureOutputPath) -> Self {
        Self {
            artifact_kind: X8664MachOFixtureArtifactKind::MachOExecutable,
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

struct ClangX8664MachOFixturePackager;

impl X8664MachOFixturePackager for ClangX8664MachOFixturePackager {
    fn package(
        &self,
        request: X8664MachOFixtureBuildRequest,
    ) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError> {
        let assembly_path = temporary_assembly_path()?;
        fs::write(&assembly_path, request.source().as_str()).map_err(|source| {
            X8664MachOFixtureError::WriteAssembly {
                path: assembly_path.clone(),
                source,
            }
        })?;

        let toolchain_command =
            X8664MachOFixtureToolchainCommand::clang_build(&assembly_path, request.output_path());
        let output = toolchain_command
            .to_command()
            .output()
            .map_err(|source| X8664MachOFixtureError::ClangSpawn { source });

        let _ = fs::remove_file(&assembly_path);

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

#[derive(Clone, Debug, Eq, PartialEq)]
struct X8664MachOFixtureToolchainCommand {
    program: &'static str,
    args: Vec<String>,
}

impl X8664MachOFixtureToolchainCommand {
    fn clang_build(assembly_path: &Path, output_path: &X8664MachOFixtureOutputPath) -> Self {
        Self {
            program: "clang",
            args: vec![
                String::from("-target"),
                X8664MachOFixtureTargetTriple::X8664AppleMacos13
                    .as_str()
                    .to_owned(),
                String::from("-x"),
                String::from("assembler"),
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

pub(crate) fn build_x86_64_mach_o_fixture(
    test_case: &TestCase,
    output_path: &Path,
) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError> {
    ensure_supported_host()?;

    let request = X8664MachOFixtureBuildRequest::from_test_case(test_case, output_path)?;
    package_x86_64_mach_o_fixture(&ClangX8664MachOFixturePackager, request)
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

fn temporary_assembly_path() -> Result<PathBuf, X8664MachOFixtureError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| X8664MachOFixtureError::TempAssemblyPath { source })?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("bara-x86-64-macho-{nanos}.s")))
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
        fs, io,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bara_oracle::test_case_from_json;

    use super::{
        package_x86_64_mach_o_fixture, GeneratedX8664MachOFixture, X8664MachOAssemblySource,
        X8664MachOFixtureBuildRequest, X8664MachOFixtureError, X8664MachOFixtureOutputPath,
        X8664MachOFixturePackager, X8664MachOFixtureToolchainCommand,
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
    fn clang_command_targets_x86_64_apple_macos() {
        let output_path = X8664MachOFixtureOutputPath::from_path(Path::new("/tmp/return_42"));
        let command = X8664MachOFixtureToolchainCommand::clang_build(
            Path::new("/tmp/return_42.s"),
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

    struct FakePackager;

    impl X8664MachOFixturePackager for FakePackager {
        fn package(
            &self,
            request: X8664MachOFixtureBuildRequest,
        ) -> Result<GeneratedX8664MachOFixture, X8664MachOFixtureError> {
            fs::write(request.output_path().as_path(), b"fake mach-o").map_err(|source| {
                X8664MachOFixtureError::WriteAssembly {
                    path: request.output_path().to_path_buf(),
                    source,
                }
            })?;
            Ok(GeneratedX8664MachOFixture::from_request(request))
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
