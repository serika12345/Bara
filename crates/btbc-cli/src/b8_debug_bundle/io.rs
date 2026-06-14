use std::{
    fs,
    path::{Path, PathBuf},
};

use bara_oracle::JsonError;
use serde::Serialize;

use super::B8DebugBundleError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugBundleOutputPaths {
    bundle_dir: String,
    input_probe: String,
    entry_bytes_bin: String,
    entry_bytes_json: String,
    decode_report: String,
    lift_ir: String,
    emit_report: String,
    pcmap: String,
    fixups: String,
    helpers: String,
    loader_plan: String,
    runtime_attempt: String,
    launch_report: String,
    blocker: String,
    repro: String,
}

impl B8DebugBundleOutputPaths {
    pub(super) fn from_dir(bundle_dir: &Path) -> Self {
        Self {
            bundle_dir: path_string(bundle_dir),
            input_probe: path_string(&bundle_dir.join("input.probe.json")),
            entry_bytes_bin: path_string(&bundle_dir.join("entry.bytes.bin")),
            entry_bytes_json: path_string(&bundle_dir.join("entry.bytes.json")),
            decode_report: path_string(&bundle_dir.join("decode.report.json")),
            lift_ir: path_string(&bundle_dir.join("lift.ir.json")),
            emit_report: path_string(&bundle_dir.join("emit.report.json")),
            pcmap: path_string(&bundle_dir.join("pcmap.json")),
            fixups: path_string(&bundle_dir.join("fixups.json")),
            helpers: path_string(&bundle_dir.join("helpers.json")),
            loader_plan: path_string(&bundle_dir.join("loader.plan.json")),
            runtime_attempt: path_string(&bundle_dir.join("runtime-attempt.json")),
            launch_report: path_string(&bundle_dir.join("launch.report.json")),
            blocker: path_string(&bundle_dir.join("blocker.json")),
            repro: path_string(&bundle_dir.join("repro.sh")),
        }
    }

    pub(super) fn input_probe_path(&self) -> PathBuf {
        PathBuf::from(&self.input_probe)
    }

    pub(super) fn entry_bytes_bin_path(&self) -> PathBuf {
        PathBuf::from(&self.entry_bytes_bin)
    }

    pub(super) fn entry_bytes_json_path(&self) -> PathBuf {
        PathBuf::from(&self.entry_bytes_json)
    }

    pub(super) fn decode_report_path(&self) -> PathBuf {
        PathBuf::from(&self.decode_report)
    }

    pub(super) fn lift_ir_path(&self) -> PathBuf {
        PathBuf::from(&self.lift_ir)
    }

    pub(super) fn emit_report_path(&self) -> PathBuf {
        PathBuf::from(&self.emit_report)
    }

    pub(super) fn pcmap_path(&self) -> PathBuf {
        PathBuf::from(&self.pcmap)
    }

    pub(super) fn fixups_path(&self) -> PathBuf {
        PathBuf::from(&self.fixups)
    }

    pub(super) fn helpers_path(&self) -> PathBuf {
        PathBuf::from(&self.helpers)
    }

    pub(super) fn loader_plan_path(&self) -> PathBuf {
        PathBuf::from(&self.loader_plan)
    }

    pub(super) fn runtime_attempt_path(&self) -> PathBuf {
        PathBuf::from(&self.runtime_attempt)
    }

    pub(super) fn launch_report_path(&self) -> PathBuf {
        PathBuf::from(&self.launch_report)
    }

    pub(super) fn blocker_path(&self) -> PathBuf {
        PathBuf::from(&self.blocker)
    }

    pub(super) fn repro_path(&self) -> PathBuf {
        PathBuf::from(&self.repro)
    }
}

pub(super) struct B8DebugReproScript<'a> {
    binary_path: &'a Path,
    output_root: &'a Path,
}

impl<'a> B8DebugReproScript<'a> {
    pub(super) const fn new(binary_path: &'a Path, output_root: &'a Path) -> Self {
        Self {
            binary_path,
            output_root,
        }
    }

    pub(super) fn into_script(self) -> String {
        format!(
            "#!/usr/bin/env sh\nset -eu\nnix develop -c cargo run -p btbc-cli -- generate-b8-debug-bundle {} {}\n",
            shell_single_quote(&path_string(self.binary_path)),
            shell_single_quote(&path_string(self.output_root))
        )
    }
}

pub(super) fn read_binary_file(path: &Path) -> Result<Vec<u8>, B8DebugBundleError> {
    fs::read(path).map_err(|source| B8DebugBundleError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn create_dir(path: &Path) -> Result<(), B8DebugBundleError> {
    fs::create_dir_all(path).map_err(|source| B8DebugBundleError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn write_json_file<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), B8DebugBundleError> {
    let json = serde_json::to_string(value)
        .map_err(JsonError::new)
        .map_err(B8DebugBundleError::Json)?;
    write_text_file(path, &json)
}

pub(super) fn write_text_file(path: &Path, contents: &str) -> Result<(), B8DebugBundleError> {
    fs::write(path, contents).map_err(|source| B8DebugBundleError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn write_binary_file(path: &Path, contents: &[u8]) -> Result<(), B8DebugBundleError> {
    fs::write(path, contents).map_err(|source| B8DebugBundleError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
