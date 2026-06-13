use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use bara_arm64::{emit_program, EmitError};
use bara_ir::{
    Program, ProgramImageMappedBytes, ProgramImageMetadata, Terminator, UnsupportedReason, X86Va,
};
use bara_isa_x86::{
    decode_function, lift_decoded_function_with_image_metadata, DecodeError, DecodedFunction,
    DecodedInstructionKind, LiftError, X86Bytes,
};
use bara_oracle::{
    binary_format_probe_report_to_json, decode_mach_o_chained_fixups_for_target,
    mach_o_entry_function_input, probe_public_binary_format, BinaryFileBytes,
    BinaryFormatProbeError, BinaryFormatProbeReport, BinaryInput, FailureKind, JsonError,
    MachOChainedFixupTargetAddress, MachOChainedFixupsBlocker, MachOChainedFixupsTargetReport,
    MachOChainedImportIdentityReport, MachOChainedRebaseTargetIdentityReport,
    MachODyldInfoCommandKind, MachODylibImportCommandKind, MachOEntryFunctionInput,
    MachOEntryFunctionTestCaseError, MachOLinkeditDataCommandKind, TestCase,
};
use serde::{Deserialize, Serialize};

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
    let loader_plan = B8DebugLoaderPlanReport::real_lc_main_attempted(
        &input,
        &entry_input,
        &input_probe,
        &attempt.decode_report,
    );
    let launch_report = attempt
        .launch_report
        .with_helper_boundary_request(loader_plan.import_boundary.helper_boundary_request.clone());
    write_json_file(&paths.loader_plan_path(), &loader_plan)?;
    write_json_file(&paths.runtime_attempt_path(), &attempt.runtime_report)?;
    write_json_file(&paths.launch_report_path(), &launch_report)?;
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
        if let Some(reason) = frontier_unsupported_terminator_reason(&program) {
            let run_error = FunctionRunError::Emit(EmitError::UnsupportedIr {
                reason: reason.clone(),
            });
            let blocker_report = B8DebugBlockerReport::from_function_error(&run_error);
            return Self {
                decode_report,
                lift_ir,
                emit_report: B8DebugArtifactReport::failed(run_error.to_string()),
                pcmap: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                fixups: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                helpers: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                runtime_report: B8DebugRuntimeAttemptReport::skipped(
                    "unsupported IR terminator",
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

fn frontier_unsupported_terminator_reason(program: &Program) -> Option<&UnsupportedReason> {
    program
        .blocks()
        .iter()
        .rev()
        .find_map(|block| match block.terminator() {
            Terminator::Unsupported { reason } => Some(reason),
            Terminator::Return
            | Terminator::BoundaryRequest { .. }
            | Terminator::Fallthrough { .. }
            | Terminator::DirectJump { .. }
            | Terminator::DirectCall { .. }
            | Terminator::CondJump { .. } => None,
        })
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
    helper_boundary_request: B8DebugHelperBoundaryRequestReport,
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
            helper_boundary_request: B8DebugHelperBoundaryRequestReport::skipped(),
            blocker: blocker.clone(),
        }
    }

    fn with_helper_boundary_request(
        mut self,
        helper_boundary_request: B8DebugHelperBoundaryRequestReport,
    ) -> Self {
        self.helper_boundary_request = helper_boundary_request;
        self
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

    fn register_indirect_call_r14_boundary(
        &self,
    ) -> Option<B8DebugRegisterIndirectCallBoundaryReport> {
        self.instructions.iter().find_map(|instruction| {
            let B8DebugDecodedInstructionKindReport::CallR14 { return_to } = &instruction.kind
            else {
                return None;
            };

            Some(B8DebugRegisterIndirectCallBoundaryReport {
                target_register: B8DebugRegisterName::R14,
                call_site: instruction.start,
                return_to: *return_to,
            })
        })
    }

    fn last_r14_load_before(&self, call_site: u64) -> Option<B8DebugTargetPointerLoadReport> {
        self.instructions
            .iter()
            .rev()
            .find_map(|instruction| match &instruction.kind {
                _ if instruction.start >= call_site => None,
                B8DebugDecodedInstructionKindReport::MovR14QwordPtrRipRelative {
                    address,
                    width,
                    ..
                } => Some(B8DebugTargetPointerLoadReport {
                    kind: B8DebugTargetPointerLoadKind::RipRelativeQwordLoad,
                    target_register: B8DebugRegisterName::R14,
                    address: *address,
                    width: *width,
                }),
                _ => None,
            })
    }

    fn last_register_materialization_before(
        &self,
        register: B8DebugRegisterName,
        call_site: u64,
    ) -> Option<B8DebugRegisterMaterializationSourceReport> {
        self.instructions
            .iter()
            .rev()
            .find_map(|instruction| match (&instruction.kind, register) {
                _ if instruction.start >= call_site => None,
                (
                    B8DebugDecodedInstructionKindReport::MovRdiQwordPtrRipRelative {
                        address,
                        width,
                        ..
                    },
                    B8DebugRegisterName::Rdi,
                ) => Some(
                    B8DebugRegisterMaterializationSourceReport::rip_relative_qword_load(
                        instruction,
                        B8DebugRegisterName::Rdi,
                        *address,
                        *width,
                    ),
                ),
                (
                    B8DebugDecodedInstructionKindReport::MovRsiQwordPtrRipRelative {
                        address,
                        width,
                        ..
                    },
                    B8DebugRegisterName::Rsi,
                ) => Some(
                    B8DebugRegisterMaterializationSourceReport::rip_relative_qword_load(
                        instruction,
                        B8DebugRegisterName::Rsi,
                        *address,
                        *width,
                    ),
                ),
                (
                    B8DebugDecodedInstructionKindReport::LeaRdiRipRelative { address, .. },
                    B8DebugRegisterName::Rdi,
                ) => Some(
                    B8DebugRegisterMaterializationSourceReport::rip_relative_address(
                        instruction,
                        B8DebugRegisterName::Rdi,
                        *address,
                    ),
                ),
                (
                    B8DebugDecodedInstructionKindReport::LeaRsiRipRelative { address, .. },
                    B8DebugRegisterName::Rsi,
                ) => Some(
                    B8DebugRegisterMaterializationSourceReport::rip_relative_address(
                        instruction,
                        B8DebugRegisterName::Rsi,
                        *address,
                    ),
                ),
                _ => None,
            })
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
    MovRdiQwordPtrRipRelative {
        displacement: String,
        address: u64,
        width: B8DebugMemoryReadWidthReport,
    },
    MovRsiQwordPtrRipRelative {
        displacement: String,
        address: u64,
        width: B8DebugMemoryReadWidthReport,
    },
    MovR14QwordPtrRipRelative {
        displacement: String,
        address: u64,
        width: B8DebugMemoryReadWidthReport,
    },
    MovR15QwordPtrRipRelative {
        displacement: String,
        address: u64,
        width: B8DebugMemoryReadWidthReport,
    },
    MovRdiQwordPtrR15,
    MovRdxQwordPtrRax,
    LeaRdiRipRelative {
        displacement: String,
        address: u64,
    },
    LeaRsiRipRelative {
        displacement: String,
        address: u64,
    },
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
    XorEdxEdx,
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
    CallR14 {
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
            DecodedInstructionKind::MovRdiQwordPtrRipRelative {
                displacement,
                address,
            } => Self::MovRdiQwordPtrRipRelative {
                displacement: format!("{displacement:?}"),
                address: address.value(),
                width: B8DebugMemoryReadWidthReport::Bits64,
            },
            DecodedInstructionKind::MovRsiQwordPtrRipRelative {
                displacement,
                address,
            } => Self::MovRsiQwordPtrRipRelative {
                displacement: format!("{displacement:?}"),
                address: address.value(),
                width: B8DebugMemoryReadWidthReport::Bits64,
            },
            DecodedInstructionKind::MovR14QwordPtrRipRelative {
                displacement,
                address,
            } => Self::MovR14QwordPtrRipRelative {
                displacement: format!("{displacement:?}"),
                address: address.value(),
                width: B8DebugMemoryReadWidthReport::Bits64,
            },
            DecodedInstructionKind::MovR15QwordPtrRipRelative {
                displacement,
                address,
            } => Self::MovR15QwordPtrRipRelative {
                displacement: format!("{displacement:?}"),
                address: address.value(),
                width: B8DebugMemoryReadWidthReport::Bits64,
            },
            DecodedInstructionKind::MovRdiQwordPtrR15 => Self::MovRdiQwordPtrR15,
            DecodedInstructionKind::MovRdxQwordPtrRax => Self::MovRdxQwordPtrRax,
            DecodedInstructionKind::LeaRdiRipRelative {
                displacement,
                address,
            } => Self::LeaRdiRipRelative {
                displacement: format!("{displacement:?}"),
                address: address.value(),
            },
            DecodedInstructionKind::LeaRsiRipRelative {
                displacement,
                address,
            } => Self::LeaRsiRipRelative {
                displacement: format!("{displacement:?}"),
                address: address.value(),
            },
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
            DecodedInstructionKind::XorEdxEdx => Self::XorEdxEdx,
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
            DecodedInstructionKind::CallR14 { return_to } => Self::CallR14 {
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
    image_mapping: B8DebugLoaderImageMappingReport,
    relocation_binding: B8DebugLoaderDeferredStepReport,
    import_boundary: B8DebugImportBoundaryReport,
    entry_source_for_this_bundle: B8DebugEntrySource,
    next_entry_source: B8DebugLoaderNextEntrySource,
}

impl B8DebugLoaderPlanReport {
    fn real_lc_main_attempted(
        input: &BinaryInput,
        entry_input: &MachOEntryFunctionInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
    ) -> Self {
        let code = entry_input.executable_image().code_segment().x86_bytes();
        Self {
            schema: "b8_debug_loader_plan_v0",
            source: "bara_runtime_user_space_launch_plan",
            status: B8DebugStageStatus::Executed,
            input_metadata: B8DebugLoaderInputMetadata::PublicMachOProbe,
            image_mapping: B8DebugLoaderImageMappingReport {
                status: B8DebugStageStatus::Executed,
                segment_source: B8DebugLoaderSegmentSource::LcSegment64FileRange,
                address_space: B8DebugLoaderAddressSpace::MachOVirtualAddress,
                code_segment_vmaddr: code.entry().value(),
                code_segment_byte_len: code.bytes().len(),
                entry_pc: entry_input.executable_image().entry().offset().value(),
                mapped_bytes_source: B8DebugLoaderMappedBytesSource::ProgramImageMetadata,
            },
            relocation_binding: B8DebugLoaderDeferredStepReport {
                status: B8DebugStageStatus::Skipped,
                reason: "public rebase/bind/import application is represented as import_boundary and remains blocked until chained fixups are decoded",
                next_action: B8DebugLoaderDeferredAction::ResolvePublicRebaseBindImports,
            },
            import_boundary: B8DebugImportBoundaryReport::from_probe_and_decode_report(
                input,
                input_probe,
                decode_report,
                code,
                entry_input.program_image_metadata(),
            ),
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLoaderImageMappingReport {
    status: B8DebugStageStatus,
    segment_source: B8DebugLoaderSegmentSource,
    address_space: B8DebugLoaderAddressSpace,
    code_segment_vmaddr: u64,
    code_segment_byte_len: usize,
    entry_pc: u64,
    mapped_bytes_source: B8DebugLoaderMappedBytesSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderSegmentSource {
    LcSegment64FileRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderAddressSpace {
    MachOVirtualAddress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderMappedBytesSource {
    ProgramImageMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLoaderDeferredStepReport {
    status: B8DebugStageStatus,
    reason: &'static str,
    next_action: B8DebugLoaderDeferredAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugLoaderDeferredAction {
    ResolvePublicRebaseBindImports,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugImportBoundaryReport {
    status: B8DebugImportBoundaryStatus,
    call_boundary: Option<B8DebugRegisterIndirectCallBoundaryReport>,
    target_pointer_load: Option<B8DebugTargetPointerLoadReport>,
    public_metadata: B8DebugPublicImportMetadataReport,
    chained_fixups: Option<MachOChainedFixupsTargetReport>,
    helper_boundary_request: B8DebugHelperBoundaryRequestReport,
    resolution: B8DebugImportBoundaryResolution,
    next_action: B8DebugImportBoundaryNextAction,
}

impl B8DebugImportBoundaryReport {
    fn from_probe_and_decode_report(
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        code_bytes: &X86Bytes,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let public_metadata = B8DebugPublicImportMetadataReport::from_probe(input_probe);
        let call_boundary = decode_report.register_indirect_call_r14_boundary();
        let target_pointer_load = call_boundary
            .as_ref()
            .and_then(|boundary| decode_report.last_r14_load_before(boundary.call_site));
        let chained_fixups = target_pointer_load.as_ref().map(|target| {
            decode_mach_o_chained_fixups_for_target(
                input,
                input_probe.metadata().mach_o_metadata(),
                MachOChainedFixupTargetAddress::from_mach_o_virtual_address(target.address),
            )
        });

        if let Some(call_boundary_report) = call_boundary {
            let resolved_import_identity = chained_fixups
                .as_ref()
                .and_then(MachOChainedFixupsTargetReport::resolved_import_identity);
            let (resolution, next_action, helper_boundary_request) =
                if public_metadata.has_chained_fixups() {
                    if let Some(import_identity) = resolved_import_identity {
                        (
                        B8DebugImportBoundaryResolution::ResolvedPublicDyldChainedFixupsImport,
                        B8DebugImportBoundaryNextAction::DefineObjcReceiverSelectorMaterialization,
                        B8DebugHelperBoundaryRequestReport::blocked_import_helper_call(
                            call_boundary_report,
                            import_identity,
                            input,
                            input_probe,
                            decode_report,
                            code_bytes,
                            image_metadata,
                        ),
                    )
                    } else {
                        (
                            B8DebugImportBoundaryResolution::RequiresPublicDyldChainedFixupsDecoder,
                            B8DebugImportBoundaryNextAction::DecodePublicDyldChainedFixupsImports,
                            B8DebugHelperBoundaryRequestReport::blocked(
                                B8DebugHelperBoundaryBlockedReason::ImportSymbolIdentityUnresolved,
                            ),
                        )
                    }
                } else if public_metadata.has_dyld_info_bind_ranges() {
                    (
                        B8DebugImportBoundaryResolution::RequiresPublicDyldBindOpcodeDecoder,
                        B8DebugImportBoundaryNextAction::DecodePublicDyldBindOpcodes,
                        B8DebugHelperBoundaryRequestReport::blocked(
                            B8DebugHelperBoundaryBlockedReason::ImportSymbolIdentityUnresolved,
                        ),
                    )
                } else {
                    (
                        B8DebugImportBoundaryResolution::MissingPublicBindMetadata,
                        B8DebugImportBoundaryNextAction::InspectUnsupportedLoaderMetadata,
                        B8DebugHelperBoundaryRequestReport::blocked(
                            B8DebugHelperBoundaryBlockedReason::ImportSymbolIdentityUnresolved,
                        ),
                    )
                };

            return Self {
                status: B8DebugImportBoundaryStatus::Blocked,
                call_boundary: Some(call_boundary_report),
                target_pointer_load,
                public_metadata,
                chained_fixups,
                helper_boundary_request,
                resolution,
                next_action,
            };
        }

        Self {
            status: B8DebugImportBoundaryStatus::Skipped,
            call_boundary,
            target_pointer_load,
            public_metadata,
            chained_fixups,
            helper_boundary_request: B8DebugHelperBoundaryRequestReport::skipped(),
            resolution: B8DebugImportBoundaryResolution::NoRegisterIndirectCallBoundary,
            next_action: B8DebugImportBoundaryNextAction::InspectNextDebugBundleBlocker,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugImportBoundaryStatus {
    Blocked,
    Executed,
    Skipped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugRegisterIndirectCallBoundaryReport {
    target_register: B8DebugRegisterName,
    call_site: u64,
    return_to: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugTargetPointerLoadReport {
    kind: B8DebugTargetPointerLoadKind,
    target_register: B8DebugRegisterName,
    address: u64,
    width: B8DebugMemoryReadWidthReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugTargetPointerLoadKind {
    RipRelativeQwordLoad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugRegisterName {
    Rax,
    Rdx,
    Rdi,
    Rsi,
    R14,
    R15,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugPublicImportMetadataReport {
    dylib_imports: Vec<B8DebugDylibImportReport>,
    dyld_info: Vec<B8DebugDyldInfoReport>,
    linkedit_data: Vec<B8DebugLinkeditDataReport>,
    symbol_table_count: usize,
    dynamic_symbol_table_count: usize,
}

impl B8DebugPublicImportMetadataReport {
    fn from_probe(input_probe: &BinaryFormatProbeReport) -> Self {
        let summary = input_probe
            .metadata()
            .mach_o_metadata()
            .load_commands()
            .summary();
        Self {
            dylib_imports: summary
                .recognized_dylib_imports()
                .iter()
                .map(B8DebugDylibImportReport::from_metadata)
                .collect(),
            dyld_info: summary
                .recognized_dyld_info()
                .iter()
                .map(B8DebugDyldInfoReport::from_metadata)
                .collect(),
            linkedit_data: summary
                .recognized_linkedit_data()
                .iter()
                .map(B8DebugLinkeditDataReport::from_metadata)
                .collect(),
            symbol_table_count: summary.recognized_symbol_tables().len(),
            dynamic_symbol_table_count: summary.recognized_dynamic_symbol_tables().len(),
        }
    }

    fn has_chained_fixups(&self) -> bool {
        self.linkedit_data
            .iter()
            .any(|metadata| metadata.command == MachOLinkeditDataCommandKind::DyldChainedFixups)
    }

    fn has_dyld_info_bind_ranges(&self) -> bool {
        self.dyld_info.iter().any(|metadata| {
            metadata.bind.byte_size > 0
                || metadata.weak_bind.byte_size > 0
                || metadata.lazy_bind.byte_size > 0
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugDylibImportReport {
    command: MachODylibImportCommandKind,
    path: String,
}

impl B8DebugDylibImportReport {
    fn from_metadata(metadata: &bara_oracle::RecognizedMachODylibImportCommand) -> Self {
        Self {
            command: metadata.command(),
            path: metadata.name().as_str().to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugDyldInfoReport {
    command: MachODyldInfoCommandKind,
    rebase: B8DebugLinkeditDataRangeReport,
    bind: B8DebugLinkeditDataRangeReport,
    weak_bind: B8DebugLinkeditDataRangeReport,
    lazy_bind: B8DebugLinkeditDataRangeReport,
    export: B8DebugLinkeditDataRangeReport,
}

impl B8DebugDyldInfoReport {
    fn from_metadata(metadata: &bara_oracle::RecognizedMachODyldInfoCommand) -> Self {
        Self {
            command: metadata.command(),
            rebase: B8DebugLinkeditDataRangeReport::from_metadata(metadata.rebase()),
            bind: B8DebugLinkeditDataRangeReport::from_metadata(metadata.bind()),
            weak_bind: B8DebugLinkeditDataRangeReport::from_metadata(metadata.weak_bind()),
            lazy_bind: B8DebugLinkeditDataRangeReport::from_metadata(metadata.lazy_bind()),
            export: B8DebugLinkeditDataRangeReport::from_metadata(metadata.export()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLinkeditDataReport {
    command: MachOLinkeditDataCommandKind,
    dataoff: u32,
    datasize: u32,
}

impl B8DebugLinkeditDataReport {
    fn from_metadata(metadata: &bara_oracle::RecognizedMachOLinkeditDataCommand) -> Self {
        Self {
            command: metadata.command(),
            dataoff: metadata.dataoff().as_u32(),
            datasize: metadata.datasize().as_u32(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugLinkeditDataRangeReport {
    offset: u32,
    byte_size: u32,
}

impl B8DebugLinkeditDataRangeReport {
    fn from_metadata(metadata: bara_oracle::MachOLinkeditDataRange) -> Self {
        Self {
            offset: metadata.offset().as_u32(),
            byte_size: metadata.byte_size().as_u32(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugHelperBoundaryRequestReport {
    status: B8DebugImportBoundaryStatus,
    reason: Option<B8DebugHelperBoundaryBlockedReason>,
    request: Option<B8DebugImportHelperRequestReport>,
    blockers: Vec<B8DebugHelperBoundaryBlocker>,
}

impl B8DebugHelperBoundaryRequestReport {
    fn blocked(reason: B8DebugHelperBoundaryBlockedReason) -> Self {
        let blockers = B8DebugHelperBoundaryBlocker::from_reason(reason);
        Self {
            status: B8DebugImportBoundaryStatus::Blocked,
            reason: Some(reason),
            request: None,
            blockers,
        }
    }

    fn blocked_import_helper_call(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        import_identity: MachOChainedImportIdentityReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        code_bytes: &X86Bytes,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let request = B8DebugImportHelperRequestReport::from_boundary_and_import(
            call_boundary,
            import_identity,
            input,
            input_probe,
            decode_report,
            code_bytes,
            image_metadata,
        );
        let reason = request.boundary_blocked_reason();
        let blockers = request.boundary_blockers();
        Self {
            status: B8DebugImportBoundaryStatus::Blocked,
            reason,
            request: Some(request),
            blockers,
        }
    }

    fn skipped() -> Self {
        Self {
            status: B8DebugImportBoundaryStatus::Skipped,
            reason: None,
            request: None,
            blockers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugImportHelperRequestReport {
    kind: B8DebugImportHelperRequestKind,
    source: B8DebugImportHelperRequestSource,
    source_isa: B8DebugSourceIsa,
    target_register: B8DebugRegisterName,
    call_site: u64,
    return_to: u64,
    import: MachOChainedImportIdentityReport,
    required_marshaling: B8DebugHelperMarshalingReport,
    helper_execution_request: Option<B8DebugObjcHelperExecutionRequestReport>,
}

impl B8DebugImportHelperRequestReport {
    fn from_boundary_and_import(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        import: MachOChainedImportIdentityReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        code_bytes: &X86Bytes,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let required_marshaling = B8DebugHelperMarshalingReport::blocked(
            call_boundary,
            input,
            input_probe,
            decode_report,
            image_metadata,
        );
        let helper_execution_request =
            B8DebugObjcHelperExecutionRequestReport::from_import_and_marshaling(
                call_boundary,
                &import,
                &required_marshaling,
                input,
                input_probe,
                code_bytes,
                image_metadata,
            );
        Self {
            kind: B8DebugImportHelperRequestKind::ImportHelperCall,
            source: B8DebugImportHelperRequestSource::PublicDyldChainedFixupsImport,
            source_isa: B8DebugSourceIsa::X8664,
            target_register: call_boundary.target_register,
            call_site: call_boundary.call_site,
            return_to: call_boundary.return_to,
            import,
            required_marshaling,
            helper_execution_request,
        }
    }

    fn boundary_blocked_reason(&self) -> Option<B8DebugHelperBoundaryBlockedReason> {
        self.helper_execution_request
            .as_ref()
            .and_then(B8DebugObjcHelperExecutionRequestReport::boundary_blocked_reason)
            .or(Some(
                B8DebugHelperBoundaryBlockedReason::ImportHelperMarshalingUnimplemented,
            ))
    }

    fn boundary_blockers(&self) -> Vec<B8DebugHelperBoundaryBlocker> {
        self.helper_execution_request
            .as_ref()
            .map(B8DebugObjcHelperExecutionRequestReport::boundary_blockers)
            .unwrap_or_else(|| self.required_marshaling.blockers.clone())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugImportHelperRequestKind {
    ImportHelperCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugImportHelperRequestSource {
    PublicDyldChainedFixupsImport,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugHelperMarshalingReport {
    status: B8DebugImportBoundaryStatus,
    argument_model: B8DebugHelperArgumentModel,
    return_model: B8DebugHelperReturnModel,
    contract: Option<B8DebugImportHelperMarshalingContractReport>,
    blockers: Vec<B8DebugHelperBoundaryBlocker>,
}

impl B8DebugHelperMarshalingReport {
    fn blocked(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let contract = B8DebugImportHelperMarshalingContractReport::blocked(
            call_boundary,
            input,
            input_probe,
            decode_report,
            image_metadata,
        );
        let blockers = contract.blockers.clone();
        Self {
            status: B8DebugImportBoundaryStatus::Blocked,
            argument_model: B8DebugHelperArgumentModel::X8664CallArguments,
            return_model: B8DebugHelperReturnModel::X8664RaxReturnValue,
            contract: Some(contract),
            blockers,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugImportHelperMarshalingContractReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    calling_convention: B8DebugHelperCallingConvention,
    argument_sources: Vec<B8DebugHelperArgumentSourceReport>,
    return_destination: B8DebugHelperReturnDestinationReport,
    materialization_boundary: B8DebugObjcMessageMaterializationBoundaryReport,
    blockers: Vec<B8DebugHelperBoundaryBlocker>,
    next_action: B8DebugHelperMarshalingNextAction,
}

impl B8DebugImportHelperMarshalingContractReport {
    fn blocked(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let materialization_boundary = B8DebugObjcMessageMaterializationBoundaryReport::blocked(
            call_boundary.call_site,
            input,
            input_probe,
            decode_report,
            image_metadata,
        );
        let receiver_materialized = materialization_boundary
            .receiver
            .is_resolved_for_helper_argument();
        let selector_materialized = materialization_boundary
            .selector
            .is_resolved_for_helper_argument();
        let mut blockers = Vec::new();
        if !receiver_materialized {
            blockers.push(B8DebugHelperBoundaryBlocker::ObjcReceiverMaterializationUnimplemented);
        }
        if !selector_materialized {
            blockers.push(B8DebugHelperBoundaryBlocker::ObjcSelectorMaterializationUnimplemented);
        }
        blockers.push(
            B8DebugHelperBoundaryBlocker::from_objc_materialization_blocker(
                materialization_boundary.return_value.blocker,
            ),
        );
        let next_action = B8DebugHelperMarshalingNextAction::from_materialization_next_action(
            materialization_boundary.next_action,
        );
        Self {
            schema: "b8_import_helper_marshaling_contract_v0",
            status: B8DebugImportBoundaryStatus::Blocked,
            calling_convention: B8DebugHelperCallingConvention::X8664MacosSystemV,
            argument_sources: vec![
                B8DebugHelperArgumentSourceReport::register_argument(
                    0,
                    B8DebugHelperArgumentRole::ObjcReceiver,
                    B8DebugRegisterName::Rdi,
                    receiver_materialized,
                ),
                B8DebugHelperArgumentSourceReport::register_argument(
                    1,
                    B8DebugHelperArgumentRole::ObjcSelector,
                    B8DebugRegisterName::Rsi,
                    selector_materialized,
                ),
            ],
            return_destination: B8DebugHelperReturnDestinationReport::register_return(
                B8DebugRegisterName::Rax,
            ),
            materialization_boundary,
            blockers,
            next_action,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperCallingConvention {
    #[serde(rename = "x86_64_macos_system_v")]
    X8664MacosSystemV,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugHelperArgumentSourceReport {
    position: u8,
    role: B8DebugHelperArgumentRole,
    source: B8DebugHelperValueSourceReport,
    materialization: B8DebugHelperMaterializationReport,
}

impl B8DebugHelperArgumentSourceReport {
    const fn register_argument(
        position: u8,
        role: B8DebugHelperArgumentRole,
        register: B8DebugRegisterName,
        materialized: bool,
    ) -> Self {
        Self {
            position,
            role,
            source: B8DebugHelperValueSourceReport::register(register),
            materialization: B8DebugHelperMaterializationReport::from_status(
                materialized,
                match role {
                    B8DebugHelperArgumentRole::ObjcReceiver => {
                        B8DebugHelperBoundaryBlocker::ObjcReceiverMaterializationUnimplemented
                    }
                    B8DebugHelperArgumentRole::ObjcSelector => {
                        B8DebugHelperBoundaryBlocker::ObjcSelectorMaterializationUnimplemented
                    }
                },
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperArgumentRole {
    ObjcReceiver,
    ObjcSelector,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugHelperReturnDestinationReport {
    role: B8DebugHelperReturnRole,
    destination: B8DebugHelperValueSourceReport,
    materialization: B8DebugHelperMaterializationReport,
}

impl B8DebugHelperReturnDestinationReport {
    const fn register_return(register: B8DebugRegisterName) -> Self {
        Self {
            role: B8DebugHelperReturnRole::ObjcMessageReturnValue,
            destination: B8DebugHelperValueSourceReport::register(register),
            materialization: B8DebugHelperMaterializationReport::available(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperReturnRole {
    ObjcMessageReturnValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugHelperValueSourceReport {
    kind: B8DebugHelperValueSourceKind,
    register: B8DebugRegisterName,
    width: B8DebugMemoryReadWidthReport,
}

impl B8DebugHelperValueSourceReport {
    const fn register(register: B8DebugRegisterName) -> Self {
        Self {
            kind: B8DebugHelperValueSourceKind::Register,
            register,
            width: B8DebugMemoryReadWidthReport::Bits64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperValueSourceKind {
    Register,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugHelperMaterializationReport {
    status: B8DebugValueMaterializationStatus,
    blocker: Option<B8DebugHelperBoundaryBlocker>,
}

impl B8DebugHelperMaterializationReport {
    const fn blocked(blocker: B8DebugHelperBoundaryBlocker) -> Self {
        Self {
            status: B8DebugValueMaterializationStatus::Blocked,
            blocker: Some(blocker),
        }
    }

    const fn available() -> Self {
        Self {
            status: B8DebugValueMaterializationStatus::Available,
            blocker: None,
        }
    }

    const fn from_status(materialized: bool, blocker: B8DebugHelperBoundaryBlocker) -> Self {
        if materialized {
            Self::available()
        } else {
            Self::blocked(blocker)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcMessageMaterializationBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    receiver: B8DebugObjcArgumentMaterializationReport,
    selector: B8DebugObjcArgumentMaterializationReport,
    return_value: B8DebugObjcReturnValueMaterializationReport,
    blockers: Vec<B8DebugObjcMessageMaterializationBlocker>,
    next_action: B8DebugObjcMessageMaterializationNextAction,
}

impl B8DebugObjcMessageMaterializationBoundaryReport {
    fn blocked(
        call_site: u64,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let receiver = B8DebugObjcArgumentMaterializationReport::from_register_argument(
            B8DebugObjcArgumentMaterializationSpec::receiver(),
            call_site,
            input,
            input_probe,
            decode_report,
            image_metadata,
        );
        let selector = B8DebugObjcArgumentMaterializationReport::from_register_argument(
            B8DebugObjcArgumentMaterializationSpec::selector(),
            call_site,
            input,
            input_probe,
            decode_report,
            image_metadata,
        );
        let return_value = B8DebugObjcReturnValueMaterializationReport::with_writeback_boundary();
        let mut blockers = Vec::new();
        if let Some(blocker) = receiver.mapped_value.blocker {
            blockers.push(blocker);
        } else if !receiver.mapped_value.is_resolved_for_helper_argument() {
            blockers.push(
                B8DebugObjcMessageMaterializationBlocker::ReceiverMappedValueFixupResolutionUnimplemented,
            );
        }
        if let Some(blocker) = selector.mapped_value.blocker {
            blockers.push(blocker);
        } else if !selector.mapped_value.is_resolved_for_helper_argument() {
            blockers.push(
                B8DebugObjcMessageMaterializationBlocker::SelectorMappedValueFixupResolutionUnimplemented,
            );
        }
        blockers.push(return_value.blocker);
        let next_action = if blockers
            .iter()
            .any(|blocker| blocker.requires_mapped_image_extension())
        {
            B8DebugObjcMessageMaterializationNextAction::ExtendMachOMappedImageMetadataForObjcMaterialization
        } else if blockers
            .iter()
            .any(|blocker| blocker.requires_mapped_value_fixup_resolution())
        {
            B8DebugObjcMessageMaterializationNextAction::ResolveObjcArgumentMappedValueFixups
        } else {
            B8DebugObjcMessageMaterializationNextAction::DefineObjcRuntimeHelperBridge
        };

        Self {
            schema: "b8_objc_message_materialization_boundary_v0",
            status: B8DebugImportBoundaryStatus::Blocked,
            receiver,
            selector,
            return_value,
            blockers,
            next_action,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct B8DebugObjcArgumentMaterializationSpec {
    position: u8,
    role: B8DebugHelperArgumentRole,
    source_register: B8DebugRegisterName,
    missing_definition_blocker: B8DebugObjcMessageMaterializationBlocker,
    unavailable_qword_blocker: B8DebugObjcMessageMaterializationBlocker,
}

impl B8DebugObjcArgumentMaterializationSpec {
    const fn receiver() -> Self {
        Self {
            position: 0,
            role: B8DebugHelperArgumentRole::ObjcReceiver,
            source_register: B8DebugRegisterName::Rdi,
            missing_definition_blocker:
                B8DebugObjcMessageMaterializationBlocker::ReceiverRegisterDefinitionUnavailable,
            unavailable_qword_blocker:
                B8DebugObjcMessageMaterializationBlocker::ReceiverMappedImageQwordUnavailable,
        }
    }

    const fn selector() -> Self {
        Self {
            position: 1,
            role: B8DebugHelperArgumentRole::ObjcSelector,
            source_register: B8DebugRegisterName::Rsi,
            missing_definition_blocker:
                B8DebugObjcMessageMaterializationBlocker::SelectorRegisterDefinitionUnavailable,
            unavailable_qword_blocker:
                B8DebugObjcMessageMaterializationBlocker::SelectorMappedImageQwordUnavailable,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcArgumentMaterializationReport {
    status: B8DebugValueMaterializationStatus,
    position: u8,
    role: B8DebugHelperArgumentRole,
    source_register: B8DebugRegisterName,
    source_definition: Option<B8DebugRegisterMaterializationSourceReport>,
    mapped_value: B8DebugObjcArgumentValueMaterializationReport,
}

impl B8DebugObjcArgumentMaterializationReport {
    fn from_register_argument(
        spec: B8DebugObjcArgumentMaterializationSpec,
        call_site: u64,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let source_definition =
            decode_report.last_register_materialization_before(spec.source_register, call_site);
        let mapped_value = B8DebugObjcArgumentValueMaterializationReport::from_source_definition(
            source_definition.as_ref(),
            input,
            input_probe,
            image_metadata,
            spec.missing_definition_blocker,
            spec.unavailable_qword_blocker,
        );
        Self {
            status: mapped_value.status,
            position: spec.position,
            role: spec.role,
            source_register: spec.source_register,
            source_definition,
            mapped_value,
        }
    }

    fn is_resolved_for_helper_argument(&self) -> bool {
        self.mapped_value.is_resolved_for_helper_argument()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcArgumentValueMaterializationReport {
    status: B8DebugValueMaterializationStatus,
    source: B8DebugObjcArgumentValueSource,
    address: Option<u64>,
    value: Option<u64>,
    fixup_resolution: Option<B8DebugObjcArgumentFixupResolutionReport>,
    blocker: Option<B8DebugObjcMessageMaterializationBlocker>,
}

impl B8DebugObjcArgumentValueMaterializationReport {
    fn from_source_definition(
        source_definition: Option<&B8DebugRegisterMaterializationSourceReport>,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        image_metadata: &ProgramImageMetadata,
        missing_definition_blocker: B8DebugObjcMessageMaterializationBlocker,
        unavailable_qword_blocker: B8DebugObjcMessageMaterializationBlocker,
    ) -> Self {
        let Some(source_definition) = source_definition else {
            return Self::blocked(
                B8DebugObjcArgumentValueSource::RegisterDefinitionUnavailable,
                None,
                missing_definition_blocker,
            );
        };

        match source_definition.kind {
            B8DebugRegisterMaterializationSourceKind::RipRelativeQwordLoad => {
                let value = image_metadata
                    .mapped_bytes()
                    .read_u64_le(X86Va::new(source_definition.address));
                match value {
                    Some(value) => {
                        let fixup_resolution =
                            B8DebugObjcArgumentFixupResolutionReport::from_mapped_pointer(
                                input,
                                input_probe,
                                source_definition.address,
                                value,
                            );
                        Self {
                            status: B8DebugValueMaterializationStatus::Available,
                            source: B8DebugObjcArgumentValueSource::ProgramImageMetadata,
                            address: Some(source_definition.address),
                            value: Some(value),
                            fixup_resolution: Some(fixup_resolution),
                            blocker: None,
                        }
                    }
                    None => Self::blocked(
                        B8DebugObjcArgumentValueSource::ProgramImageMetadata,
                        Some(source_definition.address),
                        unavailable_qword_blocker,
                    ),
                }
            }
            B8DebugRegisterMaterializationSourceKind::RipRelativeAddress => Self {
                status: B8DebugValueMaterializationStatus::Available,
                source: B8DebugObjcArgumentValueSource::RipRelativeAddress,
                address: Some(source_definition.address),
                value: Some(source_definition.address),
                fixup_resolution: None,
                blocker: None,
            },
        }
    }

    const fn blocked(
        source: B8DebugObjcArgumentValueSource,
        address: Option<u64>,
        blocker: B8DebugObjcMessageMaterializationBlocker,
    ) -> Self {
        Self {
            status: B8DebugValueMaterializationStatus::Blocked,
            source,
            address,
            value: None,
            fixup_resolution: None,
            blocker: Some(blocker),
        }
    }

    fn is_resolved_for_helper_argument(&self) -> bool {
        if matches!(
            (self.status, self.source),
            (
                B8DebugValueMaterializationStatus::Available,
                B8DebugObjcArgumentValueSource::RipRelativeAddress
            )
        ) {
            return true;
        }

        self.fixup_resolution
            .as_ref()
            .is_some_and(B8DebugObjcArgumentFixupResolutionReport::is_resolved)
    }

    fn resolved_import_identity(&self) -> Option<MachOChainedImportIdentityReport> {
        self.fixup_resolution
            .as_ref()
            .and_then(|resolution| resolution.import.clone())
    }

    fn resolved_rebase_target(&self) -> Option<MachOChainedRebaseTargetIdentityReport> {
        self.fixup_resolution
            .as_ref()
            .and_then(|resolution| resolution.rebase)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcArgumentFixupResolutionReport {
    status: B8DebugObjcArgumentFixupResolutionStatus,
    source: B8DebugObjcArgumentFixupResolutionSource,
    address: u64,
    raw_pointer: u64,
    import: Option<MachOChainedImportIdentityReport>,
    rebase: Option<MachOChainedRebaseTargetIdentityReport>,
    blocker: Option<MachOChainedFixupsBlocker>,
}

impl B8DebugObjcArgumentFixupResolutionReport {
    fn from_mapped_pointer(
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        address: u64,
        raw_pointer: u64,
    ) -> Self {
        let chained_fixups = decode_mach_o_chained_fixups_for_target(
            input,
            input_probe.metadata().mach_o_metadata(),
            MachOChainedFixupTargetAddress::from_mach_o_virtual_address(address),
        );
        let import = chained_fixups.resolved_import_identity();
        let rebase = chained_fixups.resolved_rebase_target();
        let status = if import.is_some() {
            B8DebugObjcArgumentFixupResolutionStatus::ResolvedImport
        } else if rebase.is_some() {
            B8DebugObjcArgumentFixupResolutionStatus::ResolvedRebase
        } else {
            B8DebugObjcArgumentFixupResolutionStatus::Blocked
        };

        Self {
            status,
            source: B8DebugObjcArgumentFixupResolutionSource::PublicDyldChainedFixups,
            address,
            raw_pointer,
            import,
            rebase,
            blocker: chained_fixups.blocker(),
        }
    }

    const fn is_resolved(&self) -> bool {
        matches!(
            self.status,
            B8DebugObjcArgumentFixupResolutionStatus::ResolvedImport
                | B8DebugObjcArgumentFixupResolutionStatus::ResolvedRebase
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcArgumentFixupResolutionStatus {
    Blocked,
    ResolvedImport,
    ResolvedRebase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcArgumentFixupResolutionSource {
    PublicDyldChainedFixups,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcReturnValueMaterializationReport {
    status: B8DebugValueMaterializationStatus,
    role: B8DebugHelperReturnRole,
    destination_register: B8DebugRegisterName,
    plan: B8DebugObjcReturnValueMaterializationPlan,
    writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
    blocker: B8DebugObjcMessageMaterializationBlocker,
}

impl B8DebugObjcReturnValueMaterializationReport {
    const fn with_writeback_boundary() -> Self {
        Self {
            status: B8DebugValueMaterializationStatus::Blocked,
            role: B8DebugHelperReturnRole::ObjcMessageReturnValue,
            destination_register: B8DebugRegisterName::Rax,
            plan: B8DebugObjcReturnValueMaterializationPlan::WriteHelperReturnToX8664Rax,
            writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport::blocked(),
            blocker: B8DebugObjcMessageMaterializationBlocker::ObjcHelperExecutionUnimplemented,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcHelperReturnWritebackBoundaryReport {
    schema: &'static str,
    status: B8DebugValueMaterializationStatus,
    source: B8DebugObjcHelperReturnWritebackSource,
    destination: B8DebugObjcHelperReturnWritebackDestination,
    width: B8DebugMemoryReadWidthReport,
    writeback_plan: B8DebugObjcReturnValueMaterializationPlan,
    ordering: B8DebugObjcHelperReturnWritebackOrdering,
    blocker: Option<B8DebugObjcMessageMaterializationBlocker>,
}

impl B8DebugObjcHelperReturnWritebackBoundaryReport {
    const fn blocked() -> Self {
        Self {
            schema: "b8_objc_helper_return_writeback_boundary_v0",
            status: B8DebugValueMaterializationStatus::Blocked,
            source: B8DebugObjcHelperReturnWritebackSource::ObjcHelperReturnValue,
            destination: B8DebugObjcHelperReturnWritebackDestination::X8664Rax,
            width: B8DebugMemoryReadWidthReport::Bits64,
            writeback_plan: B8DebugObjcReturnValueMaterializationPlan::WriteHelperReturnToX8664Rax,
            ordering: B8DebugObjcHelperReturnWritebackOrdering::AfterHelperCallReturns,
            blocker: Some(
                B8DebugObjcMessageMaterializationBlocker::ObjcHelperExecutionUnimplemented,
            ),
        }
    }

    const fn available(self) -> Self {
        Self {
            status: B8DebugValueMaterializationStatus::Available,
            blocker: None,
            ..self
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcHelperExecutionRequestReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    kind: B8DebugObjcHelperExecutionRequestKind,
    source_import: MachOChainedImportIdentityReport,
    receiver_identity: Option<MachOChainedImportIdentityReport>,
    selector_vm_address: Option<MachOChainedRebaseTargetIdentityReport>,
    return_writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
    required_capability: B8DebugObjcHelperExecutionCapability,
    bridge_contract: B8DebugObjcRuntimeHelperBridgeContractReport,
    host_execution: B8DebugObjcRuntimeHelperHostExecutionReport,
    return_continuation: Option<B8DebugObjcHelperReturnContinuationBoundaryReport>,
    blockers: Vec<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugObjcHelperExecutionNextAction,
}

impl B8DebugObjcHelperExecutionRequestReport {
    fn from_import_and_marshaling(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        import: &MachOChainedImportIdentityReport,
        marshaling: &B8DebugHelperMarshalingReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        code_bytes: &X86Bytes,
        image_metadata: &ProgramImageMetadata,
    ) -> Option<Self> {
        let contract = marshaling.contract.as_ref()?;
        let materialization = &contract.materialization_boundary;
        let receiver_identity = materialization
            .receiver
            .mapped_value
            .resolved_import_identity();
        let selector_vm_address = materialization
            .selector
            .mapped_value
            .resolved_rebase_target();
        let selector_identity = B8DebugObjcSelectorIdentityReport::from_rebase_target(
            selector_vm_address,
            image_metadata,
        );
        let mut blockers = Vec::new();
        if receiver_identity.is_none() {
            blockers.push(B8DebugObjcHelperExecutionBlocker::ReceiverIdentityUnavailable);
        }
        if selector_vm_address.is_none() {
            blockers.push(B8DebugObjcHelperExecutionBlocker::SelectorVmAddressUnavailable);
        }
        let requested_return_writeback_boundary = materialization.return_value.writeback_boundary;
        let required_capability =
            B8DebugObjcHelperExecutionCapability::ObjcRuntimeMessageSendHelper;
        let host_execution = B8DebugObjcRuntimeHelperHostExecutionReport::from_contract_inputs(
            import,
            receiver_identity.as_ref(),
            selector_identity.as_ref(),
            requested_return_writeback_boundary,
            required_capability,
        );
        let return_continuation =
            B8DebugObjcHelperReturnContinuationBoundaryReport::from_host_execution(
                call_boundary,
                &host_execution,
                input,
                input_probe,
                code_bytes,
                image_metadata,
            );
        if let Some(return_continuation) = &return_continuation {
            blockers.extend(return_continuation.blockers());
        } else {
            blockers.extend(host_execution.blockers());
        }
        let return_writeback_boundary = host_execution
            .executed_return_writeback_boundary()
            .unwrap_or(requested_return_writeback_boundary);
        let bridge_contract = B8DebugObjcRuntimeHelperBridgeContractReport::from_host_execution(
            import,
            receiver_identity.as_ref(),
            selector_identity,
            return_writeback_boundary,
            required_capability,
            host_execution.clone(),
        );
        let next_action = if blockers
            .iter()
            .any(|blocker| blocker.requires_materialization_inspection())
        {
            B8DebugObjcHelperExecutionNextAction::InspectObjcMessageMaterializationBoundary
        } else if return_continuation
            .as_ref()
            .and_then(|continuation| continuation.continuation_block.as_ref())
            .is_some()
        {
            B8DebugObjcHelperExecutionNextAction::InspectReturnToContinuationBlocker
        } else if host_execution.is_executed() {
            B8DebugObjcHelperExecutionNextAction::DecodeReturnToContinuationBlock
        } else if host_execution.is_skipped() {
            B8DebugObjcHelperExecutionNextAction::RunOnSupportedMacosHost
        } else {
            B8DebugObjcHelperExecutionNextAction::InspectObjcRuntimeHelperExecutionFailure
        };

        Some(Self {
            schema: "b8_objc_helper_execution_request_v0",
            status: B8DebugImportBoundaryStatus::Blocked,
            kind: B8DebugObjcHelperExecutionRequestKind::ObjcMsgSend,
            source_import: import.clone(),
            receiver_identity,
            selector_vm_address,
            return_writeback_boundary,
            required_capability,
            bridge_contract,
            host_execution,
            return_continuation,
            blockers,
            next_action,
        })
    }

    fn boundary_blocked_reason(&self) -> Option<B8DebugHelperBoundaryBlockedReason> {
        self.blockers
            .iter()
            .map(B8DebugHelperBoundaryBlockedReason::from_objc_helper_execution_blocker)
            .next()
    }

    fn boundary_blockers(&self) -> Vec<B8DebugHelperBoundaryBlocker> {
        self.blockers
            .iter()
            .map(B8DebugHelperBoundaryBlocker::from_objc_helper_execution_blocker)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperExecutionRequestKind {
    ObjcMsgSend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperExecutionCapability {
    ObjcRuntimeMessageSendHelper,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperExecutionBlocker {
    ObjcHelperExecutionUnimplemented,
    ObjcHelperReturnContinuationUnimplemented,
    ObjcRuntimeHelperHostExecutionFailed,
    ObjcRuntimeHelperHostExecutionUnsupported,
    ReceiverIdentityUnavailable,
    ReturnToContinuationDecodeFailed,
    ReturnToContinuationExecutionUnimplemented,
    ReturnToContinuationImportGlobalLoadUnimplemented,
    ReturnToContinuationUnsupportedInstruction,
    SelectorVmAddressUnavailable,
}

impl B8DebugObjcHelperExecutionBlocker {
    const fn requires_materialization_inspection(self) -> bool {
        matches!(
            self,
            Self::ReceiverIdentityUnavailable | Self::SelectorVmAddressUnavailable
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperExecutionNextAction {
    DecodeReturnToContinuationBlock,
    InspectReturnToContinuationBlocker,
    InspectObjcMessageMaterializationBoundary,
    InspectObjcRuntimeHelperExecutionFailure,
    RunOnSupportedMacosHost,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcHelperReturnContinuationBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    source: B8DebugObjcHelperReturnContinuationSourceReport,
    input: B8DebugObjcHelperReturnContinuationInputReport,
    register_state: B8DebugObjcHelperReturnContinuationRegisterStateReport,
    next_source_pc: u64,
    continuation_block: Option<B8DebugReturnToContinuationDecodeBoundaryReport>,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugObjcHelperReturnContinuationNextAction,
}

impl B8DebugObjcHelperReturnContinuationBoundaryReport {
    fn from_host_execution(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        host_execution: &B8DebugObjcRuntimeHelperHostExecutionReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        code_bytes: &X86Bytes,
        image_metadata: &ProgramImageMetadata,
    ) -> Option<Self> {
        let output = host_execution.output?;
        let return_writeback = host_execution.return_writeback?;
        let register_state = B8DebugObjcHelperReturnContinuationRegisterStateReport::from_writeback(
            return_writeback,
        );
        let imported_global_value =
            B8DebugReturnToContinuationImportedGlobalValue::nsapp_from_host_execution(
                host_execution,
            );
        let continuation_inputs = B8DebugReturnToContinuationDecodeInputs {
            imported_global_value,
            preserved_call_target_import: Some(host_execution.invocation.source_import.clone()),
        };
        let continuation_block = B8DebugReturnToContinuationDecodeBoundaryReport::from_code_bytes(
            call_boundary.return_to,
            register_state,
            continuation_inputs,
            code_bytes,
            input,
            input_probe,
            image_metadata,
        );
        let blocker = continuation_block.as_ref().map_or(
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented,
            B8DebugReturnToContinuationDecodeBoundaryReport::blocker,
        );
        let next_action = continuation_block.as_ref().map_or(
            B8DebugObjcHelperReturnContinuationNextAction::DecodeReturnToContinuationBlock,
            B8DebugReturnToContinuationDecodeBoundaryReport::next_action,
        );
        Some(Self {
            schema: "b8_objc_helper_return_continuation_boundary_v0",
            status: B8DebugImportBoundaryStatus::Blocked,
            source: B8DebugObjcHelperReturnContinuationSourceReport::from_call_boundary(
                call_boundary,
            ),
            input: B8DebugObjcHelperReturnContinuationInputReport::new(output, return_writeback),
            register_state,
            next_source_pc: call_boundary.return_to,
            continuation_block,
            blocker,
            next_action,
        })
    }

    fn blockers(&self) -> Vec<B8DebugObjcHelperExecutionBlocker> {
        vec![self.blocker]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcHelperReturnContinuationSourceReport {
    kind: B8DebugObjcHelperReturnContinuationSourceKind,
    call_site: u64,
    return_to: u64,
    target_register: B8DebugRegisterName,
}

impl B8DebugObjcHelperReturnContinuationSourceReport {
    const fn from_call_boundary(call_boundary: B8DebugRegisterIndirectCallBoundaryReport) -> Self {
        Self {
            kind: B8DebugObjcHelperReturnContinuationSourceKind::RegisterIndirectCallReturn,
            call_site: call_boundary.call_site,
            return_to: call_boundary.return_to,
            target_register: call_boundary.target_register,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperReturnContinuationSourceKind {
    RegisterIndirectCallReturn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcHelperReturnContinuationInputReport {
    helper_output: B8DebugObjcRuntimeHelperOutput,
    representation: B8DebugObjcRuntimeHelperOutputRepresentation,
    return_value: u64,
    writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
    written_value: u64,
}

impl B8DebugObjcHelperReturnContinuationInputReport {
    const fn new(
        output: B8DebugObjcRuntimeHelperOutputReport,
        return_writeback: B8DebugObjcRuntimeHelperReturnWritebackReport,
    ) -> Self {
        Self {
            helper_output: output.helper_output,
            representation: output.representation,
            return_value: output.return_value,
            writeback_boundary: return_writeback.boundary,
            written_value: return_writeback.written_value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcHelperReturnContinuationRegisterStateReport {
    register: B8DebugRegisterName,
    source: B8DebugObjcHelperReturnContinuationRegisterSource,
    value: u64,
    width: B8DebugMemoryReadWidthReport,
}

impl B8DebugObjcHelperReturnContinuationRegisterStateReport {
    const fn from_writeback(
        return_writeback: B8DebugObjcRuntimeHelperReturnWritebackReport,
    ) -> Self {
        Self {
            register: B8DebugRegisterName::Rax,
            source: B8DebugObjcHelperReturnContinuationRegisterSource::ObjcHelperReturnValue,
            value: return_writeback.written_value,
            width: B8DebugMemoryReadWidthReport::Bits64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperReturnContinuationRegisterSource {
    ObjcHelperReturnValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperReturnContinuationNextAction {
    AddReturnToContinuationInstructionSupport,
    DecodeReturnToContinuationBlock,
    ImplementReturnToContinuationExecution,
    MaterializeReturnToContinuationImportGlobalLoad,
    InspectReturnToContinuationDecodeFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationDecodeBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    source: B8DebugReturnToContinuationDecodeSourceReport,
    input_register_state: B8DebugObjcHelperReturnContinuationRegisterStateReport,
    materialized_register_states: Vec<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
    blocked_register_materializations:
        Vec<B8DebugReturnToContinuationBlockedRegisterMaterializationReport>,
    continuation_call_boundary: Option<B8DebugReturnToContinuationCallBoundaryReport>,
    decode_report: B8DebugDecodeReport,
    processed_source_pc_range: Option<B8DebugProcessedPcRange>,
    next_instruction: Option<B8DebugDecodedInstructionReport>,
    unsupported_instruction: Option<B8DebugUnsupportedInstructionReport>,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct B8DebugReturnToContinuationDecodeInputs {
    imported_global_value: Option<B8DebugReturnToContinuationImportedGlobalValue>,
    preserved_call_target_import: Option<MachOChainedImportIdentityReport>,
}

impl B8DebugReturnToContinuationDecodeBoundaryReport {
    fn from_code_bytes(
        source_pc: u64,
        input_register_state: B8DebugObjcHelperReturnContinuationRegisterStateReport,
        continuation_inputs: B8DebugReturnToContinuationDecodeInputs,
        code_bytes: &X86Bytes,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Option<Self> {
        let continuation_bytes = continuation_x86_bytes_from_code_segment(source_pc, code_bytes)?;
        let decoded_result = decode_function(&continuation_bytes);
        let decode_report = B8DebugDecodeReport::from_result(decoded_result.as_ref());
        let (
            processed_source_pc_range,
            next_instruction,
            unsupported_instruction,
            materialized_register_states,
            blocked_register_materializations,
            continuation_call_boundary,
            blocker,
            next_action,
        ) = match decoded_result {
            Ok(decoded) => {
                let unsupported_instruction =
                    B8DebugUnsupportedInstructionReport::from_decoded(&decoded);
                let (materialized_register_states, blocked_register_materializations) =
                    B8DebugReturnToContinuationMaterializedRegisterStateReport::from_decoded(
                        &decoded,
                        image_metadata.mapped_bytes(),
                        input,
                        input_probe,
                        continuation_inputs.imported_global_value,
                    );
                let continuation_call_boundary =
                    B8DebugReturnToContinuationCallBoundaryReport::from_decoded(
                        &decoded,
                        &materialized_register_states,
                        continuation_inputs.preserved_call_target_import,
                        image_metadata,
                    );
                let materialization_blocker =
                    blocked_register_materializations.first().map(|blocked| blocked.blocker);
                let blocker = if let Some(blocker) = materialization_blocker {
                    blocker
                } else if unsupported_instruction.is_some() {
                    B8DebugObjcHelperExecutionBlocker::ReturnToContinuationUnsupportedInstruction
                } else {
                    B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented
                };
                let next_action = if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationImportGlobalLoadUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationImportGlobalLoad
                } else if unsupported_instruction.is_some() {
                    B8DebugReturnToContinuationDecodeNextAction::AddReturnToContinuationInstructionSupport
                } else {
                    B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution
                };

                (
                    Some(B8DebugProcessedPcRange::from_decoded(&decoded)),
                    decoded
                        .instructions()
                        .first()
                        .map(B8DebugDecodedInstructionReport::from_instruction),
                    unsupported_instruction,
                    materialized_register_states,
                    blocked_register_materializations,
                    continuation_call_boundary,
                    blocker,
                    next_action,
                )
            }
            Err(_) => (
                None,
                None,
                None,
                Vec::new(),
                Vec::new(),
                None,
                B8DebugObjcHelperExecutionBlocker::ReturnToContinuationDecodeFailed,
                B8DebugReturnToContinuationDecodeNextAction::InspectReturnToContinuationDecodeFailure,
            ),
        };

        Some(Self {
            schema: "b8_return_to_continuation_decode_boundary_v0",
            status: B8DebugImportBoundaryStatus::Blocked,
            source: B8DebugReturnToContinuationDecodeSourceReport {
                kind: B8DebugReturnToContinuationDecodeSourceKind::ReturnToSourcePc,
                source_pc,
                byte_source: B8DebugReturnToContinuationByteSource::MachOCodeSegmentBytes,
            },
            input_register_state,
            materialized_register_states,
            blocked_register_materializations,
            continuation_call_boundary,
            decode_report,
            processed_source_pc_range,
            next_instruction,
            unsupported_instruction,
            blocker,
            next_action,
        })
    }

    const fn blocker(&self) -> B8DebugObjcHelperExecutionBlocker {
        self.blocker
    }

    const fn next_action(&self) -> B8DebugObjcHelperReturnContinuationNextAction {
        match self.next_action {
            B8DebugReturnToContinuationDecodeNextAction::AddReturnToContinuationInstructionSupport => {
                B8DebugObjcHelperReturnContinuationNextAction::AddReturnToContinuationInstructionSupport
            }
            B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution => {
                B8DebugObjcHelperReturnContinuationNextAction::ImplementReturnToContinuationExecution
            }
            B8DebugReturnToContinuationDecodeNextAction::InspectReturnToContinuationDecodeFailure => {
                B8DebugObjcHelperReturnContinuationNextAction::InspectReturnToContinuationDecodeFailure
            }
            B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationImportGlobalLoad => {
                B8DebugObjcHelperReturnContinuationNextAction::MaterializeReturnToContinuationImportGlobalLoad
            }
        }
    }
}

fn continuation_x86_bytes_from_code_segment(
    source_pc: u64,
    code_bytes: &X86Bytes,
) -> Option<X86Bytes> {
    let offset = source_pc.checked_sub(code_bytes.entry().value())?;
    let offset = usize::try_from(offset).ok()?;
    let bytes = code_bytes.bytes().get(offset..)?.to_vec();
    X86Bytes::new(X86Va::new(source_pc), bytes).ok()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationDecodeSourceReport {
    kind: B8DebugReturnToContinuationDecodeSourceKind,
    source_pc: u64,
    byte_source: B8DebugReturnToContinuationByteSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationDecodeSourceKind {
    ReturnToSourcePc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationByteSource {
    MachOCodeSegmentBytes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationDecodeNextAction {
    AddReturnToContinuationInstructionSupport,
    ImplementReturnToContinuationExecution,
    InspectReturnToContinuationDecodeFailure,
    MaterializeReturnToContinuationImportGlobalLoad,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationMaterializedRegisterStateReport {
    register: B8DebugRegisterName,
    source: B8DebugReturnToContinuationMaterializedRegisterSource,
    instruction_start: u64,
    instruction_end: u64,
    address: Option<u64>,
    base_register: Option<B8DebugRegisterName>,
    base_value: Option<u64>,
    base_fixup_resolution: Option<B8DebugObjcArgumentFixupResolutionReport>,
    value: u64,
    value_source: Option<B8DebugReturnToContinuationMaterializedRegisterValueSource>,
    fixup_resolution: Option<B8DebugObjcArgumentFixupResolutionReport>,
    width: B8DebugMemoryReadWidthReport,
}

impl B8DebugReturnToContinuationMaterializedRegisterStateReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        mapped_bytes: &ProgramImageMappedBytes,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        imported_global_value: Option<B8DebugReturnToContinuationImportedGlobalValue>,
    ) -> (
        Vec<Self>,
        Vec<B8DebugReturnToContinuationBlockedRegisterMaterializationReport>,
    ) {
        let mut states = Vec::new();
        let mut r15_address = None;
        let mut r15_value = None;
        let mut r15_fixup_resolution = None;
        let mut blocked = Vec::new();

        for instruction in decoded.instructions() {
            match instruction.kind() {
                DecodedInstructionKind::MovR15QwordPtrRipRelative { address, .. } => {
                    if let Some(value) = mapped_bytes.read_u64_le(*address) {
                        let fixup_resolution =
                            B8DebugObjcArgumentFixupResolutionReport::from_mapped_pointer(
                                input,
                                input_probe,
                                address.value(),
                                value,
                            );
                        r15_address = Some(
                            fixup_resolution
                                .rebase
                                .map_or(X86Va::new(value), |rebase| rebase.resolved_x86_va()),
                        );
                        r15_value = Some(value);
                        r15_fixup_resolution = Some(fixup_resolution.clone());
                        states.push(Self {
                            register: B8DebugRegisterName::R15,
                            source:
                                B8DebugReturnToContinuationMaterializedRegisterSource::RipRelativeQword,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            address: Some(address.value()),
                            base_register: None,
                            base_value: None,
                            base_fixup_resolution: None,
                            value,
                            value_source: None,
                            fixup_resolution: Some(fixup_resolution),
                            width: B8DebugMemoryReadWidthReport::Bits64,
                        });
                    }
                }
                DecodedInstructionKind::MovRsiQwordPtrRipRelative { address, .. } => {
                    if let Some(value) = mapped_bytes.read_u64_le(*address) {
                        let fixup_resolution =
                            B8DebugObjcArgumentFixupResolutionReport::from_mapped_pointer(
                                input,
                                input_probe,
                                address.value(),
                                value,
                            );
                        states.push(Self {
                            register: B8DebugRegisterName::Rsi,
                            source:
                                B8DebugReturnToContinuationMaterializedRegisterSource::RipRelativeQword,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            address: Some(address.value()),
                            base_register: None,
                            base_value: None,
                            base_fixup_resolution: None,
                            value,
                            value_source: None,
                            fixup_resolution: Some(fixup_resolution),
                            width: B8DebugMemoryReadWidthReport::Bits64,
                        });
                    }
                }
                DecodedInstructionKind::MovRdiQwordPtrR15 => {
                    if let Some(imported_global_value) = imported_global_value_for_resolution(
                        imported_global_value,
                        r15_fixup_resolution.as_ref(),
                    ) {
                        states.push(Self {
                            register: B8DebugRegisterName::Rdi,
                            source:
                                B8DebugReturnToContinuationMaterializedRegisterSource::ImportedGlobalPointee,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            address: None,
                            base_register: Some(B8DebugRegisterName::R15),
                            base_value: r15_value,
                            base_fixup_resolution: r15_fixup_resolution.clone(),
                            value: imported_global_value.value,
                            value_source: Some(imported_global_value.source),
                            fixup_resolution: None,
                            width: B8DebugMemoryReadWidthReport::Bits64,
                        });
                    } else if r15_fixup_resolution
                        .as_ref()
                        .is_some_and(|resolution| resolution.import.is_some())
                    {
                        if let Some(base_value) = r15_value {
                            blocked.push(
                                B8DebugReturnToContinuationBlockedRegisterMaterializationReport {
                                    register: B8DebugRegisterName::Rdi,
                                    source:
                                        B8DebugReturnToContinuationMaterializedRegisterSource::RegisterIndirectQword,
                                    instruction_start: instruction.start().value(),
                                    instruction_end: instruction.end().value(),
                                    base_register: B8DebugRegisterName::R15,
                                    base_value,
                                    base_fixup_resolution: r15_fixup_resolution.clone(),
                                    blocker: B8DebugObjcHelperExecutionBlocker::ReturnToContinuationImportGlobalLoadUnimplemented,
                                },
                            );
                        }
                    } else if let Some(address) = r15_address {
                        if let Some(value) = mapped_bytes.read_u64_le(address) {
                            let fixup_resolution =
                                B8DebugObjcArgumentFixupResolutionReport::from_mapped_pointer(
                                    input,
                                    input_probe,
                                    address.value(),
                                    value,
                                );
                            states.push(Self {
                                register: B8DebugRegisterName::Rdi,
                                source:
                                    B8DebugReturnToContinuationMaterializedRegisterSource::RegisterIndirectQword,
                                instruction_start: instruction.start().value(),
                                instruction_end: instruction.end().value(),
                                address: Some(address.value()),
                                base_register: Some(B8DebugRegisterName::R15),
                                base_value: r15_value,
                                base_fixup_resolution: r15_fixup_resolution.clone(),
                                value,
                                value_source: None,
                                fixup_resolution: Some(fixup_resolution),
                                width: B8DebugMemoryReadWidthReport::Bits64,
                            });
                        }
                    }
                }
                DecodedInstructionKind::XorEdxEdx => {
                    states.push(Self {
                        register: B8DebugRegisterName::Rdx,
                        source:
                            B8DebugReturnToContinuationMaterializedRegisterSource::XorEdxEdxZero,
                        instruction_start: instruction.start().value(),
                        instruction_end: instruction.end().value(),
                        address: None,
                        base_register: None,
                        base_value: None,
                        base_fixup_resolution: None,
                        value: 0,
                        value_source: None,
                        fixup_resolution: None,
                        width: B8DebugMemoryReadWidthReport::Bits64,
                    });
                }
                _ => {}
            }
        }

        (states, blocked)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationBlockedRegisterMaterializationReport {
    register: B8DebugRegisterName,
    source: B8DebugReturnToContinuationMaterializedRegisterSource,
    instruction_start: u64,
    instruction_end: u64,
    base_register: B8DebugRegisterName,
    base_value: u64,
    base_fixup_resolution: Option<B8DebugObjcArgumentFixupResolutionReport>,
    blocker: B8DebugObjcHelperExecutionBlocker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationMaterializedRegisterSource {
    #[serde(rename = "imported_global_pointee_load")]
    ImportedGlobalPointee,
    #[serde(rename = "register_indirect_qword_load")]
    RegisterIndirectQword,
    #[serde(rename = "rip_relative_qword_load")]
    RipRelativeQword,
    #[serde(rename = "xor_edx_edx_zero")]
    XorEdxEdxZero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationMaterializedRegisterValueSource {
    ObjcSharedApplicationHelperReturnValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct B8DebugReturnToContinuationImportedGlobalValue {
    symbol: B8DebugReturnToContinuationImportedGlobalSymbol,
    value: u64,
    source: B8DebugReturnToContinuationMaterializedRegisterValueSource,
}

impl B8DebugReturnToContinuationImportedGlobalValue {
    fn nsapp_from_host_execution(
        host_execution: &B8DebugObjcRuntimeHelperHostExecutionReport,
    ) -> Option<Self> {
        if !host_execution.is_executed()
            || !host_execution
                .invocation
                .is_supported_b8_shared_application_message()
        {
            return None;
        }

        Some(Self {
            symbol: B8DebugReturnToContinuationImportedGlobalSymbol::NsApp,
            value: host_execution.output?.return_value,
            source:
                B8DebugReturnToContinuationMaterializedRegisterValueSource::ObjcSharedApplicationHelperReturnValue,
        })
    }

    fn matches_import(self, import: &MachOChainedImportIdentityReport) -> bool {
        self.symbol.matches_import(import)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum B8DebugReturnToContinuationImportedGlobalSymbol {
    NsApp,
}

impl B8DebugReturnToContinuationImportedGlobalSymbol {
    fn matches_import(self, import: &MachOChainedImportIdentityReport) -> bool {
        match self {
            Self::NsApp => {
                import.symbol_name() == "_NSApp"
                    && import.dylib_path().is_some_and(|path| {
                        path == "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"
                    })
            }
        }
    }
}

fn imported_global_value_for_resolution(
    imported_global_value: Option<B8DebugReturnToContinuationImportedGlobalValue>,
    resolution: Option<&B8DebugObjcArgumentFixupResolutionReport>,
) -> Option<B8DebugReturnToContinuationImportedGlobalValue> {
    let imported_global_value = imported_global_value?;
    let import = resolution?.import.as_ref()?;
    imported_global_value
        .matches_import(import)
        .then_some(imported_global_value)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationCallBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    call_site: u64,
    return_to: u64,
    target_register: B8DebugRegisterName,
    target: B8DebugReturnToContinuationCallTargetReport,
    arguments: Vec<B8DebugReturnToContinuationCallArgumentReport>,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationCallBoundaryReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        materialized_register_states: &[B8DebugReturnToContinuationMaterializedRegisterStateReport],
        preserved_call_target_import: Option<MachOChainedImportIdentityReport>,
        image_metadata: &ProgramImageMetadata,
    ) -> Option<Self> {
        let (call_site, return_to) =
            decoded
                .instructions()
                .iter()
                .find_map(|instruction| match instruction.kind() {
                    DecodedInstructionKind::CallR14 { return_to } => {
                        Some((instruction.start().value(), return_to.value()))
                    }
                    _ => None,
                })?;

        Some(Self {
            schema: "b8_return_to_continuation_call_boundary_v0",
            status: B8DebugImportBoundaryStatus::Blocked,
            call_site,
            return_to,
            target_register: B8DebugRegisterName::R14,
            target: B8DebugReturnToContinuationCallTargetReport::preserved_r14(
                preserved_call_target_import,
            ),
            arguments: vec![
                B8DebugReturnToContinuationCallArgumentReport::from_materialized_register(
                    0,
                    B8DebugReturnToContinuationCallArgumentRole::Receiver,
                    B8DebugRegisterName::Rdi,
                    materialized_register_states,
                    image_metadata,
                ),
                B8DebugReturnToContinuationCallArgumentReport::from_materialized_register(
                    1,
                    B8DebugReturnToContinuationCallArgumentRole::Selector,
                    B8DebugRegisterName::Rsi,
                    materialized_register_states,
                    image_metadata,
                ),
                B8DebugReturnToContinuationCallArgumentReport::from_materialized_register(
                    2,
                    B8DebugReturnToContinuationCallArgumentRole::Argument,
                    B8DebugRegisterName::Rdx,
                    materialized_register_states,
                    image_metadata,
                ),
            ],
            blocker: B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented,
            next_action:
                B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationCallTargetReport {
    state: B8DebugValueMaterializationStatus,
    source: B8DebugReturnToContinuationCallTargetSource,
    preservation_model: B8DebugReturnToContinuationCallTargetPreservationModel,
    import: Option<MachOChainedImportIdentityReport>,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
}

impl B8DebugReturnToContinuationCallTargetReport {
    fn preserved_r14(import: Option<MachOChainedImportIdentityReport>) -> Self {
        let state = if import.is_some() {
            B8DebugValueMaterializationStatus::Available
        } else {
            B8DebugValueMaterializationStatus::Blocked
        };
        let blocker = if import.is_some() {
            None
        } else {
            Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented)
        };

        Self {
            state,
            source: B8DebugReturnToContinuationCallTargetSource::PreservedImportHelperCallTarget,
            preservation_model:
                B8DebugReturnToContinuationCallTargetPreservationModel::X8664MacosSystemVCalleeSavedRegister,
            import,
            blocker,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallTargetSource {
    PreservedImportHelperCallTarget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallTargetPreservationModel {
    #[serde(rename = "x86_64_macos_system_v_callee_saved_register")]
    X8664MacosSystemVCalleeSavedRegister,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationCallArgumentReport {
    position: u8,
    role: B8DebugReturnToContinuationCallArgumentRole,
    register: B8DebugRegisterName,
    state: B8DebugValueMaterializationStatus,
    materialized_state: Option<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
    selector_identity: Option<B8DebugObjcSelectorIdentityReport>,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
}

impl B8DebugReturnToContinuationCallArgumentReport {
    fn from_materialized_register(
        position: u8,
        role: B8DebugReturnToContinuationCallArgumentRole,
        register: B8DebugRegisterName,
        materialized_register_states: &[B8DebugReturnToContinuationMaterializedRegisterStateReport],
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        let materialized_state = materialized_register_states
            .iter()
            .rev()
            .find(|state| state.register == register)
            .cloned();
        let state = if materialized_state.is_some() {
            B8DebugValueMaterializationStatus::Available
        } else {
            B8DebugValueMaterializationStatus::Blocked
        };
        let selector_identity = if role == B8DebugReturnToContinuationCallArgumentRole::Selector {
            materialized_state
                .as_ref()
                .and_then(|state| state.fixup_resolution.as_ref())
                .and_then(|resolution| resolution.rebase)
                .and_then(|rebase| {
                    B8DebugObjcSelectorIdentityReport::from_rebase_target(
                        Some(rebase),
                        image_metadata,
                    )
                })
        } else {
            None
        };
        let blocker = if materialized_state.is_some() {
            None
        } else {
            Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented)
        };

        Self {
            position,
            role,
            register,
            state,
            materialized_state,
            selector_identity,
            blocker,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallArgumentRole {
    #[serde(rename = "objc_argument")]
    Argument,
    #[serde(rename = "objc_receiver")]
    Receiver,
    #[serde(rename = "objc_selector")]
    Selector,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcSelectorIdentityReport {
    vm_address: MachOChainedRebaseTargetIdentityReport,
    name: Option<String>,
    source: B8DebugObjcSelectorIdentitySource,
}

impl B8DebugObjcSelectorIdentityReport {
    fn from_rebase_target(
        vm_address: Option<MachOChainedRebaseTargetIdentityReport>,
        image_metadata: &ProgramImageMetadata,
    ) -> Option<Self> {
        let vm_address = vm_address?;
        let name = image_metadata
            .mapped_bytes()
            .read_nul_terminated_utf8(vm_address.resolved_x86_va())
            .map(str::to_owned);
        Some(Self {
            vm_address,
            name,
            source: B8DebugObjcSelectorIdentitySource::ProgramImageMetadataMappedBytes,
        })
    }

    fn selector_name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcSelectorIdentitySource {
    ProgramImageMetadataMappedBytes,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperHostExecutionReport {
    schema: &'static str,
    status: B8DebugObjcRuntimeHelperHostExecutionStatus,
    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary,
    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope,
    invocation: B8DebugObjcRuntimeHelperInvocationReport,
    output: Option<B8DebugObjcRuntimeHelperOutputReport>,
    return_writeback: Option<B8DebugObjcRuntimeHelperReturnWritebackReport>,
    error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    next_blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugObjcRuntimeHelperBridgeNextAction,
}

impl B8DebugObjcRuntimeHelperHostExecutionReport {
    fn from_contract_inputs(
        source_import: &MachOChainedImportIdentityReport,
        receiver_identity: Option<&MachOChainedImportIdentityReport>,
        selector_identity: Option<&B8DebugObjcSelectorIdentityReport>,
        return_writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
        capability: B8DebugObjcHelperExecutionCapability,
    ) -> Self {
        let invocation = B8DebugObjcRuntimeHelperInvocationReport::new(
            source_import,
            receiver_identity,
            selector_identity,
            capability,
        );

        if !invocation.is_supported_b8_shared_application_message() {
            return Self::blocked(
                invocation,
                B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHelperContract,
            );
        }
        if !cfg!(target_os = "macos") {
            return Self::skipped(
                invocation,
                B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHost,
            );
        }

        match run_public_objc_msg_send_shared_application_helper() {
            Ok(observation) => {
                let output = B8DebugObjcRuntimeHelperOutputReport::from_observation(observation);
                let return_writeback = B8DebugObjcRuntimeHelperReturnWritebackReport::new(
                    return_writeback_boundary.available(),
                    output.return_value,
                );
                Self {
                    schema: "b8_objc_runtime_helper_host_execution_v0",
                    status: B8DebugObjcRuntimeHelperHostExecutionStatus::Executed,
                    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
                    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
                    invocation,
                    output: Some(output),
                    return_writeback: Some(return_writeback),
                    error: None,
                    next_blocker: Some(
                        B8DebugObjcHelperExecutionBlocker::ObjcHelperReturnContinuationUnimplemented,
                    ),
                    next_action: B8DebugObjcRuntimeHelperBridgeNextAction::ContinueAfterObjcHelperReturn,
                }
            }
            Err(error) => Self::failed(invocation, error),
        }
    }

    fn blocked(
        invocation: B8DebugObjcRuntimeHelperInvocationReport,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
    ) -> Self {
        Self::with_error(
            invocation,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Blocked,
            classification,
            B8DebugObjcHelperExecutionBlocker::ObjcHelperExecutionUnimplemented,
            B8DebugObjcRuntimeHelperBridgeNextAction::ImplementPublicObjcRuntimeHelperBridge,
            None,
        )
    }

    fn skipped(
        invocation: B8DebugObjcRuntimeHelperInvocationReport,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
    ) -> Self {
        Self::with_error(
            invocation,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped,
            classification,
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionUnsupported,
            B8DebugObjcRuntimeHelperBridgeNextAction::RunOnSupportedMacosHost,
            None,
        )
    }

    fn failed(
        invocation: B8DebugObjcRuntimeHelperInvocationReport,
        error: B8DebugObjcRuntimeHelperHostExecutionErrorReport,
    ) -> Self {
        Self::with_error(
            invocation,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Failed,
            error.error_classification,
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionFailed,
            B8DebugObjcRuntimeHelperBridgeNextAction::InspectObjcRuntimeHelperExecutionFailure,
            Some(error),
        )
    }

    fn with_error(
        invocation: B8DebugObjcRuntimeHelperInvocationReport,
        status: B8DebugObjcRuntimeHelperHostExecutionStatus,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
        blocker: B8DebugObjcHelperExecutionBlocker,
        next_action: B8DebugObjcRuntimeHelperBridgeNextAction,
        error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    ) -> Self {
        Self {
            schema: "b8_objc_runtime_helper_host_execution_v0",
            status,
            api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
            fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
            invocation,
            output: None,
            return_writeback: None,
            error: error.or(Some(
                B8DebugObjcRuntimeHelperHostExecutionErrorReport::classification_only(
                    classification,
                ),
            )),
            next_blocker: Some(blocker),
            next_action,
        }
    }

    const fn is_executed(&self) -> bool {
        matches!(
            self.status,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Executed
        )
    }

    const fn is_skipped(&self) -> bool {
        matches!(
            self.status,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped
        )
    }

    fn blockers(&self) -> Vec<B8DebugObjcHelperExecutionBlocker> {
        self.next_blocker.into_iter().collect()
    }

    const fn primary_blocker(&self) -> Option<B8DebugObjcHelperExecutionBlocker> {
        self.next_blocker
    }

    fn executed_return_writeback_boundary(
        &self,
    ) -> Option<B8DebugObjcHelperReturnWritebackBoundaryReport> {
        self.return_writeback
            .as_ref()
            .map(|writeback| writeback.boundary)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperHostExecutionStatus {
    Blocked,
    Executed,
    Failed,
    Skipped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperHostApiBoundary {
    #[serde(rename = "public_objc_runtime_appkit")]
    PublicObjcRuntimeAppKit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperFixtureScope {
    SelfAuthoredB8GuiFixture,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperInvocationReport {
    source_import: MachOChainedImportIdentityReport,
    receiver_identity: Option<MachOChainedImportIdentityReport>,
    selector_identity: Option<B8DebugObjcSelectorIdentityReport>,
    required_capability: B8DebugObjcHelperExecutionCapability,
    message_send: B8DebugObjcRuntimeHelperMessageSendReport,
}

impl B8DebugObjcRuntimeHelperInvocationReport {
    fn new(
        source_import: &MachOChainedImportIdentityReport,
        receiver_identity: Option<&MachOChainedImportIdentityReport>,
        selector_identity: Option<&B8DebugObjcSelectorIdentityReport>,
        required_capability: B8DebugObjcHelperExecutionCapability,
    ) -> Self {
        Self {
            source_import: source_import.clone(),
            receiver_identity: receiver_identity.cloned(),
            selector_identity: selector_identity.cloned(),
            required_capability,
            message_send: B8DebugObjcRuntimeHelperMessageSendReport::from_inputs(
                receiver_identity,
                selector_identity,
            ),
        }
    }

    fn is_supported_b8_shared_application_message(&self) -> bool {
        self.source_import.symbol_name() == "_objc_msgSend"
            && self
                .source_import
                .dylib_path()
                .is_some_and(|path| path == "/usr/lib/libobjc.A.dylib")
            && self.receiver_identity.as_ref().is_some_and(|receiver| {
                receiver.symbol_name() == "_OBJC_CLASS_$_NSApplication"
                    && receiver.dylib_path().is_some_and(|path| {
                        path == "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"
                    })
            })
            && self
                .selector_identity
                .as_ref()
                .and_then(B8DebugObjcSelectorIdentityReport::selector_name)
                .is_some_and(|name| name == "sharedApplication")
            && self.required_capability
                == B8DebugObjcHelperExecutionCapability::ObjcRuntimeMessageSendHelper
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperMessageSendReport {
    function: B8DebugObjcRuntimeHelperMessageSendFunction,
    receiver: B8DebugObjcRuntimeHelperMessageSendReceiver,
    selector_name: Option<String>,
}

impl B8DebugObjcRuntimeHelperMessageSendReport {
    fn from_inputs(
        receiver_identity: Option<&MachOChainedImportIdentityReport>,
        selector_identity: Option<&B8DebugObjcSelectorIdentityReport>,
    ) -> Self {
        Self {
            function: B8DebugObjcRuntimeHelperMessageSendFunction::ObjcMsgSend,
            receiver: B8DebugObjcRuntimeHelperMessageSendReceiver::from_identity(receiver_identity),
            selector_name: selector_identity
                .and_then(B8DebugObjcSelectorIdentityReport::selector_name)
                .map(str::to_owned),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperMessageSendFunction {
    #[serde(rename = "_objc_msgSend")]
    ObjcMsgSend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperMessageSendReceiver {
    NsApplicationClassObject,
    Unknown,
}

impl B8DebugObjcRuntimeHelperMessageSendReceiver {
    fn from_identity(receiver_identity: Option<&MachOChainedImportIdentityReport>) -> Self {
        if receiver_identity.is_some_and(|receiver| {
            receiver.symbol_name() == "_OBJC_CLASS_$_NSApplication"
                && receiver.dylib_path().is_some_and(|path| {
                    path == "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"
                })
        }) {
            Self::NsApplicationClassObject
        } else {
            Self::Unknown
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperHostObservation {
    return_value: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperOutputReport {
    helper_output: B8DebugObjcRuntimeHelperOutput,
    representation: B8DebugObjcRuntimeHelperOutputRepresentation,
    return_value: u64,
}

impl B8DebugObjcRuntimeHelperOutputReport {
    const fn from_observation(observation: B8DebugObjcRuntimeHelperHostObservation) -> Self {
        Self {
            helper_output: B8DebugObjcRuntimeHelperOutput::ObjcHelperReturnValue,
            representation: B8DebugObjcRuntimeHelperOutputRepresentation::HostPointerU64,
            return_value: observation.return_value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperOutputRepresentation {
    HostPointerU64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperReturnWritebackReport {
    boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
    destination: B8DebugObjcHelperReturnWritebackDestination,
    written_value: u64,
}

impl B8DebugObjcRuntimeHelperReturnWritebackReport {
    const fn new(
        boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
        written_value: u64,
    ) -> Self {
        Self {
            destination: boundary.destination,
            boundary,
            written_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperHostExecutionErrorReport {
    error_classification: B8DebugObjcRuntimeHelperErrorClassification,
    message: Option<String>,
    status: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl B8DebugObjcRuntimeHelperHostExecutionErrorReport {
    const fn classification_only(
        error_classification: B8DebugObjcRuntimeHelperErrorClassification,
    ) -> Self {
        Self {
            error_classification,
            message: None,
            status: None,
            stdout: None,
            stderr: None,
        }
    }

    fn message(
        error_classification: B8DebugObjcRuntimeHelperErrorClassification,
        message: impl Into<String>,
    ) -> Self {
        Self {
            error_classification,
            message: Some(message.into()),
            status: None,
            stdout: None,
            stderr: None,
        }
    }

    fn process_output(
        error_classification: B8DebugObjcRuntimeHelperErrorClassification,
        status: String,
        output: Output,
    ) -> Self {
        Self {
            error_classification,
            message: None,
            status: Some(status),
            stdout: Some(String::from_utf8_lossy(&output.stdout).into_owned()),
            stderr: Some(String::from_utf8_lossy(&output.stderr).into_owned()),
        }
    }
}

fn run_public_objc_msg_send_shared_application_helper(
) -> Result<B8DebugObjcRuntimeHelperHostObservation, B8DebugObjcRuntimeHelperHostExecutionErrorReport>
{
    let source_path = temporary_objc_runtime_helper_path("m")?;
    let executable_path = temporary_objc_runtime_helper_path("exe")?;
    if let Err(error) = fs::write(
        &source_path,
        B8_OBJC_RUNTIME_SHARED_APPLICATION_HELPER_SOURCE,
    ) {
        let _ = fs::remove_file(&source_path);
        let _ = fs::remove_file(&executable_path);
        return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
            B8DebugObjcRuntimeHelperErrorClassification::HelperBuildFailed,
            format!(
                "failed to write Objective-C helper source {}: {error}",
                source_path.display()
            ),
        ));
    }

    let build_output = Command::new("clang")
        .args([
            "-x",
            "objective-c",
            source_path.to_string_lossy().as_ref(),
            "-framework",
            "AppKit",
            "-o",
            executable_path.to_string_lossy().as_ref(),
        ])
        .output();
    let _ = fs::remove_file(&source_path);

    let build_output = match build_output {
        Ok(output) => output,
        Err(error) => {
            let _ = fs::remove_file(&executable_path);
            return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::HelperBuildFailed,
                format!("failed to spawn clang for Objective-C helper: {error}"),
            ));
        }
    };
    if !build_output.status.success() {
        let status = build_output.status.to_string();
        let _ = fs::remove_file(&executable_path);
        return Err(
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::process_output(
                B8DebugObjcRuntimeHelperErrorClassification::HelperBuildFailed,
                status,
                build_output,
            ),
        );
    }

    let run_output = Command::new(&executable_path).output();
    let _ = fs::remove_file(&executable_path);
    let run_output = run_output.map_err(|error| {
        B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
            B8DebugObjcRuntimeHelperErrorClassification::HelperRunFailed,
            format!(
                "failed to run Objective-C runtime helper {}: {error}",
                executable_path.display()
            ),
        )
    })?;
    if !run_output.status.success() {
        return Err(
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::process_output(
                B8DebugObjcRuntimeHelperErrorClassification::HelperRunFailed,
                run_output.status.to_string(),
                run_output,
            ),
        );
    }

    let stdout = String::from_utf8_lossy(&run_output.stdout);
    let observation: B8DebugObjcRuntimeHelperHostObservation = serde_json::from_str(&stdout)
        .map_err(|error| {
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                format!(
                    "Objective-C runtime helper emitted invalid JSON: {error}; stdout={stdout:?}"
                ),
            )
        })?;
    if observation.return_value == 0 {
        return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
            B8DebugObjcRuntimeHelperErrorClassification::EmptyHelperReturnValue,
            "Objective-C runtime helper returned a null object pointer",
        ));
    }

    Ok(observation)
}

fn temporary_objc_runtime_helper_path(
    extension: &str,
) -> Result<PathBuf, B8DebugObjcRuntimeHelperHostExecutionErrorReport> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::HelperBuildFailed,
                format!("failed to build temporary Objective-C helper path: {error}"),
            )
        })?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "bara-b8-objc-runtime-helper-{}-{nanos}.{extension}",
        std::process::id()
    )))
}

const B8_OBJC_RUNTIME_SHARED_APPLICATION_HELPER_SOURCE: &str = r#"
#import <AppKit/AppKit.h>
#import <objc/message.h>
#import <objc/runtime.h>
#include <stdint.h>
#include <stdio.h>

int main(void) {
    @autoreleasepool {
        Class application_class = NSClassFromString(@"NSApplication");
        SEL selector = sel_registerName("sharedApplication");
        id (*send_id)(id, SEL) = (id (*)(id, SEL))objc_msgSend;
        id app = send_id((id)application_class, selector);
        uintptr_t value = (uintptr_t)app;
        if (value == 0) {
            return 2;
        }
        printf("{\"schema\":\"b8_objc_runtime_helper_host_observation_v0\",\"return_value\":%llu}\n",
               (unsigned long long)value);
    }
    return 0;
}
"#;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperBridgeContractReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    capability: B8DebugObjcHelperExecutionCapability,
    input_contract: B8DebugObjcRuntimeHelperBridgeInputContractReport,
    output_contract: B8DebugObjcRuntimeHelperBridgeOutputContractReport,
    error_contract: B8DebugObjcRuntimeHelperBridgeErrorContractReport,
    host_execution: B8DebugObjcRuntimeHelperHostExecutionReport,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugObjcRuntimeHelperBridgeNextAction,
}

impl B8DebugObjcRuntimeHelperBridgeContractReport {
    fn from_host_execution(
        source_import: &MachOChainedImportIdentityReport,
        receiver_identity: Option<&MachOChainedImportIdentityReport>,
        selector_identity: Option<B8DebugObjcSelectorIdentityReport>,
        return_writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
        capability: B8DebugObjcHelperExecutionCapability,
        host_execution: B8DebugObjcRuntimeHelperHostExecutionReport,
    ) -> Self {
        let status = if host_execution.is_executed() {
            B8DebugImportBoundaryStatus::Executed
        } else if host_execution.is_skipped() {
            B8DebugImportBoundaryStatus::Skipped
        } else {
            B8DebugImportBoundaryStatus::Blocked
        };
        let blocker = host_execution.primary_blocker();
        let next_action =
            B8DebugObjcRuntimeHelperBridgeNextAction::from_host_execution(host_execution.status);
        Self {
            schema: "b8_objc_runtime_helper_bridge_contract_v0",
            status,
            capability,
            input_contract: B8DebugObjcRuntimeHelperBridgeInputContractReport::new(
                source_import,
                receiver_identity,
                selector_identity.as_ref(),
                capability,
            ),
            output_contract: B8DebugObjcRuntimeHelperBridgeOutputContractReport::new(
                return_writeback_boundary,
            ),
            error_contract: B8DebugObjcRuntimeHelperBridgeErrorContractReport::from_host_execution(
                &host_execution,
            ),
            host_execution,
            blocker,
            next_action,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperBridgeInputContractReport {
    source_import: MachOChainedImportIdentityReport,
    receiver_identity: Option<MachOChainedImportIdentityReport>,
    selector_vm_address: Option<MachOChainedRebaseTargetIdentityReport>,
    selector_identity: Option<B8DebugObjcSelectorIdentityReport>,
    required_capability: B8DebugObjcHelperExecutionCapability,
}

impl B8DebugObjcRuntimeHelperBridgeInputContractReport {
    fn new(
        source_import: &MachOChainedImportIdentityReport,
        receiver_identity: Option<&MachOChainedImportIdentityReport>,
        selector_identity: Option<&B8DebugObjcSelectorIdentityReport>,
        required_capability: B8DebugObjcHelperExecutionCapability,
    ) -> Self {
        Self {
            source_import: source_import.clone(),
            receiver_identity: receiver_identity.cloned(),
            selector_vm_address: selector_identity.map(|selector| selector.vm_address),
            selector_identity: selector_identity.cloned(),
            required_capability,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperBridgeOutputContractReport {
    helper_output: B8DebugObjcRuntimeHelperOutput,
    return_writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
}

impl B8DebugObjcRuntimeHelperBridgeOutputContractReport {
    const fn new(
        return_writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
    ) -> Self {
        Self {
            helper_output: B8DebugObjcRuntimeHelperOutput::ObjcHelperReturnValue,
            return_writeback_boundary,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugObjcRuntimeHelperBridgeErrorContractReport {
    error_classification: Option<B8DebugObjcRuntimeHelperErrorClassification>,
}

impl B8DebugObjcRuntimeHelperBridgeErrorContractReport {
    fn from_host_execution(host_execution: &B8DebugObjcRuntimeHelperHostExecutionReport) -> Self {
        Self {
            error_classification: host_execution
                .error
                .as_ref()
                .map(|error| error.error_classification),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperOutput {
    ObjcHelperReturnValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperErrorClassification {
    EmptyHelperReturnValue,
    HelperBuildFailed,
    HelperRunFailed,
    InvalidHelperOutput,
    UnsupportedHelperContract,
    UnsupportedHost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcRuntimeHelperBridgeNextAction {
    ContinueAfterObjcHelperReturn,
    ImplementPublicObjcRuntimeHelperBridge,
    InspectObjcRuntimeHelperExecutionFailure,
    RunOnSupportedMacosHost,
}

impl B8DebugObjcRuntimeHelperBridgeNextAction {
    const fn from_host_execution(status: B8DebugObjcRuntimeHelperHostExecutionStatus) -> Self {
        match status {
            B8DebugObjcRuntimeHelperHostExecutionStatus::Executed => {
                Self::ContinueAfterObjcHelperReturn
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped => Self::RunOnSupportedMacosHost,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Failed => {
                Self::InspectObjcRuntimeHelperExecutionFailure
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Blocked => {
                Self::ImplementPublicObjcRuntimeHelperBridge
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperReturnWritebackSource {
    ObjcHelperReturnValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperReturnWritebackDestination {
    #[serde(rename = "x86_64_rax")]
    X8664Rax,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcHelperReturnWritebackOrdering {
    AfterHelperCallReturns,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugRegisterMaterializationSourceReport {
    kind: B8DebugRegisterMaterializationSourceKind,
    target_register: B8DebugRegisterName,
    instruction_start: u64,
    instruction_end: u64,
    address: u64,
    width: Option<B8DebugMemoryReadWidthReport>,
}

impl B8DebugRegisterMaterializationSourceReport {
    const fn rip_relative_qword_load(
        instruction: &B8DebugDecodedInstructionReport,
        target_register: B8DebugRegisterName,
        address: u64,
        width: B8DebugMemoryReadWidthReport,
    ) -> Self {
        Self {
            kind: B8DebugRegisterMaterializationSourceKind::RipRelativeQwordLoad,
            target_register,
            instruction_start: instruction.start,
            instruction_end: instruction.end,
            address,
            width: Some(width),
        }
    }

    const fn rip_relative_address(
        instruction: &B8DebugDecodedInstructionReport,
        target_register: B8DebugRegisterName,
        address: u64,
    ) -> Self {
        Self {
            kind: B8DebugRegisterMaterializationSourceKind::RipRelativeAddress,
            target_register,
            instruction_start: instruction.start,
            instruction_end: instruction.end,
            address,
            width: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugRegisterMaterializationSourceKind {
    RipRelativeQwordLoad,
    RipRelativeAddress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugValueMaterializationStatus {
    Available,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcArgumentValueSource {
    ProgramImageMetadata,
    RegisterDefinitionUnavailable,
    RipRelativeAddress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcReturnValueMaterializationPlan {
    #[serde(rename = "write_helper_return_to_x86_64_rax")]
    WriteHelperReturnToX8664Rax,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcMessageMaterializationBlocker {
    ReceiverRegisterDefinitionUnavailable,
    SelectorRegisterDefinitionUnavailable,
    ReceiverMappedImageQwordUnavailable,
    SelectorMappedImageQwordUnavailable,
    ReceiverMappedValueFixupResolutionUnimplemented,
    SelectorMappedValueFixupResolutionUnimplemented,
    ObjcHelperExecutionUnimplemented,
}

impl B8DebugObjcMessageMaterializationBlocker {
    const fn requires_mapped_image_extension(self) -> bool {
        matches!(
            self,
            Self::ReceiverRegisterDefinitionUnavailable
                | Self::SelectorRegisterDefinitionUnavailable
                | Self::ReceiverMappedImageQwordUnavailable
                | Self::SelectorMappedImageQwordUnavailable
        )
    }

    const fn requires_mapped_value_fixup_resolution(self) -> bool {
        matches!(
            self,
            Self::ReceiverMappedValueFixupResolutionUnimplemented
                | Self::SelectorMappedValueFixupResolutionUnimplemented
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugObjcMessageMaterializationNextAction {
    DefineObjcRuntimeHelperBridge,
    ExtendMachOMappedImageMetadataForObjcMaterialization,
    ResolveObjcArgumentMappedValueFixups,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperMarshalingNextAction {
    DefineObjcRuntimeHelperBridge,
    ExtendMachOMappedImageMetadataForObjcMaterialization,
    ResolveObjcArgumentMappedValueFixups,
}

impl B8DebugHelperMarshalingNextAction {
    const fn from_materialization_next_action(
        action: B8DebugObjcMessageMaterializationNextAction,
    ) -> Self {
        match action {
            B8DebugObjcMessageMaterializationNextAction::DefineObjcRuntimeHelperBridge => {
                Self::DefineObjcRuntimeHelperBridge
            }
            B8DebugObjcMessageMaterializationNextAction::ExtendMachOMappedImageMetadataForObjcMaterialization => {
                Self::ExtendMachOMappedImageMetadataForObjcMaterialization
            }
            B8DebugObjcMessageMaterializationNextAction::ResolveObjcArgumentMappedValueFixups => {
                Self::ResolveObjcArgumentMappedValueFixups
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperArgumentModel {
    #[serde(rename = "x86_64_call_arguments")]
    X8664CallArguments,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperReturnModel {
    #[serde(rename = "x86_64_rax_return_value")]
    X8664RaxReturnValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperBoundaryBlockedReason {
    ImportHelperMarshalingUnimplemented,
    ImportSymbolIdentityUnresolved,
    ObjcHelperReturnContinuationUnimplemented,
    ObjcRuntimeHelperExecutionFailed,
    ObjcRuntimeHelperExecutionUnsupported,
    ReturnToContinuationDecodeFailed,
    ReturnToContinuationExecutionUnimplemented,
    ReturnToContinuationImportGlobalLoadUnimplemented,
    ReturnToContinuationUnsupportedInstruction,
}

impl B8DebugHelperBoundaryBlockedReason {
    const fn from_objc_helper_execution_blocker(
        blocker: &B8DebugObjcHelperExecutionBlocker,
    ) -> Self {
        match blocker {
            B8DebugObjcHelperExecutionBlocker::ReceiverIdentityUnavailable
            | B8DebugObjcHelperExecutionBlocker::SelectorVmAddressUnavailable
            | B8DebugObjcHelperExecutionBlocker::ObjcHelperExecutionUnimplemented => {
                Self::ImportHelperMarshalingUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ObjcHelperReturnContinuationUnimplemented => {
                Self::ObjcHelperReturnContinuationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented => {
                Self::ReturnToContinuationExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationDecodeFailed => {
                Self::ReturnToContinuationDecodeFailed
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationImportGlobalLoadUnimplemented => {
                Self::ReturnToContinuationImportGlobalLoadUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationUnsupportedInstruction => {
                Self::ReturnToContinuationUnsupportedInstruction
            }
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionFailed => {
                Self::ObjcRuntimeHelperExecutionFailed
            }
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionUnsupported => {
                Self::ObjcRuntimeHelperExecutionUnsupported
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugHelperBoundaryBlocker {
    ImportSymbolIdentityUnresolved,
    #[serde(rename = "x86_64_argument_marshaling_unimplemented")]
    X8664ArgumentMarshalingUnimplemented,
    HelperReturnMarshalingUnimplemented,
    ObjcReceiverMaterializationUnimplemented,
    ObjcSelectorMaterializationUnimplemented,
    ObjcHelperExecutionUnimplemented,
    ObjcHelperReturnContinuationUnimplemented,
    ObjcRuntimeHelperExecutionFailed,
    ObjcRuntimeHelperExecutionUnsupported,
    ReturnToContinuationDecodeFailed,
    ReturnToContinuationExecutionUnimplemented,
    ReturnToContinuationImportGlobalLoadUnimplemented,
    ReturnToContinuationUnsupportedInstruction,
}

impl B8DebugHelperBoundaryBlocker {
    fn from_reason(reason: B8DebugHelperBoundaryBlockedReason) -> Vec<Self> {
        match reason {
            B8DebugHelperBoundaryBlockedReason::ImportHelperMarshalingUnimplemented => vec![
                Self::X8664ArgumentMarshalingUnimplemented,
                Self::HelperReturnMarshalingUnimplemented,
            ],
            B8DebugHelperBoundaryBlockedReason::ImportSymbolIdentityUnresolved => {
                vec![Self::ImportSymbolIdentityUnresolved]
            }
            B8DebugHelperBoundaryBlockedReason::ObjcHelperReturnContinuationUnimplemented => {
                vec![Self::ObjcHelperReturnContinuationUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ObjcRuntimeHelperExecutionFailed => {
                vec![Self::ObjcRuntimeHelperExecutionFailed]
            }
            B8DebugHelperBoundaryBlockedReason::ObjcRuntimeHelperExecutionUnsupported => {
                vec![Self::ObjcRuntimeHelperExecutionUnsupported]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationExecutionUnimplemented => {
                vec![Self::ReturnToContinuationExecutionUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationDecodeFailed => {
                vec![Self::ReturnToContinuationDecodeFailed]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationImportGlobalLoadUnimplemented => {
                vec![Self::ReturnToContinuationImportGlobalLoadUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationUnsupportedInstruction => {
                vec![Self::ReturnToContinuationUnsupportedInstruction]
            }
        }
    }

    const fn from_objc_helper_execution_blocker(
        blocker: &B8DebugObjcHelperExecutionBlocker,
    ) -> Self {
        match blocker {
            B8DebugObjcHelperExecutionBlocker::ReceiverIdentityUnavailable => {
                Self::ObjcReceiverMaterializationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::SelectorVmAddressUnavailable => {
                Self::ObjcSelectorMaterializationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ObjcHelperExecutionUnimplemented => {
                Self::ObjcHelperExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ObjcHelperReturnContinuationUnimplemented => {
                Self::ObjcHelperReturnContinuationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionFailed => {
                Self::ObjcRuntimeHelperExecutionFailed
            }
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionUnsupported => {
                Self::ObjcRuntimeHelperExecutionUnsupported
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented => {
                Self::ReturnToContinuationExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationDecodeFailed => {
                Self::ReturnToContinuationDecodeFailed
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationImportGlobalLoadUnimplemented => {
                Self::ReturnToContinuationImportGlobalLoadUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationUnsupportedInstruction => {
                Self::ReturnToContinuationUnsupportedInstruction
            }
        }
    }

    const fn from_objc_materialization_blocker(
        blocker: B8DebugObjcMessageMaterializationBlocker,
    ) -> Self {
        match blocker {
            B8DebugObjcMessageMaterializationBlocker::ReceiverRegisterDefinitionUnavailable
            | B8DebugObjcMessageMaterializationBlocker::ReceiverMappedImageQwordUnavailable
            | B8DebugObjcMessageMaterializationBlocker::ReceiverMappedValueFixupResolutionUnimplemented => {
                Self::ObjcReceiverMaterializationUnimplemented
            }
            B8DebugObjcMessageMaterializationBlocker::SelectorRegisterDefinitionUnavailable
            | B8DebugObjcMessageMaterializationBlocker::SelectorMappedImageQwordUnavailable
            | B8DebugObjcMessageMaterializationBlocker::SelectorMappedValueFixupResolutionUnimplemented => {
                Self::ObjcSelectorMaterializationUnimplemented
            }
            B8DebugObjcMessageMaterializationBlocker::ObjcHelperExecutionUnimplemented => {
                Self::ObjcHelperExecutionUnimplemented
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugImportBoundaryResolution {
    RequiresPublicDyldChainedFixupsDecoder,
    RequiresPublicDyldBindOpcodeDecoder,
    ResolvedPublicDyldChainedFixupsImport,
    MissingPublicBindMetadata,
    NoRegisterIndirectCallBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugImportBoundaryNextAction {
    DefineObjcReceiverSelectorMaterialization,
    DecodePublicDyldChainedFixupsImports,
    DecodePublicDyldBindOpcodes,
    InspectUnsupportedLoaderMetadata,
    InspectNextDebugBundleBlocker,
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
        let next_action = match error {
            FunctionRunError::Emit(EmitError::UnsupportedIr {
                reason: UnsupportedReason::RegisterIndirectCallUnsupported { .. },
            }) => B8DebugNextAction::ConnectPublicRebaseBindImportBoundary,
            FunctionRunError::Decode(_)
            | FunctionRunError::Lift(_)
            | FunctionRunError::Emit(_)
            | FunctionRunError::StandaloneArtifact(_)
            | FunctionRunError::InputMemory(_)
            | FunctionRunError::StdoutTrap(_)
            | FunctionRunError::Run(_) => B8DebugNextAction::AdvanceToNextIsaBlocker,
        };

        Self::blocked_with_next_action(
            B8DebugBlocker::from_failure_kind(error.failure_kind()),
            error.failure_kind(),
            error.to_string(),
            next_action,
        )
    }

    fn blocked(
        current_blocker: B8DebugBlocker,
        failure_kind: FailureKind,
        message: String,
    ) -> Self {
        Self::blocked_with_next_action(
            current_blocker,
            failure_kind,
            message,
            B8DebugNextAction::AdvanceToNextIsaBlocker,
        )
    }

    fn blocked_with_next_action(
        current_blocker: B8DebugBlocker,
        failure_kind: FailureKind,
        message: String,
        next_action: B8DebugNextAction,
    ) -> Self {
        Self {
            schema: "b8_debug_blocker_v0",
            status: B8DebugBlockerStatus::Blocked,
            current_blocker,
            failure_kind: Some(failure_kind),
            unsupported_instruction: None,
            message: Some(message),
            next_action,
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
    ConnectPublicRebaseBindImportBoundary,
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
