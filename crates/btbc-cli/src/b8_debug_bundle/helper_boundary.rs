use bara_ir::ProgramImageMetadata;
use bara_isa_x86::X86Bytes;
use bara_oracle::{BinaryFormatProbeReport, BinaryInput, MachOChainedImportIdentityReport};
use bara_runtime::MachOExecutableImageSnapshot;
use serde::Serialize;

use super::report::{B8DebugDecodeReport, B8DebugMemoryReadWidthReport, B8DebugSourceIsa};
use super::{
    B8DebugImportBoundaryStatus, B8DebugObjcHelperExecutionBlocker,
    B8DebugObjcHelperExecutionRequestContext, B8DebugObjcHelperExecutionRequestReport,
    B8DebugObjcMessageMaterializationBlocker, B8DebugObjcMessageMaterializationBoundaryReport,
    B8DebugObjcMessageMaterializationNextAction, B8DebugRegisterIndirectCallBoundaryReport,
    B8DebugRegisterName, B8DebugReturnToContinuationSavedRegisterValueReport,
    B8DebugValueMaterializationStatus,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugHelperBoundaryRequestReport {
    status: B8DebugImportBoundaryStatus,
    reason: Option<B8DebugHelperBoundaryBlockedReason>,
    request: Option<B8DebugImportHelperRequestReport>,
    blockers: Vec<B8DebugHelperBoundaryBlocker>,
}

impl B8DebugHelperBoundaryRequestReport {
    pub(super) const fn status(&self) -> B8DebugImportBoundaryStatus {
        self.status
    }

    pub(super) fn blocked(reason: B8DebugHelperBoundaryBlockedReason) -> Self {
        let blockers = B8DebugHelperBoundaryBlocker::from_reason(reason);
        Self {
            status: B8DebugImportBoundaryStatus::Blocked,
            reason: Some(reason),
            request: None,
            blockers,
        }
    }

    pub(super) fn blocked_import_helper_call(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        import_identity: MachOChainedImportIdentityReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        decode_report: &B8DebugDecodeReport,
        code_bytes: &X86Bytes,
        mach_o_snapshot: &MachOExecutableImageSnapshot,
    ) -> Self {
        let request = B8DebugImportHelperRequestReport::from_boundary_and_import(
            call_boundary,
            import_identity,
            input,
            input_probe,
            decode_report,
            code_bytes,
            mach_o_snapshot,
        );
        let reason = request.boundary_blocked_reason();
        let blockers = request.boundary_blockers();
        let status = if blockers.is_empty() {
            B8DebugImportBoundaryStatus::Executed
        } else {
            B8DebugImportBoundaryStatus::Blocked
        };
        Self {
            status,
            reason,
            request: Some(request),
            blockers,
        }
    }

    pub(super) fn skipped() -> Self {
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
    preserved_register_values: Vec<B8DebugReturnToContinuationSavedRegisterValueReport>,
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
        mach_o_snapshot: &MachOExecutableImageSnapshot,
    ) -> Self {
        let image_metadata = mach_o_snapshot.program_image_metadata();
        let required_marshaling = B8DebugHelperMarshalingReport::blocked(
            call_boundary,
            input,
            input_probe,
            decode_report,
            &image_metadata,
        );
        let preserved_register_values =
            B8DebugReturnToContinuationSavedRegisterValueReport::from_decode_report(
                decode_report,
                call_boundary.call_site,
                input,
                input_probe,
            );
        let helper_execution_request =
            B8DebugObjcHelperExecutionRequestReport::from_import_and_marshaling(
                call_boundary,
                &import,
                &required_marshaling,
                B8DebugObjcHelperExecutionRequestContext {
                    preserved_register_values: preserved_register_values.clone(),
                    input,
                    input_probe,
                    code_bytes,
                    image_metadata: &image_metadata,
                },
            );
        Self {
            kind: B8DebugImportHelperRequestKind::ImportHelperCall,
            source: B8DebugImportHelperRequestSource::PublicDyldChainedFixupsImport,
            source_isa: B8DebugSourceIsa::X8664,
            target_register: call_boundary.target_register,
            call_site: call_boundary.call_site,
            return_to: call_boundary.return_to,
            import,
            preserved_register_values,
            required_marshaling,
            helper_execution_request,
        }
    }

    fn boundary_blocked_reason(&self) -> Option<B8DebugHelperBoundaryBlockedReason> {
        if let Some(helper_execution_request) = &self.helper_execution_request {
            helper_execution_request.boundary_blocked_reason()
        } else {
            Some(B8DebugHelperBoundaryBlockedReason::ImportHelperMarshalingUnimplemented)
        }
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
pub(super) struct B8DebugHelperMarshalingReport {
    status: B8DebugImportBoundaryStatus,
    argument_model: B8DebugHelperArgumentModel,
    return_model: B8DebugHelperReturnModel,
    pub(super) contract: Option<B8DebugImportHelperMarshalingContractReport>,
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
pub(super) struct B8DebugImportHelperMarshalingContractReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    calling_convention: B8DebugHelperCallingConvention,
    argument_sources: Vec<B8DebugHelperArgumentSourceReport>,
    return_destination: B8DebugHelperReturnDestinationReport,
    pub(super) materialization_boundary: B8DebugObjcMessageMaterializationBoundaryReport,
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
pub(super) enum B8DebugHelperCallingConvention {
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
pub(super) enum B8DebugHelperArgumentRole {
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
pub(super) enum B8DebugHelperReturnRole {
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
pub(super) enum B8DebugHelperBoundaryBlockedReason {
    ImportHelperMarshalingUnimplemented,
    ImportSymbolIdentityUnresolved,
    ObjcHelperReturnContinuationUnimplemented,
    ObjcRuntimeHelperExecutionFailed,
    ObjcRuntimeHelperExecutionUnsupported,
    ReturnToContinuationDecodeFailed,
    ReturnToContinuationCallRel32HelperExecutionUnimplemented,
    ReturnToContinuationCallRel32StubSymbolResolutionUnresolved,
    ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented,
    ReturnToContinuationExecutionUnimplemented,
    ReturnToContinuationImportGlobalLoadUnimplemented,
    ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented,
    ReturnToContinuationObjcAllocInitClassBridgeUnimplemented,
    ReturnToContinuationObjcAllocInitClassIdentityUnresolved,
    ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented,
    ReturnToContinuationObjcHelperExecutionUnimplemented,
    ReturnToContinuationSavedRegisterValueMaterializationUnimplemented,
    ReturnToContinuationUnsupportedInstruction,
}

impl B8DebugHelperBoundaryBlockedReason {
    pub(super) const fn from_objc_helper_execution_blocker(
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
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32HelperExecutionUnimplemented => {
                Self::ReturnToContinuationCallRel32HelperExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved => {
                Self::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented => {
                Self::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationImportGlobalLoadUnimplemented => {
                Self::ReturnToContinuationImportGlobalLoadUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented => {
                Self::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented => {
                Self::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassIdentityUnresolved => {
                Self::ReturnToContinuationObjcAllocInitClassIdentityUnresolved
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented => {
                Self::ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcHelperExecutionUnimplemented => {
                Self::ReturnToContinuationObjcHelperExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented => {
                Self::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented
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
pub(super) enum B8DebugHelperBoundaryBlocker {
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
    ReturnToContinuationCallRel32HelperExecutionUnimplemented,
    ReturnToContinuationCallRel32StubSymbolResolutionUnresolved,
    ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented,
    ReturnToContinuationExecutionUnimplemented,
    ReturnToContinuationImportGlobalLoadUnimplemented,
    ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented,
    ReturnToContinuationObjcAllocInitClassBridgeUnimplemented,
    ReturnToContinuationObjcAllocInitClassIdentityUnresolved,
    ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented,
    ReturnToContinuationObjcHelperExecutionUnimplemented,
    ReturnToContinuationSavedRegisterValueMaterializationUnimplemented,
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
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationCallRel32HelperExecutionUnimplemented => {
                vec![Self::ReturnToContinuationCallRel32HelperExecutionUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved => {
                vec![Self::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented => {
                vec![Self::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationImportGlobalLoadUnimplemented => {
                vec![Self::ReturnToContinuationImportGlobalLoadUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented => {
                vec![Self::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented => {
                vec![Self::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationObjcAllocInitClassIdentityUnresolved => {
                vec![Self::ReturnToContinuationObjcAllocInitClassIdentityUnresolved]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented => {
                vec![Self::ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationObjcHelperExecutionUnimplemented => {
                vec![Self::ReturnToContinuationObjcHelperExecutionUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented => {
                vec![Self::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented]
            }
            B8DebugHelperBoundaryBlockedReason::ReturnToContinuationUnsupportedInstruction => {
                vec![Self::ReturnToContinuationUnsupportedInstruction]
            }
        }
    }

    pub(super) const fn from_objc_helper_execution_blocker(
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
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32HelperExecutionUnimplemented => {
                Self::ReturnToContinuationCallRel32HelperExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved => {
                Self::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented => {
                Self::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationImportGlobalLoadUnimplemented => {
                Self::ReturnToContinuationImportGlobalLoadUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented => {
                Self::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented => {
                Self::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassIdentityUnresolved => {
                Self::ReturnToContinuationObjcAllocInitClassIdentityUnresolved
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented => {
                Self::ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcHelperExecutionUnimplemented => {
                Self::ReturnToContinuationObjcHelperExecutionUnimplemented
            }
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented => {
                Self::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented
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
