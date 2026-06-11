use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use bara_isa_x86::{decode_function, DecodeError, DecodedFunction, DecodedInstructionKind};
use bara_oracle::{
    binary_format_probe_report_to_json, probe_public_binary_format, BinaryFileBytes,
    BinaryFormatProbeError, BinaryInput, JsonError,
};
use serde::Serialize;

use crate::{
    function_run::{run_test_case_function_with_bundle, FunctionRunError, FunctionRunResult},
    gui_hello_world_translated::{
        translated_entry_test_case, GuiHelloWorldTranslatedLaunchError,
        B8_GUI_TRANSLATED_ENTRY_BYTES, B8_GUI_TRANSLATED_ENTRY_CASE_ID,
    },
    x86_64_mach_o_fixture::{b8_gui_hello_world_case_id, X8664MachOFixtureError},
};

pub(crate) fn generate_b8_debug_bundle(
    binary_path: &Path,
    output_root: &Path,
) -> Result<String, B8DebugBundleError> {
    let case_id = b8_gui_hello_world_case_id().map_err(B8DebugBundleError::B8CaseId)?;
    let bundle_dir = output_root.join(case_id.as_str());
    create_dir(&bundle_dir)?;

    let input_bytes = read_binary_file(binary_path)?;
    let input =
        BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(input_bytes));
    let input_probe = probe_public_binary_format(&input).map_err(B8DebugBundleError::Probe)?;
    let input_probe_json =
        binary_format_probe_report_to_json(&input_probe).map_err(B8DebugBundleError::Json)?;

    let entry_test_case =
        translated_entry_test_case().map_err(B8DebugBundleError::TranslatedEntry)?;
    let decoded =
        decode_function(entry_test_case.x86_bytes()).map_err(B8DebugBundleError::Decode)?;
    let runtime = run_test_case_function_with_bundle(&entry_test_case)
        .map_err(B8DebugBundleError::FunctionRun)?;
    let artifacts = runtime.compiled().artifact_metadata(&entry_test_case);
    let paths = B8DebugBundleOutputPaths::from_dir(&bundle_dir);

    write_text_file(&paths.input_probe_path(), &input_probe_json)?;
    write_binary_file(&paths.entry_bytes_bin_path(), B8_GUI_TRANSLATED_ENTRY_BYTES)?;
    write_json_file(
        &paths.entry_bytes_json_path(),
        &B8DebugEntryBytesReport::translated_host_trap_entry(),
    )?;
    write_json_file(
        &paths.decode_report_path(),
        &B8DebugDecodeReport::from_decoded(&decoded),
    )?;
    write_json_file(&paths.lift_ir_path(), artifacts.compiled_ir())?;
    write_json_file(&paths.emit_report_path(), artifacts.artifact_report())?;
    write_json_file(&paths.pcmap_path(), artifacts.pcmap())?;
    write_json_file(&paths.fixups_path(), artifacts.fixups())?;
    write_json_file(&paths.helpers_path(), artifacts.helpers())?;
    write_json_file(
        &paths.loader_plan_path(),
        &B8DebugLoaderPlanReport::planned(),
    )?;
    write_json_file(
        &paths.runtime_attempt_path(),
        &B8DebugRuntimeAttemptReport::from_result(runtime.result()),
    )?;
    write_json_file(
        &paths.blocker_path(),
        &B8DebugBlockerReport::ready_for_b8_g2(),
    )?;
    write_text_file(
        &paths.repro_path(),
        &B8DebugReproScript::new(binary_path, output_root).into_script(),
    )?;

    serde_json::to_string(&paths)
        .map_err(JsonError::new)
        .map_err(B8DebugBundleError::Json)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugBundleOutputPaths {
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
    blocker: String,
    repro: String,
}

