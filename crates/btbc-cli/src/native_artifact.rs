use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{FailureKind, TestCaseHostTrapPlan};

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
    StdoutMainUnsupported(NativeStdoutMainUnsupported),
}

impl NativeArtifactError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::UnsupportedHost { .. } => FailureKind::EmitError,
            Self::StdoutMainUnsupported(_) => FailureKind::EmitError,
            Self::TempAssemblyPath { .. }
            | Self::WriteAssembly { .. }
            | Self::LinkerSpawn { .. }
            | Self::LinkerFailed { .. }
            | Self::MissingLinkedExecutable { .. } => FailureKind::InvalidTestCase,
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

pub(crate) fn link_arm64_main_executable(
    body: FunctionArm64Bytes<'_>,
    output_path: &Path,
) -> Result<(), NativeArtifactError> {
    ensure_supported_host()?;

    let source = arm64_main_assembly_source(body);
    link_assembly_source(&source, output_path)
}

pub(crate) fn link_arm64_stdout_main_executable(
    body: FunctionArm64Bytes<'_>,
    host_trap_plan: &TestCaseHostTrapPlan,
    stdout_request: FunctionStdoutHostTrapRequest,
    output_path: &Path,
) -> Result<(), NativeArtifactError> {
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
    link_assembly_source(&source, output_path)
}

fn link_assembly_source(source: &str, output_path: &Path) -> Result<(), NativeArtifactError> {
    let assembly_path = temporary_assembly_path()?;
    fs::write(&assembly_path, source).map_err(|source| NativeArtifactError::WriteAssembly {
        path: assembly_path.clone(),
        source,
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

    Ok(())
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

fn arm64_main_assembly_source(body: FunctionArm64Bytes<'_>) -> String {
    arm64_main_assembly_source_from_bytes(body.as_slice())
}

fn arm64_main_assembly_source_from_bytes(bytes: &[u8]) -> String {
    let mut source = String::from(".text\n.globl _main\n.p2align 2\n_main:\n");
    push_byte_directives(&mut source, bytes);
    source
}

fn arm64_stdout_main_assembly_source(body: FunctionArm64Bytes<'_>, stdout_text: &str) -> String {
    arm64_stdout_main_assembly_source_from_parts(body.as_slice(), stdout_text.as_bytes())
}

fn arm64_stdout_main_assembly_source_from_parts(body_bytes: &[u8], stdout_bytes: &[u8]) -> String {
    let mut source = String::from(".text\n.globl _main\n.p2align 2\n_main:\n");
    source.push_str("stp x29, x30, [sp, #-16]!\n");
    source.push_str("mov x29, sp\n");
    source.push_str("mov x0, #1\n");
    source.push_str("adrp x1, L_stdout_text@PAGE\n");
    source.push_str("add x1, x1, L_stdout_text@PAGEOFF\n");
    source.push_str(&format!("mov x2, #{}\n", stdout_bytes.len()));
    source.push_str("bl _write\n");
    source.push_str("ldp x29, x30, [sp], #16\n");
    push_byte_directives(&mut source, body_bytes);
    source.push_str(".section __TEXT,__const\n.p2align 2\nL_stdout_text:\n");
    push_byte_directives(&mut source, stdout_bytes);
    source
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
    use super::{
        arm64_main_assembly_source_from_bytes, arm64_stdout_main_assembly_source_from_parts,
    };

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
