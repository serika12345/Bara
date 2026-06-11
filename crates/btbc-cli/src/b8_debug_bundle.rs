use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use bara_arm64::emit_program;
use bara_ir::ProgramImageMetadata;
use bara_isa_x86::{
    decode_function, lift_decoded_function_with_image_metadata, DecodeError, DecodedFunction,
    DecodedInstructionKind, LiftError,
};
use bara_oracle::{
    binary_format_probe_report_to_json, mach_o_entry_function_input, probe_public_binary_format,
    BinaryFileBytes, BinaryFormatProbeError, BinaryInput, FailureKind, JsonError,
    MachOEntryFunctionTestCaseError, TestCase,
};
use serde::Serialize;

use crate::{
    function_run::{
        run_compiled_test_case_function_with_bundle, FunctionArtifactReport, FunctionCompileResult,
        FunctionCompiledIrArtifact, FunctionFixupsArtifact, FunctionHelpersArtifact,
        FunctionPcMapArtifact, FunctionRunError, FunctionRunResult,
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

    let entry_input =
        mach_o_entry_function_input(case_id.clone(), &input).map_err(B8DebugBundleError::Entry)?;
    let entry_test_case = entry_input.test_case().clone();
    let paths = B8DebugBundleOutputPaths::from_dir(&bundle_dir);

    write_text_file(&paths.input_probe_path(), &input_probe_json)?;
    write_binary_file(
        &paths.entry_bytes_bin_path(),
        entry_test_case.x86_bytes().bytes(),
    )?;
    write_json_file(
        &paths.entry_bytes_json_path(),
        &B8DebugEntryBytesReport::real_lc_main_entry(&entry_test_case),
    )?;

    let attempt = B8RealEntryAttempt::run(&entry_test_case, entry_input.program_image_metadata());
    write_json_file(&paths.decode_report_path(), &attempt.decode_report)?;
    write_json_file(&paths.lift_ir_path(), &attempt.lift_ir)?;
    write_json_file(&paths.emit_report_path(), &attempt.emit_report)?;
    write_json_file(&paths.pcmap_path(), &attempt.pcmap)?;
    write_json_file(&paths.fixups_path(), &attempt.fixups)?;
    write_json_file(&paths.helpers_path(), &attempt.helpers)?;
    write_json_file(
        &paths.loader_plan_path(),
        &B8DebugLoaderPlanReport::real_lc_main_attempted(),
    )?;
    write_json_file(&paths.runtime_attempt_path(), &attempt.runtime_report)?;
    write_json_file(&paths.launch_report_path(), &attempt.launch_report)?;
    write_json_file(&paths.blocker_path(), &attempt.blocker_report)?;
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
    launch_report: String,
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
            launch_report: path_string(&bundle_dir.join("launch.report.json")),
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

    fn launch_report_path(&self) -> PathBuf {
        PathBuf::from(&self.launch_report)
    }

    fn blocker_path(&self) -> PathBuf {
        PathBuf::from(&self.blocker)
    }

    fn repro_path(&self) -> PathBuf {
        PathBuf::from(&self.repro)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct B8RealEntryAttempt {
    decode_report: B8DebugDecodeReport,
    lift_ir: B8DebugArtifactReport<FunctionCompiledIrArtifact>,
    emit_report: B8DebugArtifactReport<FunctionArtifactReport>,
    pcmap: B8DebugArtifactReport<FunctionPcMapArtifact>,
    fixups: B8DebugArtifactReport<FunctionFixupsArtifact>,
    helpers: B8DebugArtifactReport<FunctionHelpersArtifact>,
    runtime_report: B8DebugRuntimeAttemptReport,
    launch_report: B8DebugLaunchReport,
    blocker_report: B8DebugBlockerReport,
}

impl B8RealEntryAttempt {
    fn run(test_case: &TestCase, image_metadata: &ProgramImageMetadata) -> Self {
        let decoded_result = decode_function(test_case.x86_bytes());
        let decode_report = B8DebugDecodeReport::from_result(decoded_result.as_ref());
        let decoded = match decoded_result {
            Ok(decoded) => decoded,
            Err(error) => {
                let blocker_report = B8DebugBlockerReport::from_decode_error(&error);
                return Self::blocked_without_ir(
                    test_case,
                    decode_report,
                    None,
                    "decode failed",
                    blocker_report,
                );
            }
        };

        let processed_pc_range = Some(B8DebugProcessedPcRange::from_decoded(&decoded));
        let first_unsupported_instruction =
            B8DebugUnsupportedInstructionReport::from_decoded(&decoded);
        let program =
            match lift_decoded_function_with_image_metadata(&decoded, image_metadata.clone()) {
                Ok(program) => program,
                Err(error) => {
                    let blocker_report = first_unsupported_instruction
                        .as_ref()
                        .map(B8DebugBlockerReport::from_unsupported_instruction)
                        .unwrap_or_else(|| B8DebugBlockerReport::from_lift_error(&error));
                    return Self::blocked_without_ir(
                        test_case,
                        decode_report,
                        processed_pc_range,
                        format!("lift failed: {error:?}"),
                        blocker_report,
                    );
                }
            };

        let lift_ir =
            B8DebugArtifactReport::available(FunctionCompiledIrArtifact::from_program(&program));
        let emitted = match emit_program(&program) {
            Ok(emitted) => emitted,
            Err(error) => {
                let run_error = FunctionRunError::Emit(error);
                let blocker_report = first_unsupported_instruction
                    .as_ref()
                    .map(B8DebugBlockerReport::from_unsupported_instruction)
                    .unwrap_or_else(|| B8DebugBlockerReport::from_function_error(&run_error));
                return Self {
                    decode_report,
                    lift_ir,
                    emit_report: B8DebugArtifactReport::failed(run_error.to_string()),
                    pcmap: B8DebugArtifactReport::skipped("emit failed"),
                    fixups: B8DebugArtifactReport::skipped("emit failed"),
                    helpers: B8DebugArtifactReport::skipped("emit failed"),
                    runtime_report: B8DebugRuntimeAttemptReport::skipped(
                        "emit failed",
                        B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
                    ),
                    launch_report: B8DebugLaunchReport::from_attempt(
                        test_case,
                        processed_pc_range,
                        &blocker_report,
                    ),
                    blocker_report,
                };
            }
        };

        let emit_report = B8DebugArtifactReport::available(
            FunctionArtifactReport::from_source_and_emitted(test_case, &emitted),
        );
        let pcmap =
            B8DebugArtifactReport::available(FunctionPcMapArtifact::from_entries(emitted.pc_map()));
        let fixups = B8DebugArtifactReport::available(FunctionFixupsArtifact::from_fixups(
            emitted.branch_fixups(),
        ));
        let helpers = B8DebugArtifactReport::available(FunctionHelpersArtifact::from_requests(
            emitted.host_trap_requests(),
        ));
        let compiled = FunctionCompileResult::new(program, emitted);
        match run_compiled_test_case_function_with_bundle(test_case, compiled) {
            Ok(bundle) => {
                let blocker_report = B8DebugBlockerReport::none();
                Self {
                    decode_report,
                    lift_ir,
                    emit_report,
                    pcmap,
                    fixups,
                    helpers,
                    runtime_report: B8DebugRuntimeAttemptReport::from_result(
                        bundle.result(),
                        B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
                    ),
                    launch_report: B8DebugLaunchReport::from_attempt(
                        test_case,
                        processed_pc_range,
                        &blocker_report,
                    ),
                    blocker_report,
                }
            }
            Err(error) => {
                let blocker_report = B8DebugBlockerReport::from_function_error(&error);
                Self {
                    decode_report,
                    lift_ir,
                    emit_report,
                    pcmap,
                    fixups,
                    helpers,
                    runtime_report: B8DebugRuntimeAttemptReport::failed(
                        &error,
                        B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
                    ),
                    launch_report: B8DebugLaunchReport::from_attempt(
                        test_case,
                        processed_pc_range,
                        &blocker_report,
                    ),
                    blocker_report,
                }
            }
        }
    }

    fn blocked_without_ir(
        test_case: &TestCase,
        decode_report: B8DebugDecodeReport,
        processed_pc_range: Option<B8DebugProcessedPcRange>,
        reason: impl Into<String>,
        blocker_report: B8DebugBlockerReport,
    ) -> Self {
        let reason = reason.into();
        Self {
            decode_report,
            lift_ir: B8DebugArtifactReport::failed(reason.clone()),
            emit_report: B8DebugArtifactReport::skipped(reason.clone()),
            pcmap: B8DebugArtifactReport::skipped(reason.clone()),
            fixups: B8DebugArtifactReport::skipped(reason.clone()),
            helpers: B8DebugArtifactReport::skipped(reason.clone()),
            runtime_report: B8DebugRuntimeAttemptReport::skipped(
                reason,
                B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
            ),
            launch_report: B8DebugLaunchReport::from_attempt(
                test_case,
                processed_pc_range,
                &blocker_report,
            ),
            blocker_report,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugUnsupportedInstructionReport {
    start: u64,
    end: u64,
    kind: B8DebugDecodedInstructionKindReport,
}

impl B8DebugUnsupportedInstructionReport {
    fn from_decoded(decoded: &DecodedFunction) -> Option<Self> {
        decoded
            .instructions()
            .iter()
            .find(|instruction| {
                matches!(
                    instruction.kind(),
                    DecodedInstructionKind::Unsupported { .. }
                )
            })
            .map(Self::from_instruction)
    }

    fn from_instruction(instruction: &bara_isa_x86::DecodedInstruction) -> Self {
        Self {
            start: instruction.start().value(),
            end: instruction.end().value(),
            kind: B8DebugDecodedInstructionKindReport::from_kind(instruction.kind()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum B8DebugArtifactReport<T> {
    Available { value: T },
    Failed { error: String },
    Skipped { reason: String },
}

impl<T> B8DebugArtifactReport<T> {
    fn available(value: T) -> Self {
        Self::Available { value }
    }

    fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
        }
    }

    fn skipped(reason: impl Into<String>) -> Self {
        Self::Skipped {
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLaunchReport {
    schema: &'static str,
    case_id: String,
    status: B8DebugLaunchStatus,
    entry_source: B8DebugEntrySource,
    source_pc: u64,
    processed_source_pc_range: Option<B8DebugProcessedPcRange>,
    b8_g1_host_trap_path: B8DebugHostTrapPathComparison,
    blocker: B8DebugBlockerReport,
}

impl B8DebugLaunchReport {
    fn from_attempt(
        test_case: &TestCase,
        processed_source_pc_range: Option<B8DebugProcessedPcRange>,
        blocker: &B8DebugBlockerReport,
    ) -> Self {
        Self {
            schema: "b8_debug_real_entry_launch_report_v0",
            case_id: test_case.case_id().as_str().to_owned(),
            status: B8DebugLaunchStatus::from_blocker_status(blocker.status()),
            entry_source: B8DebugEntrySource::PublicLcMainEntryoff,
            source_pc: test_case.x86_bytes().entry().value(),
            processed_source_pc_range,
            b8_g1_host_trap_path: B8DebugHostTrapPathComparison::NotUsed,
            blocker: blocker.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLaunchStatus {
    Blocked,
    CompletedWithoutCurrentBlocker,
}

impl B8DebugLaunchStatus {
    const fn from_blocker_status(status: B8DebugBlockerStatus) -> Self {
        match status {
            B8DebugBlockerStatus::Blocked => Self::Blocked,
            B8DebugBlockerStatus::NoCurrentBlocker => Self::CompletedWithoutCurrentBlocker,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugProcessedPcRange {
    start: u64,
    end: u64,
}

impl B8DebugProcessedPcRange {
    fn from_decoded(decoded: &DecodedFunction) -> Self {
        let start = decoded.entry().value();
        let end = decoded
            .instructions()
            .last()
            .map(|instruction| instruction.end().value())
            .unwrap_or(start);
        Self { start, end }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHostTrapPathComparison {
    NotUsed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugEntryBytesReport {
    schema: &'static str,
    case_id: String,
    source: B8DebugEntrySource,
    source_isa: B8DebugSourceIsa,
    source_pc: u64,
    byte_len: usize,
    bytes_hex: String,
}

impl B8DebugEntryBytesReport {
    fn real_lc_main_entry(test_case: &TestCase) -> Self {
        Self {
            schema: "b8_debug_entry_bytes_v0",
            case_id: test_case.case_id().as_str().to_owned(),
            source: B8DebugEntrySource::PublicLcMainEntryoff,
            source_isa: B8DebugSourceIsa::X8664,
            source_pc: test_case.x86_bytes().entry().value(),
            byte_len: test_case.x86_bytes().bytes().len(),
            bytes_hex: encode_lower_hex(test_case.x86_bytes().bytes()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugEntrySource {
    PublicLcMainEntryoff,
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
    entry: Option<u64>,
    instructions: Vec<B8DebugDecodedInstructionReport>,
    error: Option<String>,
}

impl B8DebugDecodeReport {
    fn from_result(decoded: Result<&DecodedFunction, &DecodeError>) -> Self {
        match decoded {
            Ok(decoded) => Self {
                schema: "b8_debug_decode_report_v0",
                status: B8DebugStageStatus::Executed,
                entry: Some(decoded.entry().value()),
                instructions: decoded
                    .instructions()
                    .iter()
                    .map(B8DebugDecodedInstructionReport::from_instruction)
                    .collect(),
                error: None,
            },
            Err(error) => Self {
                schema: "b8_debug_decode_report_v0",
                status: B8DebugStageStatus::Failed,
                entry: None,
                instructions: Vec::new(),
                error: Some(format!("{error:?}")),
            },
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
    MovRbxRax,
    MovRaxQwordPtrRipRelative {
        displacement: String,
        address: u64,
        width: B8DebugMemoryReadWidthReport,
    },
    MovRdxQwordPtrRax,
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
    MovRbpRsp,
    PushRax,
    PushRbx,
    PushRbp,
    PushR14,
    PushR15,
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
            DecodedInstructionKind::MovRbxRax => Self::MovRbxRax,
            DecodedInstructionKind::MovRaxQwordPtrRipRelative {
                displacement,
                address,
            } => Self::MovRaxQwordPtrRipRelative {
                displacement: format!("{displacement:?}"),
                address: address.value(),
                width: B8DebugMemoryReadWidthReport::Bits64,
            },
            DecodedInstructionKind::MovRdxQwordPtrRax => Self::MovRdxQwordPtrRax,
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
            DecodedInstructionKind::MovRbpRsp => Self::MovRbpRsp,
            DecodedInstructionKind::PushRax => Self::PushRax,
            DecodedInstructionKind::PushRbx => Self::PushRbx,
            DecodedInstructionKind::PushRbp => Self::PushRbp,
            DecodedInstructionKind::PushR14 => Self::PushR14,
            DecodedInstructionKind::PushR15 => Self::PushR15,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugMemoryReadWidthReport {
    Bits64,
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
    const fn real_lc_main_attempted() -> Self {
        Self {
            schema: "b8_debug_loader_plan_v0",
            source: "bara_runtime_user_space_launch_plan",
            status: B8DebugStageStatus::Executed,
            input_metadata: B8DebugLoaderInputMetadata::PublicMachOProbe,
            entry_source_for_this_bundle: B8DebugEntrySource::PublicLcMainEntryoff,
            next_entry_source: B8DebugLoaderNextEntrySource::FirstUnsupportedBoundary,
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
    FirstUnsupportedBoundary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugRuntimeAttemptReport {
    schema: &'static str,
    status: B8DebugStageStatus,
    run_scope: B8DebugRuntimeRunScope,
    return_value: Option<u64>,
    stdout: Option<String>,
    error: Option<String>,
}

impl B8DebugRuntimeAttemptReport {
    fn from_result(result: &FunctionRunResult, run_scope: B8DebugRuntimeRunScope) -> Self {
        Self {
            schema: "b8_debug_runtime_attempt_v0",
            status: B8DebugStageStatus::Executed,
            run_scope,
            return_value: Some(result.return_value()),
            stdout: Some(result.stdout().to_owned()),
            error: None,
        }
    }

    fn skipped(reason: impl Into<String>, run_scope: B8DebugRuntimeRunScope) -> Self {
        Self {
            schema: "b8_debug_runtime_attempt_v0",
            status: B8DebugStageStatus::Skipped,
            run_scope,
            return_value: None,
            stdout: None,
            error: Some(reason.into()),
        }
    }

    fn failed(error: &FunctionRunError, run_scope: B8DebugRuntimeRunScope) -> Self {
        Self {
            schema: "b8_debug_runtime_attempt_v0",
            status: B8DebugStageStatus::Failed,
            run_scope,
            return_value: None,
            stdout: None,
            error: Some(error.to_string()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugRuntimeRunScope {
    RealLcMainEntryFirstBlock,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugBlockerReport {
    schema: &'static str,
    status: B8DebugBlockerStatus,
    current_blocker: B8DebugBlocker,
    failure_kind: Option<FailureKind>,
    unsupported_instruction: Option<B8DebugUnsupportedInstructionReport>,
    message: Option<String>,
    next_action: B8DebugNextAction,
}

impl B8DebugBlockerReport {
    fn none() -> Self {
        Self {
            schema: "b8_debug_blocker_v0",
            status: B8DebugBlockerStatus::NoCurrentBlocker,
            current_blocker: B8DebugBlocker::None,
            failure_kind: None,
            unsupported_instruction: None,
            message: None,
            next_action: B8DebugNextAction::InspectNextDebugBundleBlocker,
        }
    }

    fn from_decode_error(error: &DecodeError) -> Self {
        Self::blocked(
            B8DebugBlocker::DecodeError,
            FailureKind::DecodeError,
            format!("{error:?}"),
        )
    }

    fn from_lift_error(error: &LiftError) -> Self {
        Self::blocked(
            B8DebugBlocker::LiftError,
            FailureKind::LiftError,
            format!("{error:?}"),
        )
    }

    fn from_unsupported_instruction(instruction: &B8DebugUnsupportedInstructionReport) -> Self {
        Self {
            schema: "b8_debug_blocker_v0",
            status: B8DebugBlockerStatus::Blocked,
            current_blocker: B8DebugBlocker::UnsupportedInstruction,
            failure_kind: Some(FailureKind::UnsupportedInstruction),
            unsupported_instruction: Some(instruction.clone()),
            message: Some(format!("{:?}", instruction.kind)),
            next_action: B8DebugNextAction::AdvanceToNextIsaBlocker,
        }
    }

    fn from_function_error(error: &FunctionRunError) -> Self {
        Self::blocked(
            B8DebugBlocker::from_failure_kind(error.failure_kind()),
            error.failure_kind(),
            error.to_string(),
        )
    }

    fn blocked(
        current_blocker: B8DebugBlocker,
        failure_kind: FailureKind,
        message: String,
    ) -> Self {
        Self {
            schema: "b8_debug_blocker_v0",
            status: B8DebugBlockerStatus::Blocked,
            current_blocker,
            failure_kind: Some(failure_kind),
            unsupported_instruction: None,
            message: Some(message),
            next_action: B8DebugNextAction::AdvanceToNextIsaBlocker,
        }
    }

    const fn status(&self) -> B8DebugBlockerStatus {
        self.status
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugBlockerStatus {
    Blocked,
    NoCurrentBlocker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugBlocker {
    None,
    DecodeError,
    LiftError,
    UnsupportedInstruction,
    EmitError,
    RunError,
}

impl B8DebugBlocker {
    const fn from_failure_kind(failure_kind: FailureKind) -> Self {
        match failure_kind {
            FailureKind::DecodeError => Self::DecodeError,
            FailureKind::LiftError => Self::LiftError,
            FailureKind::UnsupportedInstruction => Self::UnsupportedInstruction,
            FailureKind::RunError => Self::RunError,
            FailureKind::InvalidTestCase
            | FailureKind::MissingExpected
            | FailureKind::InvalidExpected
            | FailureKind::EmitError
            | FailureKind::ComparisonMismatch
            | FailureKind::WrongReturnValue
            | FailureKind::WrongRegisterValue
            | FailureKind::WrongFlags
            | FailureKind::WrongMemory
            | FailureKind::WrongBranchTarget
            | FailureKind::WrongCallReturn
            | FailureKind::WrongExternalCall
            | FailureKind::RunnerCrash
            | FailureKind::OracleCrash => Self::EmitError,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugNextAction {
    AdvanceToNextIsaBlocker,
    InspectNextDebugBundleBlocker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugStageStatus {
    Executed,
    Failed,
    Skipped,
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
    Entry(MachOEntryFunctionTestCaseError),
    B8CaseId(X8664MachOFixtureError),
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
            Self::Entry(error) => {
                write!(formatter, "B8 debug entry extraction failed: {error:?}")
            }
            Self::B8CaseId(error) => write!(formatter, "B8 debug case id error: {error}"),
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

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
