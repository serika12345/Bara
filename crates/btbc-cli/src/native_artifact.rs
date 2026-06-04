use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::FailureKind;

use crate::function_run::FunctionArm64Bytes;

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
}

impl NativeArtifactError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::UnsupportedHost { .. } => FailureKind::EmitError,
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
        }
    }
}

impl Error for NativeArtifactError {}

pub(crate) fn link_arm64_main_executable(
    body: FunctionArm64Bytes<'_>,
    output_path: &Path,
) -> Result<(), NativeArtifactError> {
    ensure_supported_host()?;

    let assembly_path = temporary_assembly_path()?;
    let source = arm64_main_assembly_source(body);
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
    source
}

#[cfg(test)]
mod tests {
    use super::arm64_main_assembly_source_from_bytes;

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
}