impl B8DebugBundleOutputPaths {
    fn from_dir(bundle_dir: &Path) -> Self {
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
            blocker: path_string(&bundle_dir.join("blocker.json")),
            repro: path_string(&bundle_dir.join("repro.sh")),
        }
    }

    fn input_probe_path(&self) -> PathBuf {
        PathBuf::from(&self.input_probe)
    }

    fn entry_bytes_bin_path(&self) -> PathBuf {
        PathBuf::from(&self.entry_bytes_bin)
    }

    fn entry_bytes_json_path(&self) -> PathBuf {
        PathBuf::from(&self.entry_bytes_json)
    }

    fn decode_report_path(&self) -> PathBuf {
        PathBuf::from(&self.decode_report)
    }

    fn lift_ir_path(&self) -> PathBuf {
        PathBuf::from(&self.lift_ir)
    }

    fn emit_report_path(&self) -> PathBuf {
        PathBuf::from(&self.emit_report)
    }

    fn pcmap_path(&self) -> PathBuf {
        PathBuf::from(&self.pcmap)
    }

    fn fixups_path(&self) -> PathBuf {
        PathBuf::from(&self.fixups)
    }

    fn helpers_path(&self) -> PathBuf {
        PathBuf::from(&self.helpers)
    }

    fn loader_plan_path(&self) -> PathBuf {
        PathBuf::from(&self.loader_plan)
    }

    fn runtime_attempt_path(&self) -> PathBuf {
        PathBuf::from(&self.runtime_attempt)
    }

    fn blocker_path(&self) -> PathBuf {
        PathBuf::from(&self.blocker)
    }

    fn repro_path(&self) -> PathBuf {
        PathBuf::from(&self.repro)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugEntryBytesReport {
    schema: &'static str,
    case_id: &'static str,
    source: B8DebugEntrySource,
    source_isa: B8DebugSourceIsa,
    source_pc: u64,
    byte_len: usize,
    bytes_hex: &'static str,
}

impl B8DebugEntryBytesReport {
    const fn translated_host_trap_entry() -> Self {
        Self {
            schema: "b8_debug_entry_bytes_v0",
            case_id: B8_GUI_TRANSLATED_ENTRY_CASE_ID,
            source: B8DebugEntrySource::B8G1TranslatedHostTrapEntry,
            source_isa: B8DebugSourceIsa::X8664,
            source_pc: 0,
            byte_len: B8_GUI_TRANSLATED_ENTRY_BYTES.len(),
            bytes_hex: "0f0b4238473131c0c3",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugEntrySource {
    B8G1TranslatedHostTrapEntry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum B8DebugSourceIsa {
    #[serde(rename = "x86_64")]
    X8664,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugDecodeReport {
    schema: &'static str,
    status: B8DebugStageStatus,
    entry: u64,
    instructions: Vec<B8DebugDecodedInstructionReport>,
}

impl B8DebugDecodeReport {
    fn from_decoded(decoded: &DecodedFunction) -> Self {
        Self {
            schema: "b8_debug_decode_report_v0",
            status: B8DebugStageStatus::Executed,
            entry: decoded.entry().value(),
            instructions: decoded
                .instructions()
                .iter()
                .map(B8DebugDecodedInstructionReport::from_instruction)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugDecodedInstructionReport {
    start: u64,
    end: u64,
    kind: B8DebugDecodedInstructionKindReport,
}

impl B8DebugDecodedInstructionReport {
    fn from_instruction(instruction: &bara_isa_x86::DecodedInstruction) -> Self {
        Self {
            start: instruction.start().value(),
            end: instruction.end().value(),
            kind: B8DebugDecodedInstructionKindReport::from_kind(instruction.kind()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum B8DebugDecodedInstructionKindReport {
    MovEaxImm32 {
        imm: u32,
    },
    MovRaxRdi,
    MovzxEaxBytePtrRdi,
    AddEaxImm32 {
        imm: String,
    },
    AddEaxImm8 {
        imm: String,
    },
    SubEaxImm32 {
        imm: String,
    },
    SubEaxImm8 {
        imm: String,
    },
    CmpEaxImm32 {
        imm: String,
    },
    CmpEaxImm8 {
        imm: String,
    },
    TestEaxEax,
    PushRax,
    PopRax,
    XorEaxEax,
    JccRel8 {
        condition: String,
        taken: u64,
        fallthrough: u64,
    },
    JccRel32 {
        condition: String,
        taken: u64,
        fallthrough: u64,
    },
    JmpRel8 {
        target: u64,
    },
    CallRel32 {
        target: u64,
        return_to: u64,
    },
    Syscall,
    BaraHostTrapSentinel,
    BaraAppKitGuiHelloWorldTrapSentinel,
    Ret,
    Unsupported {
        reason: String,
    },
}

impl B8DebugDecodedInstructionKindReport {
    fn from_kind(kind: &DecodedInstructionKind) -> Self {
        match kind {
            DecodedInstructionKind::MovEaxImm32 { imm } => Self::MovEaxImm32 { imm: *imm },
            DecodedInstructionKind::MovRaxRdi => Self::MovRaxRdi,
            DecodedInstructionKind::MovzxEaxBytePtrRdi => Self::MovzxEaxBytePtrRdi,
            DecodedInstructionKind::AddEaxImm32 { imm } => Self::AddEaxImm32 {
                imm: format!("{imm:?}"),
            },
            DecodedInstructionKind::AddEaxImm8 { imm } => Self::AddEaxImm8 {
                imm: format!("{imm:?}"),
            },
            DecodedInstructionKind::SubEaxImm32 { imm } => Self::SubEaxImm32 {
                imm: format!("{imm:?}"),
            },
            DecodedInstructionKind::SubEaxImm8 { imm } => Self::SubEaxImm8 {
                imm: format!("{imm:?}"),
            },
            DecodedInstructionKind::CmpEaxImm32 { imm } => Self::CmpEaxImm32 {
                imm: format!("{imm:?}"),
            },
            DecodedInstructionKind::CmpEaxImm8 { imm } => Self::CmpEaxImm8 {
                imm: format!("{imm:?}"),
            },
            DecodedInstructionKind::TestEaxEax => Self::TestEaxEax,
            DecodedInstructionKind::PushRax => Self::PushRax,
            DecodedInstructionKind::PopRax => Self::PopRax,
            DecodedInstructionKind::XorEaxEax => Self::XorEaxEax,
            DecodedInstructionKind::JccRel8 {
                condition,
                taken,
                fallthrough,
            } => Self::JccRel8 {
                condition: format!("{condition:?}"),
                taken: taken.value(),
                fallthrough: fallthrough.value(),
            },
            DecodedInstructionKind::JccRel32 {
                condition,
                taken,
                fallthrough,
            } => Self::JccRel32 {
                condition: format!("{condition:?}"),
                taken: taken.value(),
                fallthrough: fallthrough.value(),
            },
            DecodedInstructionKind::JmpRel8 { target } => Self::JmpRel8 {
                target: target.value(),
            },
            DecodedInstructionKind::CallRel32 { target, return_to } => Self::CallRel32 {
                target: target.value(),
                return_to: return_to.value(),
            },
            DecodedInstructionKind::Syscall => Self::Syscall,
            DecodedInstructionKind::BaraHostTrapSentinel => Self::BaraHostTrapSentinel,
            DecodedInstructionKind::BaraAppKitGuiHelloWorldTrapSentinel => {
                Self::BaraAppKitGuiHelloWorldTrapSentinel
            }
            DecodedInstructionKind::Ret => Self::Ret,
            DecodedInstructionKind::Unsupported { reason } => Self::Unsupported {
                reason: format!("{reason:?}"),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLoaderPlanReport {
    schema: &'static str,
    source: &'static str,
    status: B8DebugStageStatus,
    input_metadata: B8DebugLoaderInputMetadata,
    entry_source_for_this_bundle: B8DebugEntrySource,
    next_entry_source: B8DebugLoaderNextEntrySource,
}

impl B8DebugLoaderPlanReport {
    const fn planned() -> Self {
        Self {
            schema: "b8_debug_loader_plan_v0",
            source: "bara_runtime_user_space_launch_plan",
            status: B8DebugStageStatus::PlannedNotExecuted,
            input_metadata: B8DebugLoaderInputMetadata::PublicMachOProbe,
            entry_source_for_this_bundle: B8DebugEntrySource::B8G1TranslatedHostTrapEntry,
            next_entry_source: B8DebugLoaderNextEntrySource::PublicLcMainEntryoff,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderInputMetadata {
    PublicMachOProbe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderNextEntrySource {
    PublicLcMainEntryoff,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugRuntimeAttemptReport {
    schema: &'static str,
    status: B8DebugStageStatus,
    run_scope: B8DebugRuntimeRunScope,
    return_value: u64,
    stdout: String,
}

impl B8DebugRuntimeAttemptReport {
    fn from_result(result: &FunctionRunResult) -> Self {
        Self {
            schema: "b8_debug_runtime_attempt_v0",
            status: B8DebugStageStatus::Executed,
            run_scope: B8DebugRuntimeRunScope::TranslatedEntryOnly,
            return_value: result.return_value(),
            stdout: result.stdout().to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugRuntimeRunScope {
    TranslatedEntryOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugBlockerReport {
    schema: &'static str,
    status: B8DebugBlockerStatus,
    current_blocker: B8DebugBlocker,
    next_action: B8DebugNextAction,
}

impl B8DebugBlockerReport {
    const fn ready_for_b8_g2() -> Self {
        Self {
            schema: "b8_debug_blocker_v0",
            status: B8DebugBlockerStatus::ReadyForNextPrGate,
            current_blocker: B8DebugBlocker::RealLcMainEntryNotAttempted,
            next_action: B8DebugNextAction::AdvanceToB8G2RealLcMainEntry,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugBlockerStatus {
    ReadyForNextPrGate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugBlocker {
    RealLcMainEntryNotAttempted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugNextAction {
    AdvanceToB8G2RealLcMainEntry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugStageStatus {
    Executed,
    PlannedNotExecuted,
}

struct B8DebugReproScript<'a> {
    binary_path: &'a Path,
    output_root: &'a Path,
}

impl<'a> B8DebugReproScript<'a> {
    const fn new(binary_path: &'a Path, output_root: &'a Path) -> Self {
        Self {
            binary_path,
            output_root,
        }
    }

    fn into_script(self) -> String {
        format!(
            "#!/usr/bin/env sh\nset -eu\nnix develop -c cargo run -p btbc-cli -- generate-b8-debug-bundle {} {}\n",
            shell_single_quote(&path_string(self.binary_path)),
            shell_single_quote(&path_string(self.output_root))
        )
    }
}

#[derive(Debug)]
pub(crate) enum B8DebugBundleError {
    ReadFile { path: PathBuf, source: io::Error },
    WriteFile { path: PathBuf, source: io::Error },
    CreateDir { path: PathBuf, source: io::Error },
    Probe(BinaryFormatProbeError),
    B8CaseId(X8664MachOFixtureError),
    TranslatedEntry(GuiHelloWorldTranslatedLaunchError),
    Decode(DecodeError),
    FunctionRun(FunctionRunError),
    Json(JsonError),
}

impl fmt::Display for B8DebugBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFile { path, source } => {
                write!(
                    formatter,
                    "failed to read B8 debug input {}: {source}",
                    path.display()
                )
            }
            Self::WriteFile { path, source } => {
                write!(
                    formatter,
                    "failed to write B8 debug bundle file {}: {source}",
                    path.display()
                )
            }
            Self::CreateDir { path, source } => {
                write!(
                    formatter,
                    "failed to create B8 debug bundle directory {}: {source}",
                    path.display()
                )
            }
            Self::Probe(error) => write!(formatter, "B8 debug input probe failed: {error:?}"),
            Self::B8CaseId(error) => write!(formatter, "B8 debug case id error: {error}"),
            Self::TranslatedEntry(error) => {
                write!(formatter, "B8 debug translated entry error: {error}")
            }
            Self::Decode(error) => write!(formatter, "B8 debug decode failed: {error:?}"),
            Self::FunctionRun(error) => write!(formatter, "B8 debug runtime failed: {error}"),
            Self::Json(error) => write!(formatter, "B8 debug JSON error: {error}"),
        }
    }
}

impl Error for B8DebugBundleError {}

fn read_binary_file(path: &Path) -> Result<Vec<u8>, B8DebugBundleError> {
    fs::read(path).map_err(|source| B8DebugBundleError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

fn create_dir(path: &Path) -> Result<(), B8DebugBundleError> {
    fs::create_dir_all(path).map_err(|source| B8DebugBundleError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), B8DebugBundleError> {
    let json = serde_json::to_string(value)
        .map_err(JsonError::new)
        .map_err(B8DebugBundleError::Json)?;
    write_text_file(path, &json)
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), B8DebugBundleError> {
    fs::write(path, contents).map_err(|source| B8DebugBundleError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn write_binary_file(path: &Path, contents: &[u8]) -> Result<(), B8DebugBundleError> {
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
