use bara_arm64::EmitError;
use bara_ir::UnsupportedReason;
use bara_isa_x86::{
    DecodeError, DecodedFunction, DecodedInstruction, DecodedInstructionKind, LiftError,
};
use bara_oracle::{FailureKind, TestCase};
use serde::Serialize;

use super::helper_boundary::B8DebugHelperBoundaryRequestReport;
use super::{
    encode_lower_hex, B8DebugRegisterIndirectCallBoundaryReport,
    B8DebugRegisterMaterializationSourceReport, B8DebugRegisterName, B8DebugTargetPointerLoadKind,
    B8DebugTargetPointerLoadReport,
};

use crate::function_run::{FunctionRunError, FunctionRunResult};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugUnsupportedInstructionReport {
    pub(super) start: u64,
    pub(super) end: u64,
    pub(super) kind: B8DebugDecodedInstructionKindReport,
}

impl B8DebugUnsupportedInstructionReport {
    pub(super) fn from_decoded(decoded: &DecodedFunction) -> Option<Self> {
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

    pub(super) fn from_instruction(instruction: &DecodedInstruction) -> Self {
        Self {
            start: instruction.start().value(),
            end: instruction.end().value(),
            kind: B8DebugDecodedInstructionKindReport::from_kind(instruction.kind()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum B8DebugArtifactReport<T> {
    Available { value: T },
    Failed { error: String },
    Skipped { reason: String },
}

impl<T> B8DebugArtifactReport<T> {
    pub(super) fn available(value: T) -> Self {
        Self::Available { value }
    }

    pub(super) fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
        }
    }

    pub(super) fn skipped(reason: impl Into<String>) -> Self {
        Self::Skipped {
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugLaunchReport {
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
    pub(super) fn from_attempt(
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

    pub(super) fn with_helper_boundary_request(
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
pub(super) struct B8DebugProcessedPcRange {
    start: u64,
    end: u64,
}

impl B8DebugProcessedPcRange {
    pub(super) fn from_decoded(decoded: &DecodedFunction) -> Self {
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
pub(super) struct B8DebugEntryBytesReport {
    schema: &'static str,
    case_id: String,
    source: B8DebugEntrySource,
    source_isa: B8DebugSourceIsa,
    source_pc: u64,
    byte_len: usize,
    bytes_hex: String,
}

impl B8DebugEntryBytesReport {
    pub(super) fn real_lc_main_entry(test_case: &TestCase) -> Self {
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
pub(super) enum B8DebugEntrySource {
    PublicLcMainEntryoff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) enum B8DebugSourceIsa {
    #[serde(rename = "x86_64")]
    X8664,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugDecodeReport {
    schema: &'static str,
    status: B8DebugStageStatus,
    entry: Option<u64>,
    pub(super) instructions: Vec<B8DebugDecodedInstructionReport>,
    error: Option<String>,
}

impl B8DebugDecodeReport {
    pub(super) fn from_result(decoded: Result<&DecodedFunction, &DecodeError>) -> Self {
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

    pub(super) fn register_indirect_call_r14_boundary(
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

    pub(super) fn last_r14_load_before(
        &self,
        call_site: u64,
    ) -> Option<B8DebugTargetPointerLoadReport> {
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

    pub(super) fn last_register_materialization_before(
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
pub(super) struct B8DebugDecodedInstructionReport {
    pub(super) start: u64,
    pub(super) end: u64,
    pub(super) kind: B8DebugDecodedInstructionKindReport,
}

impl B8DebugDecodedInstructionReport {
    pub(super) fn from_instruction(instruction: &DecodedInstruction) -> Self {
        Self {
            start: instruction.start().value(),
            end: instruction.end().value(),
            kind: B8DebugDecodedInstructionKindReport::from_kind(instruction.kind()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum B8DebugDecodedInstructionKindReport {
    MovEaxImm32 {
        imm: u32,
    },
    MovRaxRdi,
    MovRbxRax,
    MovRdiRbx,
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
    MovRdxRax,
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
    AddRspImm8 {
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
    PopRbx,
    PopRbp,
    PopR14,
    PopR15,
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
            DecodedInstructionKind::MovRdiRbx => Self::MovRdiRbx,
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
            DecodedInstructionKind::MovRdxRax => Self::MovRdxRax,
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
            DecodedInstructionKind::AddRspImm8 { imm } => Self::AddRspImm8 {
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
            DecodedInstructionKind::PopRbx => Self::PopRbx,
            DecodedInstructionKind::PopRbp => Self::PopRbp,
            DecodedInstructionKind::PopR14 => Self::PopR14,
            DecodedInstructionKind::PopR15 => Self::PopR15,
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
pub(super) enum B8DebugMemoryReadWidthReport {
    Bits64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugRuntimeAttemptReport {
    schema: &'static str,
    status: B8DebugStageStatus,
    run_scope: B8DebugRuntimeRunScope,
    return_value: Option<u64>,
    stdout: Option<String>,
    error: Option<String>,
}

impl B8DebugRuntimeAttemptReport {
    pub(super) fn from_result(
        result: &FunctionRunResult,
        run_scope: B8DebugRuntimeRunScope,
    ) -> Self {
        Self {
            schema: "b8_debug_runtime_attempt_v0",
            status: B8DebugStageStatus::Executed,
            run_scope,
            return_value: Some(result.return_value()),
            stdout: Some(result.stdout().to_owned()),
            error: None,
        }
    }

    pub(super) fn skipped(reason: impl Into<String>, run_scope: B8DebugRuntimeRunScope) -> Self {
        Self {
            schema: "b8_debug_runtime_attempt_v0",
            status: B8DebugStageStatus::Skipped,
            run_scope,
            return_value: None,
            stdout: None,
            error: Some(reason.into()),
        }
    }

    pub(super) fn failed(error: &FunctionRunError, run_scope: B8DebugRuntimeRunScope) -> Self {
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
pub(super) enum B8DebugRuntimeRunScope {
    RealLcMainEntryFirstBlock,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugBlockerReport {
    schema: &'static str,
    status: B8DebugBlockerStatus,
    current_blocker: B8DebugBlocker,
    failure_kind: Option<FailureKind>,
    unsupported_instruction: Option<B8DebugUnsupportedInstructionReport>,
    message: Option<String>,
    next_action: B8DebugNextAction,
}

impl B8DebugBlockerReport {
    pub(super) fn none() -> Self {
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

    pub(super) fn from_decode_error(error: &DecodeError) -> Self {
        Self::blocked(
            B8DebugBlocker::DecodeError,
            FailureKind::DecodeError,
            format!("{error:?}"),
        )
    }

    pub(super) fn from_lift_error(error: &LiftError) -> Self {
        Self::blocked(
            B8DebugBlocker::LiftError,
            FailureKind::LiftError,
            format!("{error:?}"),
        )
    }

    pub(super) fn from_unsupported_instruction(
        instruction: &B8DebugUnsupportedInstructionReport,
    ) -> Self {
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

    pub(super) fn from_function_error(error: &FunctionRunError) -> Self {
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
            | FunctionRunError::TranslationArtifact(_)
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
pub(super) enum B8DebugStageStatus {
    Executed,
    Failed,
    Skipped,
}
