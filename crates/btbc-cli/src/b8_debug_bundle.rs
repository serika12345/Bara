use std::{
    error::Error,
    fmt, fs, io as std_io,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use bara_ir::{ProgramImageMappedBytes, ProgramImageMetadata, UnsupportedReason, X86Va};
use bara_isa_x86::{decode_function, DecodedFunction, DecodedInstructionKind, X86Bytes};
use bara_oracle::{
    binary_format_probe_report_to_json, decode_mach_o_chained_fixups_for_target,
    mach_o_entry_function_input, probe_public_binary_format, resolve_mach_o_symbol_for_x86_va,
    resolve_mach_o_symbol_stub_for_target, BinaryFileBytes, BinaryFormatProbeError,
    BinaryFormatProbeReport, BinaryInput, JsonError, MachOChainedFixupTargetAddress,
    MachOChainedFixupsBlocker, MachOChainedImportIdentityReport,
    MachOChainedRebaseTargetIdentityReport, MachOEntryFunctionTestCaseError,
    MachOStubSymbolResolution, MachOStubSymbolResolutionBlocker, MachOStubSymbolResolutionStatus,
    MachOStubVirtualAddress, MachOSymbolAddressResolution, MachOSymbolAddressResolutionBlocker,
    MachOSymbolAddressResolutionStatus,
};
use serde::{Deserialize, Serialize};

mod attempt;
mod helper_boundary;
mod import_boundary;
mod io;
mod loader;
mod report;

use self::attempt::B8RealEntryAttempt;
use self::helper_boundary::{
    B8DebugHelperArgumentRole, B8DebugHelperBoundaryBlockedReason, B8DebugHelperBoundaryBlocker,
    B8DebugHelperCallingConvention, B8DebugHelperMarshalingReport, B8DebugHelperReturnRole,
};
use self::io::{
    create_dir, read_binary_file, write_binary_file, write_json_file, write_text_file,
    B8DebugBundleOutputPaths, B8DebugReproScript,
};
use self::loader::B8DebugLoaderPlanReport;
use self::report::{
    B8DebugDecodeReport, B8DebugDecodedInstructionKindReport, B8DebugDecodedInstructionReport,
    B8DebugEntryBytesReport, B8DebugMemoryReadWidthReport, B8DebugProcessedPcRange,
    B8DebugSourceIsa, B8DebugUnsupportedInstructionReport,
};

use crate::x86_64_mach_o_fixture::{b8_gui_hello_world_case_id, X8664MachOFixtureError};

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
        .with_helper_boundary_request(loader_plan.helper_boundary_request());
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
    Rbx,
    Rbp,
    Rdx,
    Rdi,
    Rsi,
    Rsp,
    R14,
    R15,
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
    preserved_register_values: Vec<B8DebugReturnToContinuationSavedRegisterValueReport>,
    bridge_contract: B8DebugObjcRuntimeHelperBridgeContractReport,
    host_execution: B8DebugObjcRuntimeHelperHostExecutionReport,
    return_continuation: Option<B8DebugObjcHelperReturnContinuationBoundaryReport>,
    blockers: Vec<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugObjcHelperExecutionNextAction,
}

struct B8DebugObjcHelperExecutionRequestContext<'a> {
    preserved_register_values: Vec<B8DebugReturnToContinuationSavedRegisterValueReport>,
    input: &'a BinaryInput,
    input_probe: &'a BinaryFormatProbeReport,
    code_bytes: &'a X86Bytes,
    image_metadata: &'a ProgramImageMetadata,
}

impl B8DebugObjcHelperExecutionRequestReport {
    fn from_import_and_marshaling(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        import: &MachOChainedImportIdentityReport,
        marshaling: &B8DebugHelperMarshalingReport,
        context: B8DebugObjcHelperExecutionRequestContext<'_>,
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
            context.image_metadata,
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
                context.preserved_register_values.clone(),
                context.input,
                context.input_probe,
                context.code_bytes,
                context.image_metadata,
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
        let status = if blockers.is_empty() && host_execution.is_executed() {
            B8DebugImportBoundaryStatus::Executed
        } else if host_execution.is_skipped() {
            B8DebugImportBoundaryStatus::Skipped
        } else {
            B8DebugImportBoundaryStatus::Blocked
        };
        let next_action = if blockers
            .iter()
            .any(|blocker| blocker.requires_materialization_inspection())
        {
            B8DebugObjcHelperExecutionNextAction::InspectObjcMessageMaterializationBoundary
        } else if blockers.is_empty() {
            B8DebugObjcHelperExecutionNextAction::ReviewB8HelloWorldGuiCompletion
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
            status,
            kind: B8DebugObjcHelperExecutionRequestKind::ObjcMsgSend,
            source_import: import.clone(),
            receiver_identity,
            selector_vm_address,
            return_writeback_boundary,
            required_capability,
            preserved_register_values: context.preserved_register_values,
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
    ReviewB8HelloWorldGuiCompletion,
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
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugObjcHelperReturnContinuationNextAction,
}

impl B8DebugObjcHelperReturnContinuationBoundaryReport {
    fn from_host_execution(
        call_boundary: B8DebugRegisterIndirectCallBoundaryReport,
        host_execution: &B8DebugObjcRuntimeHelperHostExecutionReport,
        preserved_register_values: Vec<B8DebugReturnToContinuationSavedRegisterValueReport>,
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
            preserved_r15_value: None,
            preserved_r15_fixup_resolution: None,
            preserved_register_values,
        };
        let continuation_block = B8DebugReturnToContinuationDecodeBoundaryReport::from_code_bytes(
            call_boundary.return_to,
            Some(register_state),
            continuation_inputs,
            code_bytes,
            input,
            input_probe,
            image_metadata,
        );
        let blocker = continuation_block.as_ref().map_or(
            Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented),
            B8DebugReturnToContinuationDecodeBoundaryReport::blocker,
        );
        let next_action = continuation_block.as_ref().map_or(
            B8DebugObjcHelperReturnContinuationNextAction::DecodeReturnToContinuationBlock,
            B8DebugReturnToContinuationDecodeBoundaryReport::next_action,
        );
        let status = if blocker.is_none() {
            B8DebugImportBoundaryStatus::Executed
        } else {
            B8DebugImportBoundaryStatus::Blocked
        };
        Some(Self {
            schema: "b8_objc_helper_return_continuation_boundary_v0",
            status,
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
        self.blocker.iter().copied().collect()
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
    DefineReturnToContinuationObjcAllocInitClassBridge,
    ImplementReturnToContinuationCallRel32HelperExecution,
    ImplementReturnToContinuationObjcAllocInitFixtureDelegateHostExecution,
    ImplementReturnToContinuationObjcHelperExecution,
    ImplementReturnToContinuationExecution,
    MaterializeReturnToContinuationObjcAllocInitClassArgument,
    MaterializeReturnToContinuationSavedRegisterValue,
    ReviewB8HelloWorldGuiCompletion,
    ResolveReturnToContinuationObjcAllocInitClassIdentity,
    ResolveReturnToContinuationCallRel32StubSymbol,
    MaterializeReturnToContinuationCallRel32ReturnValue,
    MaterializeReturnToContinuationImportGlobalLoad,
    InspectReturnToContinuationObjcHelperExecutionFailure,
    InspectReturnToContinuationDecodeFailure,
    RunReturnToContinuationObjcHelperOnSupportedMacosHost,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationDecodeBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    source: B8DebugReturnToContinuationDecodeSourceReport,
    input_register_state: Option<B8DebugObjcHelperReturnContinuationRegisterStateReport>,
    materialized_register_states: Vec<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
    blocked_register_materializations:
        Vec<B8DebugReturnToContinuationBlockedRegisterMaterializationReport>,
    autorelease_pool_pop_boundary:
        Option<B8DebugReturnToContinuationAutoreleasePoolPopBoundaryReport>,
    epilogue_stack_adjustment: Option<B8DebugReturnToContinuationEpilogueStackAdjustmentReport>,
    epilogue_register_restores: Vec<B8DebugReturnToContinuationEpilogueRegisterRestoreReport>,
    epilogue_return_completion: Option<B8DebugReturnToContinuationEpilogueReturnCompletionReport>,
    modeled_execution_completion:
        Option<B8DebugReturnToContinuationModeledExecutionCompletionReport>,
    continuation_call_boundary: Option<B8DebugReturnToContinuationCallBoundaryReport>,
    decode_report: B8DebugDecodeReport,
    processed_source_pc_range: Option<B8DebugProcessedPcRange>,
    next_instruction: Option<B8DebugDecodedInstructionReport>,
    unsupported_instruction: Option<B8DebugUnsupportedInstructionReport>,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct B8DebugReturnToContinuationDecodeInputs {
    imported_global_value: Option<B8DebugReturnToContinuationImportedGlobalValue>,
    preserved_call_target_import: Option<MachOChainedImportIdentityReport>,
    preserved_r15_value: Option<u64>,
    preserved_r15_fixup_resolution: Option<B8DebugObjcArgumentFixupResolutionReport>,
    preserved_register_values: Vec<B8DebugReturnToContinuationSavedRegisterValueReport>,
}

impl B8DebugReturnToContinuationDecodeInputs {
    fn saved_register_value(
        &self,
        register: B8DebugRegisterName,
    ) -> Option<&B8DebugReturnToContinuationSavedRegisterValueReport> {
        self.preserved_register_values
            .iter()
            .rev()
            .find(|value| value.register == register)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationEpilogueStackAdjustmentReport {
    schema: &'static str,
    status: B8DebugReturnToContinuationEpilogueStackAdjustmentStatus,
    role: B8DebugReturnToContinuationEpilogueStackAdjustmentRole,
    source: B8DebugReturnToContinuationEpilogueStackAdjustmentSource,
    instruction: B8DebugDecodedInstructionReport,
    stack_pointer_register: B8DebugRegisterName,
    stack_pointer_delta: String,
    next_blocker_after_adjustment: Option<B8DebugUnsupportedInstructionReport>,
}

impl B8DebugReturnToContinuationEpilogueStackAdjustmentReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        autorelease_pool_pop_boundary: Option<
            &B8DebugReturnToContinuationAutoreleasePoolPopBoundaryReport,
        >,
    ) -> Option<Self> {
        let autorelease_pool_pop_boundary = autorelease_pool_pop_boundary?;
        if autorelease_pool_pop_boundary.status != B8DebugImportBoundaryStatus::Executed {
            return None;
        }

        let instruction = decoded.instructions().iter().find(|instruction| {
            instruction.start().value() >= autorelease_pool_pop_boundary.return_to
                && matches!(
                    instruction.kind(),
                    DecodedInstructionKind::AddRspImm8 { .. }
                )
        })?;
        let DecodedInstructionKind::AddRspImm8 { imm } = instruction.kind() else {
            return None;
        };
        let next_blocker_after_adjustment = decoded
            .next_unsupported_instruction_before_next_return(instruction.end())
            .map(B8DebugUnsupportedInstructionReport::from_instruction);

        Some(Self {
            schema: "b8_return_to_continuation_epilogue_stack_adjustment_v0",
            status: B8DebugReturnToContinuationEpilogueStackAdjustmentStatus::Decoded,
            role: B8DebugReturnToContinuationEpilogueStackAdjustmentRole::PostRunHelperBoundaryStackRestore,
            source: B8DebugReturnToContinuationEpilogueStackAdjustmentSource::AfterAutoreleasePoolPopHelperReturn,
            instruction: B8DebugDecodedInstructionReport::from_instruction(instruction),
            stack_pointer_register: B8DebugRegisterName::Rsp,
            stack_pointer_delta: format!("{imm:?}"),
            next_blocker_after_adjustment,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueStackAdjustmentStatus {
    Decoded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueStackAdjustmentRole {
    PostRunHelperBoundaryStackRestore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueStackAdjustmentSource {
    AfterAutoreleasePoolPopHelperReturn,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationEpilogueRegisterRestoreReport {
    schema: &'static str,
    status: B8DebugReturnToContinuationEpilogueRegisterRestoreStatus,
    role: B8DebugReturnToContinuationEpilogueRegisterRestoreRole,
    source: B8DebugReturnToContinuationEpilogueRegisterRestoreSource,
    instruction: B8DebugDecodedInstructionReport,
    register: B8DebugRegisterName,
    stack_slot_source: B8DebugReturnToContinuationEpilogueRegisterRestoreStackSlotSource,
    next_blocker_after_restore: Option<B8DebugUnsupportedInstructionReport>,
}

impl B8DebugReturnToContinuationEpilogueRegisterRestoreReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        epilogue_stack_adjustment: Option<
            &B8DebugReturnToContinuationEpilogueStackAdjustmentReport,
        >,
    ) -> Vec<Self> {
        let Some(epilogue_stack_adjustment) = epilogue_stack_adjustment else {
            return Vec::new();
        };
        let mut next_start = epilogue_stack_adjustment.instruction.end;
        let mut reports = Vec::new();

        while let Some(instruction) = decoded
            .instructions()
            .iter()
            .find(|instruction| instruction.start().value() == next_start)
        {
            let Some(register) = Self::restored_register(instruction.kind()) else {
                break;
            };
            let source = if reports.is_empty() {
                B8DebugReturnToContinuationEpilogueRegisterRestoreSource::AfterEpilogueStackAdjustment
            } else {
                B8DebugReturnToContinuationEpilogueRegisterRestoreSource::AfterPreviousEpilogueRegisterRestore
            };
            let stack_slot_source = if reports.is_empty() {
                B8DebugReturnToContinuationEpilogueRegisterRestoreStackSlotSource::PostAdjustmentStackTop
            } else {
                B8DebugReturnToContinuationEpilogueRegisterRestoreStackSlotSource::SequentialEpilogueStackTop
            };
            let role = if register == B8DebugRegisterName::Rbp {
                B8DebugReturnToContinuationEpilogueRegisterRestoreRole::PostRunEpilogueFramePointerRestore
            } else {
                B8DebugReturnToContinuationEpilogueRegisterRestoreRole::PostRunEpiloguePreservedRegisterRestore
            };
            let next_blocker_after_restore = decoded
                .next_unsupported_instruction_before_next_return(instruction.end())
                .map(B8DebugUnsupportedInstructionReport::from_instruction);

            reports.push(Self {
                schema: "b8_return_to_continuation_epilogue_register_restore_v0",
                status: B8DebugReturnToContinuationEpilogueRegisterRestoreStatus::Decoded,
                role,
                source,
                instruction: B8DebugDecodedInstructionReport::from_instruction(instruction),
                register,
                stack_slot_source,
                next_blocker_after_restore,
            });
            next_start = instruction.end().value();
        }

        reports
    }

    const fn restored_register(kind: &DecodedInstructionKind) -> Option<B8DebugRegisterName> {
        match kind {
            DecodedInstructionKind::PopRbx => Some(B8DebugRegisterName::Rbx),
            DecodedInstructionKind::PopRbp => Some(B8DebugRegisterName::Rbp),
            DecodedInstructionKind::PopR14 => Some(B8DebugRegisterName::R14),
            DecodedInstructionKind::PopR15 => Some(B8DebugRegisterName::R15),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueRegisterRestoreStatus {
    Decoded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueRegisterRestoreRole {
    PostRunEpiloguePreservedRegisterRestore,
    PostRunEpilogueFramePointerRestore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueRegisterRestoreSource {
    AfterEpilogueStackAdjustment,
    AfterPreviousEpilogueRegisterRestore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueRegisterRestoreStackSlotSource {
    PostAdjustmentStackTop,
    SequentialEpilogueStackTop,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationEpilogueReturnCompletionReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    role: B8DebugReturnToContinuationEpilogueReturnCompletionRole,
    source: B8DebugReturnToContinuationEpilogueReturnCompletionSource,
    instruction: B8DebugDecodedInstructionReport,
    post_ret_padding_boundary: Option<B8DebugReturnToContinuationPostRetPaddingBoundaryReport>,
    next_blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationEpilogueReturnCompletionReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        epilogue_register_restores: &[B8DebugReturnToContinuationEpilogueRegisterRestoreReport],
    ) -> Option<Self> {
        let last_restore = epilogue_register_restores.last()?;
        if last_restore.role
            != B8DebugReturnToContinuationEpilogueRegisterRestoreRole::PostRunEpilogueFramePointerRestore
        {
            return None;
        }
        let instruction = decoded.instructions().iter().find(|instruction| {
            instruction.start().value() == last_restore.instruction.end
                && matches!(instruction.kind(), DecodedInstructionKind::Ret)
        })?;
        let post_ret_padding_boundary =
            B8DebugReturnToContinuationPostRetPaddingBoundaryReport::from_decoded(
                decoded,
                instruction,
            );

        Some(Self {
            schema: "b8_return_to_continuation_epilogue_return_completion_v0",
            status: B8DebugImportBoundaryStatus::Executed,
            role: B8DebugReturnToContinuationEpilogueReturnCompletionRole::PostRunEpilogueReturnTerminator,
            source: B8DebugReturnToContinuationEpilogueReturnCompletionSource::AfterEpilogueFramePointerRestore,
            instruction: B8DebugDecodedInstructionReport::from_instruction(instruction),
            post_ret_padding_boundary,
            next_blocker: None,
            next_action: B8DebugReturnToContinuationDecodeNextAction::ReviewB8HelloWorldGuiCompletion,
        })
    }

    fn classifies_unsupported_instruction(
        &self,
        unsupported_instruction: Option<&B8DebugUnsupportedInstructionReport>,
    ) -> bool {
        let Some(post_ret_padding_boundary) = &self.post_ret_padding_boundary else {
            return false;
        };
        let Some(unsupported_instruction) = unsupported_instruction else {
            return false;
        };

        post_ret_padding_boundary.instruction.start == unsupported_instruction.start
            && post_ret_padding_boundary.instruction.end == unsupported_instruction.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueReturnCompletionRole {
    PostRunEpilogueReturnTerminator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationEpilogueReturnCompletionSource {
    AfterEpilogueFramePointerRestore,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationPostRetPaddingBoundaryReport {
    schema: &'static str,
    status: B8DebugReturnToContinuationPostRetPaddingBoundaryStatus,
    role: B8DebugReturnToContinuationPostRetPaddingBoundaryRole,
    instruction: B8DebugUnsupportedInstructionReport,
    classification: B8DebugReturnToContinuationPostRetPaddingClassification,
    effect: B8DebugReturnToContinuationPostRetPaddingEffect,
}

impl B8DebugReturnToContinuationPostRetPaddingBoundaryReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        return_instruction: &bara_isa_x86::DecodedInstruction,
    ) -> Option<Self> {
        let instruction = decoded.instructions().iter().find(|candidate| {
            candidate.start().value() == return_instruction.end().value()
                && matches!(
                    candidate.kind(),
                    DecodedInstructionKind::Unsupported {
                        reason: UnsupportedReason::DecodeUnsupportedOpcode { opcode: 0, .. }
                    }
                )
        })?;

        Some(Self {
            schema: "b8_return_to_continuation_post_ret_padding_boundary_v0",
            status: B8DebugReturnToContinuationPostRetPaddingBoundaryStatus::Classified,
            role:
                B8DebugReturnToContinuationPostRetPaddingBoundaryRole::PostRunTrailingZeroPaddingAfterReturn,
            instruction: B8DebugUnsupportedInstructionReport::from_instruction(instruction),
            classification:
                B8DebugReturnToContinuationPostRetPaddingClassification::IgnoredAfterReturnTerminator,
            effect: B8DebugReturnToContinuationPostRetPaddingEffect::DoesNotExtendFunctionBody,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationPostRetPaddingBoundaryStatus {
    Classified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationPostRetPaddingBoundaryRole {
    PostRunTrailingZeroPaddingAfterReturn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationPostRetPaddingClassification {
    IgnoredAfterReturnTerminator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationPostRetPaddingEffect {
    DoesNotExtendFunctionBody,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationModeledExecutionCompletionReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    role: B8DebugReturnToContinuationModeledExecutionCompletionRole,
    source: B8DebugReturnToContinuationModeledExecutionCompletionSource,
    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope,
    completion_model: B8DebugReturnToContinuationModeledExecutionCompletionModel,
    launch_path_status: B8DebugReturnToContinuationModeledExecutionLaunchPathStatus,
    remaining_b8_hwgui_blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    automated_expected_actual_comparison:
        B8DebugReturnToContinuationModeledExecutionReviewItemReport,
    manual_visible_mode: B8DebugReturnToContinuationModeledExecutionReviewItemReport,
}

impl B8DebugReturnToContinuationModeledExecutionCompletionReport {
    fn from_boundaries(
        continuation_inputs: &B8DebugReturnToContinuationDecodeInputs,
        autorelease_pool_pop_boundary: Option<
            &B8DebugReturnToContinuationAutoreleasePoolPopBoundaryReport,
        >,
        epilogue_return_completion: Option<
            &B8DebugReturnToContinuationEpilogueReturnCompletionReport,
        >,
    ) -> Option<Self> {
        let imported_global_value = continuation_inputs.imported_global_value?;
        if imported_global_value.symbol != B8DebugReturnToContinuationImportedGlobalSymbol::NsApp {
            return None;
        }
        let autorelease_pool_pop_boundary = autorelease_pool_pop_boundary?;
        if autorelease_pool_pop_boundary.status != B8DebugImportBoundaryStatus::Executed {
            return None;
        }
        let epilogue_return_completion = epilogue_return_completion?;
        if epilogue_return_completion.status != B8DebugImportBoundaryStatus::Executed
            || epilogue_return_completion
                .post_ret_padding_boundary
                .is_none()
        {
            return None;
        }

        Some(Self {
            schema: "b8_return_to_continuation_modeled_execution_completion_v0",
            status: B8DebugImportBoundaryStatus::Executed,
            role:
                B8DebugReturnToContinuationModeledExecutionCompletionRole::SelfAuthoredHelloWorldGuiLaunchPath,
            source:
                B8DebugReturnToContinuationModeledExecutionCompletionSource::PostRunEpilogueReturnCompletion,
            fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
            completion_model:
                B8DebugReturnToContinuationModeledExecutionCompletionModel::ModeledRealEntryHelperContinuationChain,
            launch_path_status:
                B8DebugReturnToContinuationModeledExecutionLaunchPathStatus::Completed,
            remaining_b8_hwgui_blocker: None,
            automated_expected_actual_comparison:
                B8DebugReturnToContinuationModeledExecutionReviewItemReport::pending_large_target_review(
                    B8DebugReturnToContinuationModeledExecutionReviewItem::AutomatedExpectedActualComparison,
                ),
            manual_visible_mode:
                B8DebugReturnToContinuationModeledExecutionReviewItemReport::pending_large_target_review(
                    B8DebugReturnToContinuationModeledExecutionReviewItem::ManualVisibleMode,
                ),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationModeledExecutionCompletionRole {
    SelfAuthoredHelloWorldGuiLaunchPath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationModeledExecutionCompletionSource {
    PostRunEpilogueReturnCompletion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationModeledExecutionCompletionModel {
    ModeledRealEntryHelperContinuationChain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationModeledExecutionLaunchPathStatus {
    Completed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationModeledExecutionReviewItemReport {
    item: B8DebugReturnToContinuationModeledExecutionReviewItem,
    remaining_difference: B8DebugReturnToContinuationModeledExecutionReviewRemainingDifference,
}

impl B8DebugReturnToContinuationModeledExecutionReviewItemReport {
    const fn pending_large_target_review(
        item: B8DebugReturnToContinuationModeledExecutionReviewItem,
    ) -> Self {
        Self {
            item,
            remaining_difference:
                B8DebugReturnToContinuationModeledExecutionReviewRemainingDifference::PendingLargeTargetReview,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationModeledExecutionReviewItem {
    AutomatedExpectedActualComparison,
    ManualVisibleMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationModeledExecutionReviewRemainingDifference {
    PendingLargeTargetReview,
}

trait B8DebugDecodedFunctionExt {
    fn next_unsupported_instruction_before_next_return(
        &self,
        start: X86Va,
    ) -> Option<&bara_isa_x86::DecodedInstruction>;
}

impl B8DebugDecodedFunctionExt for DecodedFunction {
    fn next_unsupported_instruction_before_next_return(
        &self,
        start: X86Va,
    ) -> Option<&bara_isa_x86::DecodedInstruction> {
        let next_return_start = self
            .instructions()
            .iter()
            .find(|candidate| {
                candidate.start().value() >= start.value()
                    && matches!(candidate.kind(), DecodedInstructionKind::Ret)
            })
            .map(|instruction| instruction.start().value());

        self.instructions().iter().find(|candidate| {
            candidate.start().value() >= start.value()
                && next_return_start
                    .map(|return_start| candidate.start().value() < return_start)
                    .unwrap_or(true)
                && matches!(candidate.kind(), DecodedInstructionKind::Unsupported { .. })
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct B8DebugReturnToContinuationHostExecutionContext<'a> {
    code_bytes: &'a X86Bytes,
    input: &'a BinaryInput,
    input_probe: &'a BinaryFormatProbeReport,
    image_metadata: &'a ProgramImageMetadata,
    continuation_inputs: &'a B8DebugReturnToContinuationDecodeInputs,
}

impl B8DebugReturnToContinuationDecodeBoundaryReport {
    fn from_code_bytes(
        source_pc: u64,
        input_register_state: Option<B8DebugObjcHelperReturnContinuationRegisterStateReport>,
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
            autorelease_pool_pop_boundary,
            epilogue_stack_adjustment,
            epilogue_register_restores,
            epilogue_return_completion,
            modeled_execution_completion,
            continuation_call_boundary,
            blocker,
            next_action,
        ) = match decoded_result {
            Ok(decoded) => {
                let decoded_unsupported_instruction =
                    B8DebugUnsupportedInstructionReport::from_decoded(&decoded);
                let (materialized_register_states, blocked_register_materializations) =
                    B8DebugReturnToContinuationMaterializedRegisterStateReport::from_decoded(
                        &decoded,
                        image_metadata.mapped_bytes(),
                        input,
                        input_probe,
                        &continuation_inputs,
                    );
                let host_execution_context = B8DebugReturnToContinuationHostExecutionContext {
                    code_bytes,
                    input,
                    input_probe,
                    image_metadata,
                    continuation_inputs: &continuation_inputs,
                };
                let autorelease_pool_pop_boundary =
                    B8DebugReturnToContinuationAutoreleasePoolPopBoundaryReport::from_decoded(
                        &decoded,
                        &materialized_register_states,
                        input,
                        input_probe,
                    );
                let epilogue_stack_adjustment =
                    B8DebugReturnToContinuationEpilogueStackAdjustmentReport::from_decoded(
                        &decoded,
                        autorelease_pool_pop_boundary.as_ref(),
                    );
                let epilogue_register_restores =
                    B8DebugReturnToContinuationEpilogueRegisterRestoreReport::from_decoded(
                        &decoded,
                        epilogue_stack_adjustment.as_ref(),
                    );
                let epilogue_return_completion =
                    B8DebugReturnToContinuationEpilogueReturnCompletionReport::from_decoded(
                        &decoded,
                        &epilogue_register_restores,
                    );
                let modeled_execution_completion =
                    B8DebugReturnToContinuationModeledExecutionCompletionReport::from_boundaries(
                        &continuation_inputs,
                        autorelease_pool_pop_boundary.as_ref(),
                        epilogue_return_completion.as_ref(),
                    );
                let unsupported_instruction = if epilogue_return_completion
                    .as_ref()
                    .is_some_and(|completion| {
                        completion.classifies_unsupported_instruction(
                            decoded_unsupported_instruction.as_ref(),
                        )
                    }) {
                    None
                } else {
                    decoded_unsupported_instruction
                };
                let continuation_call_boundary =
                    B8DebugReturnToContinuationCallBoundaryReport::from_decoded(
                        &decoded,
                        &materialized_register_states,
                        continuation_inputs.preserved_call_target_import.clone(),
                        host_execution_context,
                    );
                let materialization_blocker =
                    blocked_register_materializations.first().map(|blocked| blocked.blocker);
                let autorelease_pool_pop_blocker = autorelease_pool_pop_boundary
                    .as_ref()
                    .and_then(|boundary| boundary.blocker);
                let continuation_call_blocker = continuation_call_boundary
                    .as_ref()
                    .and_then(|boundary| boundary.blocker);
                let completed_continuation_call = continuation_call_boundary
                    .as_ref()
                    .is_some_and(|boundary| boundary.blocker.is_none());
                let blocker = if let Some(blocker) = materialization_blocker {
                    Some(blocker)
                } else if let Some(blocker) = autorelease_pool_pop_blocker {
                    Some(blocker)
                } else if unsupported_instruction.is_some() {
                    Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationUnsupportedInstruction)
                } else if let Some(blocker) = continuation_call_blocker {
                    Some(blocker)
                } else if modeled_execution_completion.is_some() || completed_continuation_call {
                    None
                } else {
                    Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented)
                };
                let next_action = if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationImportGlobalLoadUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationImportGlobalLoad
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32HelperExecutionUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationCallRel32HelperExecution
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::ResolveReturnToContinuationCallRel32StubSymbol
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationObjcAllocInitClassArgument
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::DefineReturnToContinuationObjcAllocInitClassBridge
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassIdentityUnresolved,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::ResolveReturnToContinuationObjcAllocInitClassIdentity
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationObjcAllocInitFixtureDelegateHostExecution
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationCallRel32ReturnValue
                } else if materialization_blocker
                    == Some(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented,
                    )
                {
                    B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationSavedRegisterValue
                } else if let Some(boundary) = autorelease_pool_pop_boundary
                    .as_ref()
                    .filter(|boundary| boundary.blocker.is_some())
                {
                    boundary.next_action
                } else if unsupported_instruction.is_some() {
                    B8DebugReturnToContinuationDecodeNextAction::AddReturnToContinuationInstructionSupport
                } else if let Some(boundary) = continuation_call_boundary.as_ref() {
                    boundary.next_action
                } else if modeled_execution_completion.is_some() {
                    B8DebugReturnToContinuationDecodeNextAction::ReviewB8HelloWorldGuiCompletion
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
                    autorelease_pool_pop_boundary,
                    epilogue_stack_adjustment,
                    epilogue_register_restores,
                    epilogue_return_completion,
                    modeled_execution_completion,
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
                None,
                Vec::new(),
                None,
                None,
                None,
                Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationDecodeFailed),
                B8DebugReturnToContinuationDecodeNextAction::InspectReturnToContinuationDecodeFailure,
            ),
        };
        let status = if blocker.is_none() {
            B8DebugImportBoundaryStatus::Executed
        } else {
            B8DebugImportBoundaryStatus::Blocked
        };

        Some(Self {
            schema: "b8_return_to_continuation_decode_boundary_v0",
            status,
            source: B8DebugReturnToContinuationDecodeSourceReport {
                kind: B8DebugReturnToContinuationDecodeSourceKind::ReturnToSourcePc,
                source_pc,
                byte_source: B8DebugReturnToContinuationByteSource::MachOCodeSegmentBytes,
            },
            input_register_state,
            materialized_register_states,
            blocked_register_materializations,
            autorelease_pool_pop_boundary,
            epilogue_stack_adjustment,
            epilogue_register_restores,
            epilogue_return_completion,
            modeled_execution_completion,
            continuation_call_boundary,
            decode_report,
            processed_source_pc_range,
            next_instruction,
            unsupported_instruction,
            blocker,
            next_action,
        })
    }

    const fn blocker(&self) -> Option<B8DebugObjcHelperExecutionBlocker> {
        self.blocker
    }

    const fn next_action(&self) -> B8DebugObjcHelperReturnContinuationNextAction {
        match self.next_action {
            B8DebugReturnToContinuationDecodeNextAction::AddReturnToContinuationInstructionSupport => {
                B8DebugObjcHelperReturnContinuationNextAction::AddReturnToContinuationInstructionSupport
            }
            B8DebugReturnToContinuationDecodeNextAction::DefineReturnToContinuationObjcAllocInitClassBridge => {
                B8DebugObjcHelperReturnContinuationNextAction::DefineReturnToContinuationObjcAllocInitClassBridge
            }
            B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationCallRel32HelperExecution => {
                B8DebugObjcHelperReturnContinuationNextAction::ImplementReturnToContinuationCallRel32HelperExecution
            }
            B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationObjcAllocInitFixtureDelegateHostExecution => {
                B8DebugObjcHelperReturnContinuationNextAction::ImplementReturnToContinuationObjcAllocInitFixtureDelegateHostExecution
            }
            B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationObjcHelperExecution => {
                B8DebugObjcHelperReturnContinuationNextAction::ImplementReturnToContinuationObjcHelperExecution
            }
            B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution => {
                B8DebugObjcHelperReturnContinuationNextAction::ImplementReturnToContinuationExecution
            }
            B8DebugReturnToContinuationDecodeNextAction::InspectReturnToContinuationDecodeFailure => {
                B8DebugObjcHelperReturnContinuationNextAction::InspectReturnToContinuationDecodeFailure
            }
            B8DebugReturnToContinuationDecodeNextAction::InspectReturnToContinuationObjcHelperExecutionFailure => {
                B8DebugObjcHelperReturnContinuationNextAction::InspectReturnToContinuationObjcHelperExecutionFailure
            }
            B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationCallRel32ReturnValue => {
                B8DebugObjcHelperReturnContinuationNextAction::MaterializeReturnToContinuationCallRel32ReturnValue
            }
            B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationImportGlobalLoad => {
                B8DebugObjcHelperReturnContinuationNextAction::MaterializeReturnToContinuationImportGlobalLoad
            }
            B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationObjcAllocInitClassArgument => {
                B8DebugObjcHelperReturnContinuationNextAction::MaterializeReturnToContinuationObjcAllocInitClassArgument
            }
            B8DebugReturnToContinuationDecodeNextAction::MaterializeReturnToContinuationSavedRegisterValue => {
                B8DebugObjcHelperReturnContinuationNextAction::MaterializeReturnToContinuationSavedRegisterValue
            }
            B8DebugReturnToContinuationDecodeNextAction::ReviewB8HelloWorldGuiCompletion => {
                B8DebugObjcHelperReturnContinuationNextAction::ReviewB8HelloWorldGuiCompletion
            }
            B8DebugReturnToContinuationDecodeNextAction::ResolveReturnToContinuationObjcAllocInitClassIdentity => {
                B8DebugObjcHelperReturnContinuationNextAction::ResolveReturnToContinuationObjcAllocInitClassIdentity
            }
            B8DebugReturnToContinuationDecodeNextAction::ResolveReturnToContinuationCallRel32StubSymbol => {
                B8DebugObjcHelperReturnContinuationNextAction::ResolveReturnToContinuationCallRel32StubSymbol
            }
            B8DebugReturnToContinuationDecodeNextAction::RunReturnToContinuationObjcHelperOnSupportedMacosHost => {
                B8DebugObjcHelperReturnContinuationNextAction::RunReturnToContinuationObjcHelperOnSupportedMacosHost
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
    DefineReturnToContinuationObjcAllocInitClassBridge,
    ImplementReturnToContinuationCallRel32HelperExecution,
    ImplementReturnToContinuationObjcAllocInitFixtureDelegateHostExecution,
    ImplementReturnToContinuationObjcHelperExecution,
    ImplementReturnToContinuationExecution,
    InspectReturnToContinuationDecodeFailure,
    InspectReturnToContinuationObjcHelperExecutionFailure,
    MaterializeReturnToContinuationCallRel32ReturnValue,
    MaterializeReturnToContinuationImportGlobalLoad,
    MaterializeReturnToContinuationObjcAllocInitClassArgument,
    MaterializeReturnToContinuationSavedRegisterValue,
    ReviewB8HelloWorldGuiCompletion,
    ResolveReturnToContinuationObjcAllocInitClassIdentity,
    ResolveReturnToContinuationCallRel32StubSymbol,
    RunReturnToContinuationObjcHelperOnSupportedMacosHost,
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
    source_register: Option<B8DebugRegisterName>,
    source_saved_register_value: Option<B8DebugReturnToContinuationSavedRegisterValueReport>,
    source_call_return: Option<Box<B8DebugReturnToContinuationCallRel32ReturnValueReport>>,
    source_call_return_dataflow:
        Option<B8DebugReturnToContinuationCallRel32ReturnValueDataflowReport>,
    fixup_resolution: Option<B8DebugObjcArgumentFixupResolutionReport>,
    width: B8DebugMemoryReadWidthReport,
}

impl B8DebugReturnToContinuationMaterializedRegisterStateReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        mapped_bytes: &ProgramImageMappedBytes,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        continuation_inputs: &B8DebugReturnToContinuationDecodeInputs,
    ) -> (
        Vec<Self>,
        Vec<B8DebugReturnToContinuationBlockedRegisterMaterializationReport>,
    ) {
        let mut states = Vec::new();
        let mut r15_address = None;
        let mut r15_value = continuation_inputs.preserved_r15_value;
        let mut r15_fixup_resolution = continuation_inputs.preserved_r15_fixup_resolution.clone();
        let mut rax_call_return = None;
        let mut blocked = Vec::new();

        for instruction in decoded.instructions() {
            match instruction.kind() {
                DecodedInstructionKind::CallRel32 { target, return_to } => {
                    let class_argument = latest_materialized_register_state_before(
                        &states,
                        B8DebugRegisterName::Rdi,
                        instruction.start().value(),
                    )
                    .cloned();
                    rax_call_return = Some(
                        B8DebugReturnToContinuationCallRel32ReturnValueReport::from_call_rel32(
                            instruction.start().value(),
                            return_to.value(),
                            target.value(),
                            class_argument,
                            input,
                            input_probe,
                        ),
                    );
                }
                DecodedInstructionKind::MovRdiQwordPtrRipRelative { address, .. } => {
                    if let Some(value) = mapped_bytes.read_u64_le(*address) {
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
                                B8DebugReturnToContinuationMaterializedRegisterSource::RipRelativeQword,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            address: Some(address.value()),
                            base_register: None,
                            base_value: None,
                            base_fixup_resolution: None,
                            value,
                            value_source: None,
                            source_register: None,
                            source_saved_register_value: None,
                            source_call_return: None,
                            source_call_return_dataflow: None,
                            fixup_resolution: Some(fixup_resolution),
                            width: B8DebugMemoryReadWidthReport::Bits64,
                        });
                    }
                }
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
                            source_register: None,
                            source_saved_register_value: None,
                            source_call_return: None,
                            source_call_return_dataflow: None,
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
                            source_register: None,
                            source_saved_register_value: None,
                            source_call_return: None,
                            source_call_return_dataflow: None,
                            fixup_resolution: Some(fixup_resolution),
                            width: B8DebugMemoryReadWidthReport::Bits64,
                        });
                    }
                }
                DecodedInstructionKind::MovRdiQwordPtrR15 => {
                    if let Some(imported_global_value) = imported_global_value_for_resolution(
                        continuation_inputs.imported_global_value,
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
                            source_register: None,
                            source_saved_register_value: None,
                            source_call_return: None,
                            source_call_return_dataflow: None,
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
                                    base_register: Some(B8DebugRegisterName::R15),
                                    base_value: Some(base_value),
                                    base_fixup_resolution: r15_fixup_resolution.clone(),
                                    source_register: None,
                                    source_call_return: None,
                                    source_call_return_dataflow: None,
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
                                source_register: None,
                                source_saved_register_value: None,
                                source_call_return: None,
                                source_call_return_dataflow: None,
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
                        source_register: None,
                        source_saved_register_value: None,
                        source_call_return: None,
                        source_call_return_dataflow: None,
                        fixup_resolution: None,
                        width: B8DebugMemoryReadWidthReport::Bits64,
                    });
                }
                DecodedInstructionKind::MovRdxRax => {
                    let source_call_return = rax_call_return.clone();
                    let source_call_return_dataflow = source_call_return.as_ref().map(
                        |call_return| {
                            B8DebugReturnToContinuationCallRel32ReturnValueDataflowReport::from_consumer(
                                call_return,
                                instruction.start().value(),
                                instruction.end().value(),
                                B8DebugRegisterName::Rdx,
                                B8DebugRegisterName::Rax,
                            )
                        },
                    );
                    if let Some(return_value) = source_call_return.as_ref().and_then(
                        B8DebugReturnToContinuationCallRel32ReturnValueReport::return_value,
                    ) {
                        states.push(Self {
                            register: B8DebugRegisterName::Rdx,
                            source:
                                B8DebugReturnToContinuationMaterializedRegisterSource::RegisterCopyFromRax,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            address: None,
                            base_register: None,
                            base_value: None,
                            base_fixup_resolution: None,
                            value: return_value,
                            value_source: None,
                            source_register: Some(B8DebugRegisterName::Rax),
                            source_saved_register_value: None,
                            source_call_return: source_call_return.map(Box::new),
                            source_call_return_dataflow,
                            fixup_resolution: None,
                            width: B8DebugMemoryReadWidthReport::Bits64,
                        });
                        continue;
                    }
                    let blocker = source_call_return.as_ref().map_or(
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32ReturnValueMaterializationUnimplemented,
                        |call_return| call_return.helper_boundary.blocker,
                    );
                    blocked.push(
                        B8DebugReturnToContinuationBlockedRegisterMaterializationReport {
                            register: B8DebugRegisterName::Rdx,
                            source:
                                B8DebugReturnToContinuationMaterializedRegisterSource::RegisterCopyFromRax,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            base_register: None,
                            base_value: None,
                            base_fixup_resolution: None,
                            source_register: Some(B8DebugRegisterName::Rax),
                            source_call_return,
                            source_call_return_dataflow,
                            blocker,
                        },
                    );
                }
                DecodedInstructionKind::MovRdiRbx => {
                    if let Some(saved_register_value) =
                        continuation_inputs.saved_register_value(B8DebugRegisterName::Rbx)
                    {
                        states.push(Self {
                            register: B8DebugRegisterName::Rdi,
                            source:
                                B8DebugReturnToContinuationMaterializedRegisterSource::RegisterCopyFromRbx,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            address: None,
                            base_register: None,
                            base_value: None,
                            base_fixup_resolution: None,
                            value: saved_register_value.value,
                            value_source: Some(saved_register_value.value_source),
                            source_register: Some(B8DebugRegisterName::Rbx),
                            source_saved_register_value: Some(saved_register_value.clone()),
                            source_call_return: None,
                            source_call_return_dataflow: None,
                            fixup_resolution: None,
                            width: B8DebugMemoryReadWidthReport::Bits64,
                        });
                        continue;
                    }
                    blocked.push(
                        B8DebugReturnToContinuationBlockedRegisterMaterializationReport {
                            register: B8DebugRegisterName::Rdi,
                            source:
                                B8DebugReturnToContinuationMaterializedRegisterSource::RegisterCopyFromRbx,
                            instruction_start: instruction.start().value(),
                            instruction_end: instruction.end().value(),
                            base_register: None,
                            base_value: None,
                            base_fixup_resolution: None,
                            source_register: Some(B8DebugRegisterName::Rbx),
                            source_call_return: None,
                            source_call_return_dataflow: None,
                            blocker:
                                B8DebugObjcHelperExecutionBlocker::ReturnToContinuationSavedRegisterValueMaterializationUnimplemented,
                        },
                    );
                }
                _ => {}
            }
        }

        (states, blocked)
    }
}

fn latest_materialized_register_state_before(
    states: &[B8DebugReturnToContinuationMaterializedRegisterStateReport],
    register: B8DebugRegisterName,
    source_pc: u64,
) -> Option<&B8DebugReturnToContinuationMaterializedRegisterStateReport> {
    states
        .iter()
        .rev()
        .find(|state| state.register == register && state.instruction_end <= source_pc)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationBlockedRegisterMaterializationReport {
    register: B8DebugRegisterName,
    source: B8DebugReturnToContinuationMaterializedRegisterSource,
    instruction_start: u64,
    instruction_end: u64,
    base_register: Option<B8DebugRegisterName>,
    base_value: Option<u64>,
    base_fixup_resolution: Option<B8DebugObjcArgumentFixupResolutionReport>,
    source_register: Option<B8DebugRegisterName>,
    source_call_return: Option<B8DebugReturnToContinuationCallRel32ReturnValueReport>,
    source_call_return_dataflow:
        Option<B8DebugReturnToContinuationCallRel32ReturnValueDataflowReport>,
    blocker: B8DebugObjcHelperExecutionBlocker,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationCallRel32ReturnValueReport {
    schema: &'static str,
    call_site: u64,
    return_to: u64,
    target: u64,
    return_register: B8DebugRegisterName,
    helper_boundary: B8DebugReturnToContinuationCallRel32HelperBoundaryReport,
}

impl B8DebugReturnToContinuationCallRel32ReturnValueReport {
    fn from_call_rel32(
        call_site: u64,
        return_to: u64,
        target: u64,
        class_argument: Option<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_call_rel32_return_value_v0",
            call_site,
            return_to,
            target,
            return_register: B8DebugRegisterName::Rax,
            helper_boundary:
                B8DebugReturnToContinuationCallRel32HelperBoundaryReport::from_call_rel32(
                    call_site,
                    return_to,
                    target,
                    class_argument,
                    input,
                    input_probe,
                ),
        }
    }

    fn return_value(&self) -> Option<u64> {
        self.helper_boundary.return_value()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationCallRel32HelperBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    kind: B8DebugReturnToContinuationCallRel32HelperBoundaryKind,
    source: B8DebugReturnToContinuationCallRel32HelperBoundarySource,
    call_site: u64,
    return_to: u64,
    target: u64,
    return_register: B8DebugRegisterName,
    target_resolution: B8DebugReturnToContinuationMachOStubSymbolResolutionReport,
    helper_execution_request:
        Option<B8DebugReturnToContinuationCallRel32HelperExecutionRequestReport>,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction,
}

impl B8DebugReturnToContinuationCallRel32HelperBoundaryReport {
    fn from_call_rel32(
        call_site: u64,
        return_to: u64,
        target: u64,
        class_argument: Option<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
    ) -> Self {
        let resolution =
            B8DebugReturnToContinuationMachOStubSymbolResolutionReport::from_resolution(
                resolve_mach_o_symbol_stub_for_target(
                    input,
                    input_probe,
                    MachOStubVirtualAddress::new(target),
                ),
            );
        let helper_execution_request =
            B8DebugReturnToContinuationCallRel32HelperExecutionRequestReport::from_boundary_inputs(
                &resolution,
                call_site,
                return_to,
                target,
                class_argument,
                input,
                input_probe,
            );
        let (blocker, next_action) = if let Some(request) = helper_execution_request.as_ref() {
            (request.blocker, request.next_action)
        } else if resolution.is_resolved() {
            (
                B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32HelperExecutionUnimplemented,
                B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::ImplementReturnToContinuationCallRel32HelperExecution,
            )
        } else {
            (
                B8DebugObjcHelperExecutionBlocker::ReturnToContinuationCallRel32StubSymbolResolutionUnresolved,
                B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::ResolveReturnToContinuationCallRel32StubSymbol,
            )
        };
        let status = helper_execution_request
            .as_ref()
            .map_or(B8DebugImportBoundaryStatus::Blocked, |request| {
                request.status
            });

        Self {
            schema: "b8_return_to_continuation_call_rel32_helper_boundary_v0",
            status,
            kind: B8DebugReturnToContinuationCallRel32HelperBoundaryKind::MachOSymbolStubCall,
            source:
                B8DebugReturnToContinuationCallRel32HelperBoundarySource::PublicMachOSection64DysymtabSymtab,
            call_site,
            return_to,
            target,
            return_register: B8DebugRegisterName::Rax,
            target_resolution: resolution,
            helper_execution_request,
            blocker,
            next_action,
        }
    }

    fn return_value(&self) -> Option<u64> {
        self.helper_execution_request.as_ref().and_then(
            B8DebugReturnToContinuationCallRel32HelperExecutionRequestReport::return_value,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallRel32HelperBoundaryKind {
    MachOSymbolStubCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallRel32HelperBoundarySource {
    PublicMachOSection64DysymtabSymtab,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction {
    DefineReturnToContinuationObjcAllocInitClassBridge,
    ImplementReturnToContinuationCallRel32HelperExecution,
    ImplementReturnToContinuationObjcHelperExecution,
    InspectReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionFailure,
    MaterializeReturnToContinuationObjcAllocInitClassArgument,
    ResolveReturnToContinuationObjcAllocInitClassIdentity,
    ResolveReturnToContinuationCallRel32StubSymbol,
    RunReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionOnSupportedMacosHost,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationCallRel32HelperExecutionRequestReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    kind: B8DebugReturnToContinuationCallRel32HelperExecutionRequestKind,
    source_symbol_name: String,
    call_site: u64,
    return_to: u64,
    target: u64,
    class_argument: B8DebugReturnToContinuationObjcAllocInitClassArgumentReport,
    return_writeback_boundary: B8DebugObjcHelperReturnWritebackBoundaryReport,
    return_writeback: Option<B8DebugObjcRuntimeHelperReturnWritebackReport>,
    required_capability: B8DebugReturnToContinuationCallRel32HelperExecutionCapability,
    class_bridge: B8DebugReturnToContinuationObjcAllocInitClassBridgeReport,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction,
}

impl B8DebugReturnToContinuationCallRel32HelperExecutionRequestReport {
    fn from_boundary_inputs(
        target_resolution: &B8DebugReturnToContinuationMachOStubSymbolResolutionReport,
        call_site: u64,
        return_to: u64,
        target: u64,
        class_argument: Option<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
    ) -> Option<Self> {
        if target_resolution.symbol_name.as_deref() != Some("_objc_alloc_init") {
            return None;
        }

        let class_argument =
            B8DebugReturnToContinuationObjcAllocInitClassArgumentReport::from_materialized_state(
                class_argument,
            );
        let class_bridge =
            B8DebugReturnToContinuationObjcAllocInitClassBridgeReport::from_class_argument(
                &class_argument,
                input,
                input_probe,
            );
        let blocker = class_bridge.blocker;
        let next_action = class_bridge.next_action;
        let return_writeback = class_bridge
            .fixture_delegate_bridge_contract
            .as_ref()
            .and_then(|contract| contract.host_execution.return_writeback());
        let return_writeback_boundary = return_writeback.map_or(
            B8DebugObjcHelperReturnWritebackBoundaryReport::blocked(),
            |writeback| writeback.boundary,
        );
        let status = class_bridge.status;

        Some(Self {
            schema: "b8_return_to_continuation_call_rel32_helper_execution_request_v0",
            status,
            kind: B8DebugReturnToContinuationCallRel32HelperExecutionRequestKind::ObjcAllocInit,
            source_symbol_name: "_objc_alloc_init".to_owned(),
            call_site,
            return_to,
            target,
            class_argument,
            return_writeback_boundary,
            return_writeback,
            required_capability:
                B8DebugReturnToContinuationCallRel32HelperExecutionCapability::ObjcAllocInitHelper,
            class_bridge,
            blocker,
            next_action,
        })
    }

    fn return_value(&self) -> Option<u64> {
        self.return_writeback
            .map(|writeback| writeback.written_value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallRel32HelperExecutionRequestKind {
    ObjcAllocInit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationCallRel32HelperExecutionCapability {
    ObjcAllocInitHelper,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitClassArgumentReport {
    status: B8DebugValueMaterializationStatus,
    role: B8DebugReturnToContinuationObjcAllocInitArgumentRole,
    register: B8DebugRegisterName,
    materialized_state: Option<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
    class_import: Option<MachOChainedImportIdentityReport>,
    class_rebase: Option<MachOChainedRebaseTargetIdentityReport>,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
}

impl B8DebugReturnToContinuationObjcAllocInitClassArgumentReport {
    fn from_materialized_state(
        materialized_state: Option<B8DebugReturnToContinuationMaterializedRegisterStateReport>,
    ) -> Self {
        let class_import = materialized_state
            .as_ref()
            .and_then(|state| state.fixup_resolution.as_ref())
            .and_then(|resolution| resolution.import.clone());
        let class_rebase = materialized_state
            .as_ref()
            .and_then(|state| state.fixup_resolution.as_ref())
            .and_then(|resolution| resolution.rebase);
        let is_available =
            materialized_state.is_some() && (class_import.is_some() || class_rebase.is_some());
        Self {
            status: if is_available {
                B8DebugValueMaterializationStatus::Available
            } else {
                B8DebugValueMaterializationStatus::Blocked
            },
            role: B8DebugReturnToContinuationObjcAllocInitArgumentRole::ObjcClass,
            register: B8DebugRegisterName::Rdi,
            materialized_state,
            class_import,
            class_rebase,
            blocker: if is_available {
                None
            } else {
                Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented)
            },
        }
    }

    const fn is_available(&self) -> bool {
        matches!(self.status, B8DebugValueMaterializationStatus::Available)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitArgumentRole {
    ObjcClass,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitClassBridgeReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    bridge_state: B8DebugReturnToContinuationObjcAllocInitClassBridgeState,
    class_import: Option<MachOChainedImportIdentityReport>,
    class_rebase: Option<MachOChainedRebaseTargetIdentityReport>,
    class_identity: Option<B8DebugReturnToContinuationObjcAllocInitClassIdentityReport>,
    fixture_delegate_bridge_contract:
        Option<B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeContractReport>,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction,
}

impl B8DebugReturnToContinuationObjcAllocInitClassBridgeReport {
    fn from_class_argument(
        class_argument: &B8DebugReturnToContinuationObjcAllocInitClassArgumentReport,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
    ) -> Self {
        let class_identity = class_argument.class_rebase.map(|class_rebase| {
            B8DebugReturnToContinuationObjcAllocInitClassIdentityReport::from_rebase(
                input,
                input_probe,
                class_rebase,
            )
        });
        let fixture_delegate_bridge_contract = class_identity.as_ref().and_then(|identity| {
            B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeContractReport::from_class_identity(
                identity,
            )
        });
        let (status, bridge_state, blocker, next_action) = if !class_argument.is_available() {
            (
                B8DebugImportBoundaryStatus::Blocked,
                B8DebugReturnToContinuationObjcAllocInitClassBridgeState::Blocked,
                B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassArgumentMaterializationUnimplemented,
                B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::MaterializeReturnToContinuationObjcAllocInitClassArgument,
            )
        } else if let Some(contract) = fixture_delegate_bridge_contract.as_ref() {
            (
                contract.status,
                B8DebugReturnToContinuationObjcAllocInitClassBridgeState::from_fixture_delegate_host_execution_status(
                    contract.host_execution.status,
                ),
                contract.blocker,
                contract.next_action,
            )
        } else if class_identity.is_some() {
            (
                B8DebugImportBoundaryStatus::Blocked,
                B8DebugReturnToContinuationObjcAllocInitClassBridgeState::ClassIdentityUnresolved,
                B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassIdentityUnresolved,
                B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::ResolveReturnToContinuationObjcAllocInitClassIdentity,
            )
        } else {
            (
                B8DebugImportBoundaryStatus::Blocked,
                B8DebugReturnToContinuationObjcAllocInitClassBridgeState::Unimplemented,
                B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcAllocInitClassBridgeUnimplemented,
                B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::DefineReturnToContinuationObjcAllocInitClassBridge,
            )
        };

        Self {
            schema: "b8_return_to_continuation_objc_alloc_init_class_bridge_v0",
            status,
            bridge_state,
            class_import: class_argument.class_import.clone(),
            class_rebase: class_argument.class_rebase,
            class_identity,
            fixture_delegate_bridge_contract,
            blocker,
            next_action,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitClassBridgeState {
    Blocked,
    ClassIdentityUnresolved,
    FixtureDelegateHostExecutionExecuted,
    FixtureDelegateHostExecutionFailed,
    FixtureDelegateHostExecutionSkipped,
    FixtureDelegateHostExecutionUnimplemented,
    Unimplemented,
}

impl B8DebugReturnToContinuationObjcAllocInitClassBridgeState {
    const fn from_fixture_delegate_host_execution_status(
        status: B8DebugObjcRuntimeHelperHostExecutionStatus,
    ) -> Self {
        match status {
            B8DebugObjcRuntimeHelperHostExecutionStatus::Executed => {
                Self::FixtureDelegateHostExecutionExecuted
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Failed => {
                Self::FixtureDelegateHostExecutionFailed
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped => {
                Self::FixtureDelegateHostExecutionSkipped
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Blocked => {
                Self::FixtureDelegateHostExecutionUnimplemented
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeContractReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    scope: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeScope,
    source: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeSource,
    helper_symbol_name: &'static str,
    required_capability: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeCapability,
    class_identity: B8DebugReturnToContinuationObjcAllocInitClassIdentityReport,
    input_contract: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateInputContractReport,
    output_contract: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateOutputContractReport,
    error_contract: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateErrorContractReport,
    host_execution: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionReport,
    host_execution_boundary:
        B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionBoundaryReport,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction,
}

impl B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeContractReport {
    fn from_class_identity(
        class_identity: &B8DebugReturnToContinuationObjcAllocInitClassIdentityReport,
    ) -> Option<Self> {
        if !class_identity.is_fixture_delegate() {
            return None;
        }

        let host_execution =
            B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionReport::from_class_identity(
                class_identity,
            );
        let host_execution_boundary =
            B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionBoundaryReport::from_host_execution(
                &host_execution,
            );
        Some(Self {
            schema: "b8_return_to_continuation_objc_alloc_init_fixture_delegate_bridge_contract_v0",
            status: host_execution.import_boundary_status(),
            scope:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeScope::SelfAuthoredB8GuiHelloWorldDelegateFixture,
            source:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeSource::PublicMachOSymtabNlist64AndSelfAuthoredFixture,
            helper_symbol_name: "_objc_alloc_init",
            required_capability:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeCapability::ObjcAllocInitFixtureDelegateHostSubstitute,
            class_identity: class_identity.clone(),
            input_contract: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateInputContractReport::from_class_identity(
                class_identity,
            ),
            output_contract:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateOutputContractReport::new(),
            error_contract:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateErrorContractReport::from_host_execution(
                    &host_execution,
                ),
            blocker: host_execution_boundary.blocker,
            next_action: host_execution_boundary.next_action,
            host_execution,
            host_execution_boundary,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeScope {
    SelfAuthoredB8GuiHelloWorldDelegateFixture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeSource {
    PublicMachOSymtabNlist64AndSelfAuthoredFixture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitFixtureDelegateBridgeCapability {
    ObjcAllocInitFixtureDelegateHostSubstitute,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitFixtureDelegateInputContractReport {
    class_argument_register: B8DebugRegisterName,
    class_argument_role: B8DebugReturnToContinuationObjcAllocInitArgumentRole,
    class_symbol_name: Option<String>,
    class_name: Option<String>,
    class_identity_source: B8DebugReturnToContinuationMachOSymbolAddressResolutionSource,
}

impl B8DebugReturnToContinuationObjcAllocInitFixtureDelegateInputContractReport {
    fn from_class_identity(
        class_identity: &B8DebugReturnToContinuationObjcAllocInitClassIdentityReport,
    ) -> Self {
        Self {
            class_argument_register: B8DebugRegisterName::Rdi,
            class_argument_role: B8DebugReturnToContinuationObjcAllocInitArgumentRole::ObjcClass,
            class_symbol_name: class_identity.class_symbol_name.clone(),
            class_name: class_identity.class_name.clone(),
            class_identity_source: class_identity.source,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitFixtureDelegateOutputContractReport {
    output_representation:
        B8DebugReturnToContinuationObjcAllocInitFixtureDelegateOutputRepresentation,
    return_register: B8DebugRegisterName,
    return_value_handling:
        B8DebugReturnToContinuationObjcAllocInitFixtureDelegateReturnValueHandling,
    consumer_register: B8DebugRegisterName,
    consumer_source_register: B8DebugRegisterName,
    consumer_selector_name: &'static str,
}

impl B8DebugReturnToContinuationObjcAllocInitFixtureDelegateOutputContractReport {
    const fn new() -> Self {
        Self {
            output_representation:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateOutputRepresentation::HostPointerU64,
            return_register: B8DebugRegisterName::Rax,
            return_value_handling:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateReturnValueHandling::CapturedAsX8664RaxReturnValue,
            consumer_register: B8DebugRegisterName::Rdx,
            consumer_source_register: B8DebugRegisterName::Rax,
            consumer_selector_name: B8_GUI_HELLO_WORLD_SET_DELEGATE_SELECTOR_NAME,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitFixtureDelegateOutputRepresentation {
    HostPointerU64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitFixtureDelegateReturnValueHandling {
    #[serde(rename = "captured_as_x86_64_rax_return_value")]
    CapturedAsX8664RaxReturnValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitFixtureDelegateErrorContractReport {
    blocked_error: Option<B8DebugObjcHelperExecutionBlocker>,
}

impl B8DebugReturnToContinuationObjcAllocInitFixtureDelegateErrorContractReport {
    const fn from_host_execution(
        host_execution: &B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionReport,
    ) -> Self {
        Self {
            blocked_error: host_execution.error_blocker(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionReport {
    schema: &'static str,
    status: B8DebugObjcRuntimeHelperHostExecutionStatus,
    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary,
    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope,
    effect: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostEffect,
    class_symbol_name: Option<String>,
    class_name: Option<String>,
    output: Option<B8DebugObjcRuntimeHelperOutputReport>,
    return_writeback: Option<B8DebugObjcRuntimeHelperReturnWritebackReport>,
    error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    next_blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction,
}

impl B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionReport {
    fn from_class_identity(
        class_identity: &B8DebugReturnToContinuationObjcAllocInitClassIdentityReport,
    ) -> Self {
        if !cfg!(target_os = "macos") {
            return Self::skipped(
                class_identity,
                B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHost,
            );
        }

        match run_public_objc_alloc_init_fixture_delegate_helper() {
            Ok(observation) => {
                let output = B8DebugObjcRuntimeHelperOutputReport::from_observation(observation);
                let return_writeback = B8DebugObjcRuntimeHelperReturnWritebackReport::new(
                    B8DebugObjcHelperReturnWritebackBoundaryReport::blocked().available(),
                    output.return_value,
                );
                Self {
                    schema:
                        "b8_return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_v0",
                    status: B8DebugObjcRuntimeHelperHostExecutionStatus::Executed,
                    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
                    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
                    effect:
                        B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostEffect::AllocInitFixtureDelegate,
                    class_symbol_name: class_identity.class_symbol_name.clone(),
                    class_name: class_identity.class_name.clone(),
                    output: Some(output),
                    return_writeback: Some(return_writeback),
                    error: None,
                    next_blocker:
                        B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcHelperExecutionUnimplemented,
                    next_action:
                        B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::ImplementReturnToContinuationObjcHelperExecution,
                }
            }
            Err(error) => Self::failed(class_identity, error),
        }
    }

    fn skipped(
        class_identity: &B8DebugReturnToContinuationObjcAllocInitClassIdentityReport,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
    ) -> Self {
        Self::with_error(
            class_identity,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped,
            classification,
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionUnsupported,
            B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::RunReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionOnSupportedMacosHost,
            None,
        )
    }

    fn failed(
        class_identity: &B8DebugReturnToContinuationObjcAllocInitClassIdentityReport,
        error: B8DebugObjcRuntimeHelperHostExecutionErrorReport,
    ) -> Self {
        Self::with_error(
            class_identity,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Failed,
            error.error_classification,
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionFailed,
            B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction::InspectReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionFailure,
            Some(error),
        )
    }

    fn with_error(
        class_identity: &B8DebugReturnToContinuationObjcAllocInitClassIdentityReport,
        status: B8DebugObjcRuntimeHelperHostExecutionStatus,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
        blocker: B8DebugObjcHelperExecutionBlocker,
        next_action: B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction,
        error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_v0",
            status,
            api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
            fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
            effect:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostEffect::AllocInitFixtureDelegate,
            class_symbol_name: class_identity.class_symbol_name.clone(),
            class_name: class_identity.class_name.clone(),
            output: None,
            return_writeback: None,
            error: error.or(Some(
                B8DebugObjcRuntimeHelperHostExecutionErrorReport::classification_only(
                    classification,
                ),
            )),
            next_blocker: blocker,
            next_action,
        }
    }

    const fn import_boundary_status(&self) -> B8DebugImportBoundaryStatus {
        match self.status {
            B8DebugObjcRuntimeHelperHostExecutionStatus::Executed => {
                B8DebugImportBoundaryStatus::Executed
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped => {
                B8DebugImportBoundaryStatus::Skipped
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Blocked
            | B8DebugObjcRuntimeHelperHostExecutionStatus::Failed => {
                B8DebugImportBoundaryStatus::Blocked
            }
        }
    }

    const fn error_blocker(&self) -> Option<B8DebugObjcHelperExecutionBlocker> {
        match self.status {
            B8DebugObjcRuntimeHelperHostExecutionStatus::Executed => None,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Blocked
            | B8DebugObjcRuntimeHelperHostExecutionStatus::Failed
            | B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped => Some(self.next_blocker),
        }
    }

    const fn return_writeback(&self) -> Option<B8DebugObjcRuntimeHelperReturnWritebackReport> {
        self.return_writeback
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    effect: B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostEffect,
    blocker: B8DebugObjcHelperExecutionBlocker,
    next_action: B8DebugReturnToContinuationCallRel32HelperBoundaryNextAction,
}

impl B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionBoundaryReport {
    const fn from_host_execution(
        host_execution: &B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostExecutionReport,
    ) -> Self {
        Self {
            schema:
                "b8_return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_boundary_v0",
            status: host_execution.import_boundary_status(),
            effect:
                B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostEffect::AllocInitFixtureDelegate,
            blocker: host_execution.next_blocker,
            next_action: host_execution.next_action,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcAllocInitFixtureDelegateHostEffect {
    AllocInitFixtureDelegate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcAllocInitClassIdentityReport {
    schema: &'static str,
    status: MachOSymbolAddressResolutionStatus,
    source: B8DebugReturnToContinuationMachOSymbolAddressResolutionSource,
    class_rebase: MachOChainedRebaseTargetIdentityReport,
    symbol_resolution: B8DebugReturnToContinuationMachOSymbolAddressResolutionReport,
    class_symbol_name: Option<String>,
    class_name: Option<String>,
    blocker: Option<MachOSymbolAddressResolutionBlocker>,
}

impl B8DebugReturnToContinuationObjcAllocInitClassIdentityReport {
    fn from_rebase(
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        class_rebase: MachOChainedRebaseTargetIdentityReport,
    ) -> Self {
        let symbol_resolution =
            B8DebugReturnToContinuationMachOSymbolAddressResolutionReport::from_resolution(
                resolve_mach_o_symbol_for_x86_va(
                    input,
                    input_probe,
                    class_rebase.resolved_x86_va(),
                ),
            );
        let class_symbol_name = symbol_resolution.symbol_name.clone();
        let class_name = class_symbol_name
            .as_deref()
            .and_then(|symbol_name| symbol_name.strip_prefix(OBJC_CLASS_SYMBOL_PREFIX))
            .map(ToOwned::to_owned);
        Self {
            schema: "b8_return_to_continuation_objc_alloc_init_class_identity_v0",
            status: symbol_resolution.status,
            source: B8DebugReturnToContinuationMachOSymbolAddressResolutionSource::PublicMachOSymtabNlist64,
            class_rebase,
            blocker: symbol_resolution.blocker,
            symbol_resolution,
            class_symbol_name,
            class_name,
        }
    }

    fn is_fixture_delegate(&self) -> bool {
        self.class_symbol_name.as_deref() == Some(B8_GUI_HELLO_WORLD_DELEGATE_CLASS_SYMBOL_NAME)
            && self.class_name.as_deref() == Some(B8_GUI_HELLO_WORLD_DELEGATE_CLASS_NAME)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationMachOSymbolAddressResolutionReport {
    schema: &'static str,
    status: MachOSymbolAddressResolutionStatus,
    source: B8DebugReturnToContinuationMachOSymbolAddressResolutionSource,
    symbol_vm_address: Option<u64>,
    symbol_table_index: Option<u32>,
    symbol_name: Option<String>,
    blocker: Option<MachOSymbolAddressResolutionBlocker>,
}

impl B8DebugReturnToContinuationMachOSymbolAddressResolutionReport {
    fn from_resolution(resolution: MachOSymbolAddressResolution) -> Self {
        let resolved = resolution.resolved_symbol();
        Self {
            schema: "b8_return_to_continuation_mach_o_symbol_address_resolution_v0",
            status: resolution.status(),
            source: B8DebugReturnToContinuationMachOSymbolAddressResolutionSource::PublicMachOSymtabNlist64,
            symbol_vm_address: resolved.map(|symbol| symbol.symbol_vm_address().value()),
            symbol_table_index: resolved.map(|symbol| symbol.symbol_table_index().as_u32()),
            symbol_name: resolved.map(|symbol| symbol.symbol_name().as_str().to_owned()),
            blocker: resolution.blocker(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationMachOSymbolAddressResolutionSource {
    PublicMachOSymtabNlist64,
}

const OBJC_CLASS_SYMBOL_PREFIX: &str = "_OBJC_CLASS_$_";
const B8_GUI_HELLO_WORLD_DELEGATE_CLASS_SYMBOL_NAME: &str =
    "_OBJC_CLASS_$_BaraGuiHelloWorldDelegate";
const B8_GUI_HELLO_WORLD_DELEGATE_CLASS_NAME: &str = "BaraGuiHelloWorldDelegate";
const B8_GUI_HELLO_WORLD_TITLE: &str = "Bara GUI Hello World";
const B8_GUI_HELLO_WORLD_TEXT: &str = "hello world";
const B8_GUI_HELLO_WORLD_SET_DELEGATE_SELECTOR_NAME: &str = "setDelegate:";
const B8_GUI_HELLO_WORLD_RUN_SELECTOR_NAME: &str = "run";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationMachOStubSymbolResolutionReport {
    schema: &'static str,
    status: MachOStubSymbolResolutionStatus,
    source: B8DebugReturnToContinuationCallRel32HelperBoundarySource,
    section_segment_name: Option<String>,
    section_name: Option<String>,
    stub_address: Option<u64>,
    stub_byte_size: Option<u32>,
    stub_index: Option<u32>,
    indirect_symbol_table_slot: Option<u32>,
    indirect_symbol_table_file_offset: Option<u32>,
    symbol_table_index: Option<u32>,
    symbol_name: Option<String>,
    blocker: Option<MachOStubSymbolResolutionBlocker>,
}

impl B8DebugReturnToContinuationMachOStubSymbolResolutionReport {
    fn from_resolution(resolution: MachOStubSymbolResolution) -> Self {
        let resolved = resolution.resolved_symbol();
        Self {
            schema: "b8_return_to_continuation_mach_o_stub_symbol_resolution_v0",
            status: resolution.status(),
            source:
                B8DebugReturnToContinuationCallRel32HelperBoundarySource::PublicMachOSection64DysymtabSymtab,
            section_segment_name: resolved.map(|symbol| symbol.section_segment_name().to_owned()),
            section_name: resolved.map(|symbol| symbol.section_name().to_owned()),
            stub_address: resolved.map(|symbol| symbol.stub_address().as_u64()),
            stub_byte_size: resolved.map(|symbol| symbol.stub_byte_size().as_u32()),
            stub_index: resolved.map(|symbol| symbol.stub_index().as_u32()),
            indirect_symbol_table_slot: resolved
                .map(|symbol| symbol.indirect_symbol_table_slot().as_u32()),
            indirect_symbol_table_file_offset: resolved
                .map(|symbol| symbol.indirect_symbol_table_file_offset().as_u32()),
            symbol_table_index: resolved.map(|symbol| symbol.symbol_table_index().as_u32()),
            symbol_name: resolved.map(|symbol| symbol.symbol_name().as_str().to_owned()),
            blocker: resolution.blocker(),
        }
    }

    const fn is_resolved(&self) -> bool {
        matches!(self.status, MachOStubSymbolResolutionStatus::Resolved)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationCallRel32ReturnValueDataflowReport {
    schema: &'static str,
    producer_call_site: u64,
    producer_return_to: u64,
    producer_target: u64,
    producer_symbol_name: Option<String>,
    return_register: B8DebugRegisterName,
    consumer_instruction_start: u64,
    consumer_instruction_end: u64,
    consumer_register: B8DebugRegisterName,
    consumer_source_register: B8DebugRegisterName,
}

impl B8DebugReturnToContinuationCallRel32ReturnValueDataflowReport {
    fn from_consumer(
        call_return: &B8DebugReturnToContinuationCallRel32ReturnValueReport,
        consumer_instruction_start: u64,
        consumer_instruction_end: u64,
        consumer_register: B8DebugRegisterName,
        consumer_source_register: B8DebugRegisterName,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_call_rel32_return_value_dataflow_v0",
            producer_call_site: call_return.call_site,
            producer_return_to: call_return.return_to,
            producer_target: call_return.target,
            producer_symbol_name: call_return
                .helper_boundary
                .target_resolution
                .symbol_name
                .clone(),
            return_register: call_return.return_register,
            consumer_instruction_start,
            consumer_instruction_end,
            consumer_register,
            consumer_source_register,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationSavedRegisterValueReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    register: B8DebugRegisterName,
    source: B8DebugReturnToContinuationSavedRegisterValueSource,
    preservation_model: B8DebugReturnToContinuationCallTargetPreservationModel,
    producer_call_site: u64,
    producer_return_to: u64,
    producer_target: u64,
    producer_symbol_name: Option<String>,
    producer_target_resolution: B8DebugReturnToContinuationMachOStubSymbolResolutionReport,
    return_register: B8DebugRegisterName,
    consumer_instruction_start: u64,
    consumer_instruction_end: u64,
    consumer_register: B8DebugRegisterName,
    consumer_source_register: B8DebugRegisterName,
    host_observation: B8DebugObjcRuntimeHelperOutputReport,
    raw_pointer_reuse: B8DebugReturnToContinuationSavedRegisterRawPointerReuse,
    value: u64,
    value_source: B8DebugReturnToContinuationMaterializedRegisterValueSource,
    width: B8DebugMemoryReadWidthReport,
}

impl B8DebugReturnToContinuationSavedRegisterValueReport {
    fn from_decode_report(
        decode_report: &B8DebugDecodeReport,
        boundary_call_site: u64,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
    ) -> Vec<Self> {
        Self::autorelease_pool_push_rbx_value_before(
            decode_report,
            boundary_call_site,
            input,
            input_probe,
        )
        .into_iter()
        .collect()
    }

    fn autorelease_pool_push_rbx_value_before(
        decode_report: &B8DebugDecodeReport,
        boundary_call_site: u64,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
    ) -> Option<Self> {
        let (producer, producer_target, producer_return_to, consumer) = decode_report
            .instructions
            .windows(2)
            .rev()
            .find_map(|pair| {
                let producer = &pair[0];
                let consumer = &pair[1];
                let B8DebugDecodedInstructionKindReport::CallRel32 { target, return_to } =
                    &producer.kind
                else {
                    return None;
                };
                let target = *target;
                let return_to = *return_to;
                if producer.start >= boundary_call_site
                    || consumer.end > boundary_call_site
                    || consumer.start != return_to
                    || !matches!(
                        &consumer.kind,
                        B8DebugDecodedInstructionKindReport::MovRbxRax
                    )
                {
                    return None;
                }

                Some((producer, target, return_to, consumer))
            })?;

        let producer_target_resolution =
            B8DebugReturnToContinuationMachOStubSymbolResolutionReport::from_resolution(
                resolve_mach_o_symbol_stub_for_target(
                    input,
                    input_probe,
                    MachOStubVirtualAddress::new(producer_target),
                ),
            );
        if producer_target_resolution.symbol_name.as_deref() != Some("_objc_autoreleasePoolPush") {
            return None;
        }
        if !cfg!(target_os = "macos") {
            return None;
        }

        let host_observation = B8DebugObjcRuntimeHelperOutputReport::from_observation(
            run_public_objc_autorelease_pool_push_helper().ok()?,
        );

        Some(Self {
            schema: "b8_return_to_continuation_saved_register_value_v0",
            status: B8DebugImportBoundaryStatus::Executed,
            register: B8DebugRegisterName::Rbx,
            source:
                B8DebugReturnToContinuationSavedRegisterValueSource::CallRel32ReturnCopyToSavedRegister,
            preservation_model:
                B8DebugReturnToContinuationCallTargetPreservationModel::X8664MacosSystemVCalleeSavedRegister,
            producer_call_site: producer.start,
            producer_return_to,
            producer_target,
            producer_symbol_name: producer_target_resolution.symbol_name.clone(),
            producer_target_resolution,
            return_register: B8DebugRegisterName::Rax,
            consumer_instruction_start: consumer.start,
            consumer_instruction_end: consumer.end,
            consumer_register: B8DebugRegisterName::Rbx,
            consumer_source_register: B8DebugRegisterName::Rax,
            host_observation,
            raw_pointer_reuse:
                B8DebugReturnToContinuationSavedRegisterRawPointerReuse::NotReusedAcrossHelperProcesses,
            value: host_observation.return_value,
            value_source:
                B8DebugReturnToContinuationMaterializedRegisterValueSource::ObjcAutoreleasePoolPushHelperReturnValue,
            width: B8DebugMemoryReadWidthReport::Bits64,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationSavedRegisterValueSource {
    CallRel32ReturnCopyToSavedRegister,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationSavedRegisterRawPointerReuse {
    NotReusedAcrossHelperProcesses,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationAutoreleasePoolPopBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    kind: B8DebugReturnToContinuationAutoreleasePoolBoundaryKind,
    source: B8DebugReturnToContinuationCallRel32HelperBoundarySource,
    call_site: u64,
    return_to: u64,
    target: u64,
    target_resolution: B8DebugReturnToContinuationMachOStubSymbolResolutionReport,
    token_argument: B8DebugReturnToContinuationAutoreleasePoolTokenArgumentReport,
    host_execution: B8DebugReturnToContinuationAutoreleasePoolPopHostExecutionReport,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationAutoreleasePoolPopBoundaryReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        materialized_register_states: &[B8DebugReturnToContinuationMaterializedRegisterStateReport],
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
    ) -> Option<Self> {
        let (call_site, return_to, target) =
            decoded
                .instructions()
                .iter()
                .find_map(|instruction| match instruction.kind() {
                    DecodedInstructionKind::CallRel32 { target, return_to } => Some((
                        instruction.start().value(),
                        return_to.value(),
                        target.value(),
                    )),
                    _ => None,
                })?;
        let target_resolution =
            B8DebugReturnToContinuationMachOStubSymbolResolutionReport::from_resolution(
                resolve_mach_o_symbol_stub_for_target(
                    input,
                    input_probe,
                    MachOStubVirtualAddress::new(target),
                ),
            );
        if target_resolution.symbol_name.as_deref() != Some("_objc_autoreleasePoolPop") {
            return None;
        }

        let token_argument = B8DebugReturnToContinuationAutoreleasePoolTokenArgumentReport::from_materialized_register(
            materialized_register_states,
            call_site,
        )?;
        let host_execution =
            B8DebugReturnToContinuationAutoreleasePoolPopHostExecutionReport::from_token_argument(
                &token_argument,
            );

        Some(Self {
            schema: "b8_return_to_continuation_autorelease_pool_pop_boundary_v0",
            status: host_execution.import_boundary_status(),
            kind: B8DebugReturnToContinuationAutoreleasePoolBoundaryKind::AutoreleasePoolPop,
            source:
                B8DebugReturnToContinuationCallRel32HelperBoundarySource::PublicMachOSection64DysymtabSymtab,
            call_site,
            return_to,
            target,
            target_resolution,
            token_argument,
            blocker: host_execution.blocker(),
            next_action: host_execution.next_action,
            host_execution,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationAutoreleasePoolBoundaryKind {
    AutoreleasePoolPop,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationAutoreleasePoolTokenArgumentReport {
    role: B8DebugReturnToContinuationAutoreleasePoolArgumentRole,
    register: B8DebugRegisterName,
    state: B8DebugValueMaterializationStatus,
    source: B8DebugReturnToContinuationAutoreleasePoolTokenSource,
    materialized_state: B8DebugReturnToContinuationMaterializedRegisterStateReport,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
}

impl B8DebugReturnToContinuationAutoreleasePoolTokenArgumentReport {
    fn from_materialized_register(
        materialized_register_states: &[B8DebugReturnToContinuationMaterializedRegisterStateReport],
        call_site: u64,
    ) -> Option<Self> {
        let materialized_state = latest_materialized_register_state_before(
            materialized_register_states,
            B8DebugRegisterName::Rdi,
            call_site,
        )?
        .clone();
        if !materialized_state
            .source_saved_register_value
            .as_ref()
            .is_some_and(|saved| {
                saved.producer_symbol_name.as_deref() == Some("_objc_autoreleasePoolPush")
                    && saved.register == B8DebugRegisterName::Rbx
            })
        {
            return None;
        }

        Some(Self {
            role: B8DebugReturnToContinuationAutoreleasePoolArgumentRole::AutoreleasePoolToken,
            register: B8DebugRegisterName::Rdi,
            state: B8DebugValueMaterializationStatus::Available,
            source:
                B8DebugReturnToContinuationAutoreleasePoolTokenSource::SavedRbxFromAutoreleasePoolPush,
            materialized_state,
            blocker: None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationAutoreleasePoolArgumentRole {
    AutoreleasePoolToken,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationAutoreleasePoolTokenSource {
    SavedRbxFromAutoreleasePoolPush,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationAutoreleasePoolPopHostExecutionReport {
    schema: &'static str,
    status: B8DebugObjcRuntimeHelperHostExecutionStatus,
    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary,
    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope,
    effect: B8DebugReturnToContinuationAutoreleasePoolHostEffect,
    input_token_model: B8DebugReturnToContinuationAutoreleasePoolPopInputTokenModel,
    fixture_token_value: u64,
    helper_token_observation: Option<B8DebugObjcRuntimeHelperOutputReport>,
    raw_pointer_reuse: B8DebugReturnToContinuationSavedRegisterRawPointerReuse,
    output: Option<B8DebugReturnToContinuationAutoreleasePoolPopHostOutputReport>,
    error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationAutoreleasePoolPopHostExecutionReport {
    fn from_token_argument(
        token_argument: &B8DebugReturnToContinuationAutoreleasePoolTokenArgumentReport,
    ) -> Self {
        if !cfg!(target_os = "macos") {
            return Self::with_error(
                token_argument,
                B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped,
                B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHost,
                B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionUnsupported,
                B8DebugReturnToContinuationDecodeNextAction::RunReturnToContinuationObjcHelperOnSupportedMacosHost,
                None,
            );
        }

        match run_public_objc_autorelease_pool_push_helper() {
            Ok(observation) => {
                let helper_token_observation =
                    B8DebugObjcRuntimeHelperOutputReport::from_observation(observation);
                Self {
                    schema: "b8_return_to_continuation_autorelease_pool_pop_host_execution_v0",
                    status: B8DebugObjcRuntimeHelperHostExecutionStatus::Executed,
                    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
                    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
                    effect:
                        B8DebugReturnToContinuationAutoreleasePoolHostEffect::AutoreleasePoolPushPop,
                    input_token_model:
                        B8DebugReturnToContinuationAutoreleasePoolPopInputTokenModel::FreshHelperProcessPushPopToken,
                    fixture_token_value: token_argument.materialized_state.value,
                    helper_token_observation: Some(helper_token_observation),
                    raw_pointer_reuse:
                        B8DebugReturnToContinuationSavedRegisterRawPointerReuse::NotReusedAcrossHelperProcesses,
                    output: Some(
                        B8DebugReturnToContinuationAutoreleasePoolPopHostOutputReport::void_return(),
                    ),
                    error: None,
                    blocker: None,
                    next_action:
                        B8DebugReturnToContinuationDecodeNextAction::AddReturnToContinuationInstructionSupport,
                }
            }
            Err(error) => Self::with_error(
                token_argument,
                B8DebugObjcRuntimeHelperHostExecutionStatus::Failed,
                error.error_classification,
                B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionFailed,
                B8DebugReturnToContinuationDecodeNextAction::InspectReturnToContinuationObjcHelperExecutionFailure,
                Some(error),
            ),
        }
    }

    fn with_error(
        token_argument: &B8DebugReturnToContinuationAutoreleasePoolTokenArgumentReport,
        status: B8DebugObjcRuntimeHelperHostExecutionStatus,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
        blocker: B8DebugObjcHelperExecutionBlocker,
        next_action: B8DebugReturnToContinuationDecodeNextAction,
        error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_autorelease_pool_pop_host_execution_v0",
            status,
            api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
            fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
            effect: B8DebugReturnToContinuationAutoreleasePoolHostEffect::AutoreleasePoolPushPop,
            input_token_model:
                B8DebugReturnToContinuationAutoreleasePoolPopInputTokenModel::FreshHelperProcessPushPopToken,
            fixture_token_value: token_argument.materialized_state.value,
            helper_token_observation: None,
            raw_pointer_reuse:
                B8DebugReturnToContinuationSavedRegisterRawPointerReuse::NotReusedAcrossHelperProcesses,
            output: None,
            error: if let Some(error) = error {
                Some(error)
            } else {
                Some(B8DebugObjcRuntimeHelperHostExecutionErrorReport::classification_only(
                    classification,
                ))
            },
            blocker: Some(blocker),
            next_action,
        }
    }

    const fn import_boundary_status(&self) -> B8DebugImportBoundaryStatus {
        match self.status {
            B8DebugObjcRuntimeHelperHostExecutionStatus::Executed => {
                B8DebugImportBoundaryStatus::Executed
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped => {
                B8DebugImportBoundaryStatus::Skipped
            }
            B8DebugObjcRuntimeHelperHostExecutionStatus::Blocked
            | B8DebugObjcRuntimeHelperHostExecutionStatus::Failed => {
                B8DebugImportBoundaryStatus::Blocked
            }
        }
    }

    const fn blocker(&self) -> Option<B8DebugObjcHelperExecutionBlocker> {
        self.blocker
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationAutoreleasePoolHostEffect {
    AutoreleasePoolPushPop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationAutoreleasePoolPopInputTokenModel {
    FreshHelperProcessPushPopToken,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationAutoreleasePoolPopHostOutputReport {
    helper_output: B8DebugObjcRuntimeHelperOutput,
    representation: B8DebugReturnToContinuationObjcHelperOutputRepresentation,
    return_value: Option<u64>,
}

impl B8DebugReturnToContinuationAutoreleasePoolPopHostOutputReport {
    const fn void_return() -> Self {
        Self {
            helper_output: B8DebugObjcRuntimeHelperOutput::ObjcHelperVoidReturn,
            representation:
                B8DebugReturnToContinuationObjcHelperOutputRepresentation::VoidNoReturnValue,
            return_value: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationMaterializedRegisterSource {
    #[serde(rename = "imported_global_pointee_load")]
    ImportedGlobalPointee,
    #[serde(rename = "register_indirect_qword_load")]
    RegisterIndirectQword,
    #[serde(rename = "register_copy_from_rax")]
    RegisterCopyFromRax,
    #[serde(rename = "register_copy_from_rbx")]
    RegisterCopyFromRbx,
    #[serde(rename = "rip_relative_qword_load")]
    RipRelativeQword,
    #[serde(rename = "xor_edx_edx_zero")]
    XorEdxEdxZero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationMaterializedRegisterValueSource {
    ObjcAutoreleasePoolPushHelperReturnValue,
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

    fn nsapp_from_set_activation_policy_request(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
    ) -> Option<Self> {
        if !request.is_supported_b8_set_activation_policy_message() {
            return None;
        }

        Self::nsapp_from_objc_helper_request(request)
    }

    fn nsapp_from_set_delegate_request(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
    ) -> Option<Self> {
        if !request.is_supported_b8_set_delegate_message() {
            return None;
        }

        Self::nsapp_from_objc_helper_request(request)
    }

    fn nsapp_from_run_loop_request(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
    ) -> Option<Self> {
        if !request.is_supported_b8_run_message() {
            return None;
        }

        Self::nsapp_from_objc_helper_request(request)
    }

    fn nsapp_from_objc_helper_request(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
    ) -> Option<Self> {
        if B8DebugReturnToContinuationObjcHelperReceiver::from_argument(&request.receiver)
            != B8DebugReturnToContinuationObjcHelperReceiver::NsAppSharedApplicationValue
        {
            return None;
        }

        Some(Self {
            symbol: B8DebugReturnToContinuationImportedGlobalSymbol::NsApp,
            value: request.receiver.materialized_state.as_ref()?.value,
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
    objc_helper_boundary: Option<B8DebugReturnToContinuationObjcHelperBoundaryReport>,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationCallBoundaryReport {
    fn from_decoded(
        decoded: &DecodedFunction,
        materialized_register_states: &[B8DebugReturnToContinuationMaterializedRegisterStateReport],
        preserved_call_target_import: Option<MachOChainedImportIdentityReport>,
        host_execution_context: B8DebugReturnToContinuationHostExecutionContext<'_>,
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

        let target = B8DebugReturnToContinuationCallTargetReport::preserved_r14(
            preserved_call_target_import,
        );
        let receiver_argument =
            B8DebugReturnToContinuationCallArgumentReport::from_materialized_register(
                0,
                B8DebugReturnToContinuationCallArgumentRole::Receiver,
                B8DebugRegisterName::Rdi,
                materialized_register_states,
                host_execution_context.image_metadata,
            );
        let selector_argument =
            B8DebugReturnToContinuationCallArgumentReport::from_materialized_register(
                1,
                B8DebugReturnToContinuationCallArgumentRole::Selector,
                B8DebugRegisterName::Rsi,
                materialized_register_states,
                host_execution_context.image_metadata,
            );
        let is_run_selector = selector_argument
            .selector_identity
            .as_ref()
            .and_then(B8DebugObjcSelectorIdentityReport::selector_name)
            == Some(B8_GUI_HELLO_WORLD_RUN_SELECTOR_NAME);
        let mut arguments = vec![receiver_argument, selector_argument];
        if !is_run_selector {
            arguments.push(
                B8DebugReturnToContinuationCallArgumentReport::from_materialized_register(
                    2,
                    B8DebugReturnToContinuationCallArgumentRole::Argument,
                    B8DebugRegisterName::Rdx,
                    materialized_register_states,
                    host_execution_context.image_metadata,
                ),
            );
        }
        let objc_helper_boundary =
            B8DebugReturnToContinuationObjcHelperBoundaryReport::from_call_boundary(
                call_site,
                return_to,
                &target,
                &arguments,
                host_execution_context,
            );
        let blocker = objc_helper_boundary.as_ref().map_or(
            Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented),
            |boundary| boundary.blocker,
        );
        let next_action = objc_helper_boundary.as_ref().map_or(
            B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution,
            |boundary| boundary.next_action,
        );

        Some(Self {
            schema: "b8_return_to_continuation_call_boundary_v0",
            status: if blocker.is_none() {
                B8DebugImportBoundaryStatus::Executed
            } else {
                B8DebugImportBoundaryStatus::Blocked
            },
            call_site,
            return_to,
            target_register: B8DebugRegisterName::R14,
            target,
            arguments,
            objc_helper_boundary,
            blocker,
            next_action,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    helper_request: B8DebugReturnToContinuationObjcHelperRequestReport,
    bridge_contract: B8DebugReturnToContinuationObjcHelperBridgeContractReport,
    available_or_blocked_state: B8DebugReturnToContinuationObjcHelperStateReport,
    host_execution: B8DebugReturnToContinuationObjcHelperHostExecutionReport,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationObjcHelperBoundaryReport {
    fn from_call_boundary(
        call_site: u64,
        return_to: u64,
        target: &B8DebugReturnToContinuationCallTargetReport,
        arguments: &[B8DebugReturnToContinuationCallArgumentReport],
        host_execution_context: B8DebugReturnToContinuationHostExecutionContext<'_>,
    ) -> Option<Self> {
        let source_import = target.import.as_ref()?;
        if !is_objc_msg_send_import(source_import) {
            return None;
        }

        let receiver = find_continuation_call_argument(
            arguments,
            B8DebugReturnToContinuationCallArgumentRole::Receiver,
        )?;
        let selector = find_continuation_call_argument(
            arguments,
            B8DebugReturnToContinuationCallArgumentRole::Selector,
        )?;
        let selector_name = selector
            .selector_identity
            .as_ref()
            .and_then(B8DebugObjcSelectorIdentityReport::selector_name);
        if selector_name != Some("setActivationPolicy:")
            && selector_name != Some(B8_GUI_HELLO_WORLD_SET_DELEGATE_SELECTOR_NAME)
            && selector_name != Some(B8_GUI_HELLO_WORLD_RUN_SELECTOR_NAME)
        {
            return None;
        }
        let argument = if selector_name == Some(B8_GUI_HELLO_WORLD_RUN_SELECTOR_NAME) {
            None
        } else {
            Some(find_continuation_call_argument(
                arguments,
                B8DebugReturnToContinuationCallArgumentRole::Argument,
            )?)
        };

        let helper_request = B8DebugReturnToContinuationObjcHelperRequestReport::new(
            call_site,
            return_to,
            source_import,
            receiver,
            selector,
            argument,
            host_execution_context
                .continuation_inputs
                .preserved_register_values
                .clone(),
        );
        let host_execution = B8DebugReturnToContinuationObjcHelperHostExecutionReport::from_request(
            &helper_request,
            host_execution_context.code_bytes,
            host_execution_context.input,
            host_execution_context.input_probe,
            host_execution_context.image_metadata,
        );
        let available_or_blocked_state =
            B8DebugReturnToContinuationObjcHelperStateReport::from_request_and_host_execution(
                &helper_request,
                &host_execution,
            );
        let bridge_contract =
            B8DebugReturnToContinuationObjcHelperBridgeContractReport::from_host_execution(
                &helper_request,
                available_or_blocked_state,
                &host_execution,
            );
        let blocker = host_execution.next_blocker;
        let next_action = host_execution.next_action;

        Some(Self {
            schema: "b8_return_to_continuation_objc_helper_boundary_v0",
            status: if blocker.is_none() {
                B8DebugImportBoundaryStatus::Executed
            } else {
                B8DebugImportBoundaryStatus::Blocked
            },
            helper_request,
            bridge_contract,
            available_or_blocked_state,
            host_execution,
            blocker,
            next_action,
        })
    }
}

fn is_objc_msg_send_import(import: &MachOChainedImportIdentityReport) -> bool {
    import.symbol_name() == "_objc_msgSend"
        && import
            .dylib_path()
            .is_some_and(|path| path == "/usr/lib/libobjc.A.dylib")
}

fn find_continuation_call_argument(
    arguments: &[B8DebugReturnToContinuationCallArgumentReport],
    role: B8DebugReturnToContinuationCallArgumentRole,
) -> Option<&B8DebugReturnToContinuationCallArgumentReport> {
    arguments.iter().find(|argument| argument.role == role)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperRequestReport {
    schema: &'static str,
    kind: B8DebugObjcHelperExecutionRequestKind,
    source_isa: B8DebugSourceIsa,
    call_site: u64,
    return_to: u64,
    source_import: MachOChainedImportIdentityReport,
    receiver: B8DebugReturnToContinuationCallArgumentReport,
    selector: B8DebugReturnToContinuationCallArgumentReport,
    argument: Option<B8DebugReturnToContinuationCallArgumentReport>,
    required_capability: B8DebugObjcHelperExecutionCapability,
    preserved_register_values: Vec<B8DebugReturnToContinuationSavedRegisterValueReport>,
}

impl B8DebugReturnToContinuationObjcHelperRequestReport {
    fn new(
        call_site: u64,
        return_to: u64,
        source_import: &MachOChainedImportIdentityReport,
        receiver: &B8DebugReturnToContinuationCallArgumentReport,
        selector: &B8DebugReturnToContinuationCallArgumentReport,
        argument: Option<&B8DebugReturnToContinuationCallArgumentReport>,
        preserved_register_values: Vec<B8DebugReturnToContinuationSavedRegisterValueReport>,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_objc_helper_request_v0",
            kind: B8DebugObjcHelperExecutionRequestKind::ObjcMsgSend,
            source_isa: B8DebugSourceIsa::X8664,
            call_site,
            return_to,
            source_import: source_import.clone(),
            receiver: receiver.clone(),
            selector: selector.clone(),
            argument: argument.cloned(),
            required_capability: B8DebugObjcHelperExecutionCapability::ObjcRuntimeMessageSendHelper,
            preserved_register_values,
        }
    }

    fn selector_name(&self) -> Option<&str> {
        self.selector
            .selector_identity
            .as_ref()
            .and_then(B8DebugObjcSelectorIdentityReport::selector_name)
    }

    fn argument_value(&self) -> Option<u64> {
        self.argument
            .as_ref()
            .and_then(|argument| argument.materialized_state.as_ref())
            .map(|state| state.value)
    }

    fn argument_register(&self) -> Option<B8DebugRegisterName> {
        self.argument.as_ref().map(|argument| argument.register)
    }

    fn argument_model(&self) -> B8DebugReturnToContinuationObjcHelperArgumentModel {
        if self.argument.is_some() {
            B8DebugReturnToContinuationObjcHelperArgumentModel::X8664RegisterArgument
        } else {
            B8DebugReturnToContinuationObjcHelperArgumentModel::NoArguments
        }
    }

    fn argument_state(&self) -> B8DebugValueMaterializationStatus {
        self.argument
            .as_ref()
            .map_or(B8DebugValueMaterializationStatus::NotRequired, |argument| {
                argument.state
            })
    }

    fn has_no_arguments(&self) -> bool {
        self.argument.is_none()
    }

    fn is_supported_b8_set_activation_policy_message(&self) -> bool {
        is_objc_msg_send_import(&self.source_import)
            && B8DebugReturnToContinuationObjcHelperReceiver::from_argument(&self.receiver)
                == B8DebugReturnToContinuationObjcHelperReceiver::NsAppSharedApplicationValue
            && self.selector_name() == Some("setActivationPolicy:")
            && self.argument_value() == Some(0)
            && self.required_capability
                == B8DebugObjcHelperExecutionCapability::ObjcRuntimeMessageSendHelper
    }

    fn is_supported_b8_set_delegate_message(&self) -> bool {
        is_objc_msg_send_import(&self.source_import)
            && B8DebugReturnToContinuationObjcHelperReceiver::from_argument(&self.receiver)
                == B8DebugReturnToContinuationObjcHelperReceiver::NsAppSharedApplicationValue
            && self.selector_name() == Some(B8_GUI_HELLO_WORLD_SET_DELEGATE_SELECTOR_NAME)
            && self.argument_is_fixture_delegate_host_substitute()
            && self.required_capability
                == B8DebugObjcHelperExecutionCapability::ObjcRuntimeMessageSendHelper
    }

    fn is_supported_b8_run_message(&self) -> bool {
        is_objc_msg_send_import(&self.source_import)
            && B8DebugReturnToContinuationObjcHelperReceiver::from_argument(&self.receiver)
                == B8DebugReturnToContinuationObjcHelperReceiver::NsAppSharedApplicationValue
            && self.selector_name() == Some(B8_GUI_HELLO_WORLD_RUN_SELECTOR_NAME)
            && self.has_no_arguments()
            && self.required_capability
                == B8DebugObjcHelperExecutionCapability::ObjcRuntimeMessageSendHelper
    }

    fn argument_is_fixture_delegate_host_substitute(&self) -> bool {
        self.argument
            .as_ref()
            .and_then(|argument| argument.materialized_state.as_ref())
            .and_then(|state| state.source_call_return.as_ref())
            .and_then(|call_return| {
                call_return
                    .helper_boundary
                    .helper_execution_request
                    .as_ref()
            })
            .is_some_and(|request| {
                request.kind
                    == B8DebugReturnToContinuationCallRel32HelperExecutionRequestKind::ObjcAllocInit
                    && request
                        .class_bridge
                        .fixture_delegate_bridge_contract
                        .is_some()
                    && request.return_writeback.is_some()
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperArgumentModel {
    NoArguments,
    #[serde(rename = "x86_64_register_argument")]
    X8664RegisterArgument,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperHostExecutionReport {
    schema: &'static str,
    status: B8DebugObjcRuntimeHelperHostExecutionStatus,
    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary,
    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope,
    effect: B8DebugReturnToContinuationObjcHelperEffect,
    selector_name: Option<String>,
    argument_value: Option<u64>,
    host_object_boundary:
        Option<B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectBoundaryReport>,
    appkit_run_loop_boundary:
        Option<B8DebugReturnToContinuationObjcHelperAppKitRunLoopBoundaryReport>,
    output: Option<B8DebugReturnToContinuationObjcHelperHostOutputReport>,
    next_source_pc: u64,
    next_continuation: Option<Box<B8DebugReturnToContinuationDecodeBoundaryReport>>,
    error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    next_blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationObjcHelperHostExecutionReport {
    fn from_request(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        code_bytes: &X86Bytes,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        if request.is_supported_b8_set_activation_policy_message() {
            return Self::execute_set_activation_policy(
                request,
                code_bytes,
                input,
                input_probe,
                image_metadata,
            );
        }
        if request.is_supported_b8_set_delegate_message() {
            return Self::execute_set_delegate(
                request,
                code_bytes,
                input,
                input_probe,
                image_metadata,
            );
        }
        if request.is_supported_b8_run_message() {
            return Self::execute_appkit_run_loop(
                request,
                code_bytes,
                input,
                input_probe,
                image_metadata,
            );
        }

        Self::blocked(
            request,
            B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHelperContract,
        )
    }

    fn execute_set_activation_policy(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        code_bytes: &X86Bytes,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        if !cfg!(target_os = "macos") {
            return Self::skipped(
                request,
                B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHost,
            );
        }

        match run_public_objc_msg_send_set_activation_policy_helper() {
            Ok(observation) => match B8DebugReturnToContinuationObjcHelperHostOutputReport::from_set_activation_policy_observation(
                observation,
            ) {
                Ok(output) => {
                let register_state = B8DebugObjcHelperReturnContinuationRegisterStateReport {
                    register: B8DebugRegisterName::Rax,
                    source:
                        B8DebugObjcHelperReturnContinuationRegisterSource::ObjcHelperReturnValue,
                    value: output.return_value.unwrap_or(0),
                    width: B8DebugMemoryReadWidthReport::Bits64,
                };
                let preserved_r15_state =
                    request.receiver.materialized_state.as_ref().filter(|state| {
                        state.base_register == Some(B8DebugRegisterName::R15)
                    });
                let continuation_inputs = B8DebugReturnToContinuationDecodeInputs {
                    imported_global_value:
                        B8DebugReturnToContinuationImportedGlobalValue::nsapp_from_set_activation_policy_request(
                            request,
                        ),
                    preserved_call_target_import: Some(request.source_import.clone()),
                    preserved_r15_value: preserved_r15_state.and_then(|state| state.base_value),
                    preserved_r15_fixup_resolution: preserved_r15_state
                        .and_then(|state| state.base_fixup_resolution.clone()),
                    preserved_register_values: request.preserved_register_values.clone(),
                };
                let next_continuation =
                    B8DebugReturnToContinuationDecodeBoundaryReport::from_code_bytes(
                        request.return_to,
                        Some(register_state),
                        continuation_inputs,
                        code_bytes,
                        input,
                        input_probe,
                        image_metadata,
                    );
                let next_blocker = next_continuation.as_ref().map_or(
                    Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented),
                    |continuation| continuation.blocker(),
                );
                let next_action = next_continuation.as_ref().map_or(
                    B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution,
                    |continuation| continuation.next_action,
                );

                Self {
                    schema: "b8_return_to_continuation_objc_helper_host_execution_v0",
                    status: B8DebugObjcRuntimeHelperHostExecutionStatus::Executed,
                    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
                    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
                    effect: B8DebugReturnToContinuationObjcHelperEffect::SetActivationPolicy,
                    selector_name: request.selector_name().map(str::to_owned),
                    argument_value: request.argument_value(),
                    host_object_boundary: None,
                    appkit_run_loop_boundary: None,
                    output: Some(output),
                    next_source_pc: request.return_to,
                    next_continuation: next_continuation.map(Box::new),
                    error: None,
                    next_blocker,
                    next_action,
                }
                }
                Err(error) => Self::failed(request, error),
            },
            Err(error) => Self::failed(request, error),
        }
    }

    fn execute_set_delegate(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        code_bytes: &X86Bytes,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        if !cfg!(target_os = "macos") {
            return Self::skipped(
                request,
                B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHost,
            );
        }

        match run_public_objc_msg_send_set_delegate_helper() {
            Ok(observation) => {
                let output =
                    B8DebugReturnToContinuationObjcHelperHostOutputReport::from_set_delegate_observation(
                        observation,
                    );
                let preserved_r15_state = request
                    .receiver
                    .materialized_state
                    .as_ref()
                    .filter(|state| state.base_register == Some(B8DebugRegisterName::R15));
                let continuation_inputs = B8DebugReturnToContinuationDecodeInputs {
                    imported_global_value:
                        B8DebugReturnToContinuationImportedGlobalValue::nsapp_from_set_delegate_request(
                            request,
                        ),
                    preserved_call_target_import: Some(request.source_import.clone()),
                    preserved_r15_value: preserved_r15_state.and_then(|state| state.base_value),
                    preserved_r15_fixup_resolution: preserved_r15_state
                        .and_then(|state| state.base_fixup_resolution.clone()),
                    preserved_register_values: request.preserved_register_values.clone(),
                };
                let next_continuation =
                    B8DebugReturnToContinuationDecodeBoundaryReport::from_code_bytes(
                        request.return_to,
                        None,
                        continuation_inputs,
                        code_bytes,
                        input,
                        input_probe,
                        image_metadata,
                    );
                let next_blocker = next_continuation.as_ref().map_or(
                    Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented),
                    |continuation| continuation.blocker(),
                );
                let next_action = next_continuation.as_ref().map_or(
                    B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution,
                    |continuation| continuation.next_action,
                );
                Self {
                    schema: "b8_return_to_continuation_objc_helper_host_execution_v0",
                    status: B8DebugObjcRuntimeHelperHostExecutionStatus::Executed,
                    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
                    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
                    effect: B8DebugReturnToContinuationObjcHelperEffect::SetDelegate,
                    selector_name: request.selector_name().map(str::to_owned),
                    argument_value: request.argument_value(),
                    host_object_boundary:
                        B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectBoundaryReport::from_request(
                            request,
                    ),
                    appkit_run_loop_boundary: None,
                    output: Some(output),
                    next_source_pc: request.return_to,
                    next_continuation: next_continuation.map(Box::new),
                    error: None,
                    next_blocker,
                    next_action,
                }
            }
            Err(error) => Self::failed(request, error),
        }
    }

    fn execute_appkit_run_loop(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        code_bytes: &X86Bytes,
        input: &BinaryInput,
        input_probe: &BinaryFormatProbeReport,
        image_metadata: &ProgramImageMetadata,
    ) -> Self {
        if !cfg!(target_os = "macos") {
            return Self::skipped(
                request,
                B8DebugObjcRuntimeHelperErrorClassification::UnsupportedHost,
            );
        }

        match run_public_objc_msg_send_appkit_run_loop_helper() {
            Ok(observation) => {
                let output =
                    B8DebugReturnToContinuationObjcHelperHostOutputReport::from_appkit_run_loop_observation(
                        &observation,
                    );
                let preserved_r15_state = request
                    .receiver
                    .materialized_state
                    .as_ref()
                    .filter(|state| state.base_register == Some(B8DebugRegisterName::R15));
                let continuation_inputs = B8DebugReturnToContinuationDecodeInputs {
                    imported_global_value:
                        B8DebugReturnToContinuationImportedGlobalValue::nsapp_from_run_loop_request(
                            request,
                        ),
                    preserved_call_target_import: Some(request.source_import.clone()),
                    preserved_r15_value: preserved_r15_state.and_then(|state| state.base_value),
                    preserved_r15_fixup_resolution: preserved_r15_state
                        .and_then(|state| state.base_fixup_resolution.clone()),
                    preserved_register_values: request.preserved_register_values.clone(),
                };
                let next_continuation =
                    B8DebugReturnToContinuationDecodeBoundaryReport::from_code_bytes(
                        request.return_to,
                        None,
                        continuation_inputs,
                        code_bytes,
                        input,
                        input_probe,
                        image_metadata,
                    );
                let next_blocker = next_continuation.as_ref().map_or(
                    Some(B8DebugObjcHelperExecutionBlocker::ReturnToContinuationExecutionUnimplemented),
                    |continuation| continuation.blocker(),
                );
                let next_action = next_continuation.as_ref().map_or(
                    B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationExecution,
                    |continuation| continuation.next_action,
                );

                Self {
                    schema: "b8_return_to_continuation_objc_helper_host_execution_v0",
                    status: B8DebugObjcRuntimeHelperHostExecutionStatus::Executed,
                    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
                    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
                    effect: B8DebugReturnToContinuationObjcHelperEffect::RunApplication,
                    selector_name: request.selector_name().map(str::to_owned),
                    argument_value: request.argument_value(),
                    host_object_boundary: None,
                    appkit_run_loop_boundary:
                        B8DebugReturnToContinuationObjcHelperAppKitRunLoopBoundaryReport::executed_from_observation(
                            request,
                            &observation,
                            next_action,
                    ),
                    output: Some(output),
                    next_source_pc: request.return_to,
                    next_continuation: next_continuation.map(Box::new),
                    error: None,
                    next_blocker,
                    next_action,
                }
            }
            Err(error) => Self::failed(request, error),
        }
    }

    fn blocked(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
    ) -> Self {
        Self::with_error(
            request,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Blocked,
            classification,
            B8DebugObjcHelperExecutionBlocker::ReturnToContinuationObjcHelperExecutionUnimplemented,
            B8DebugReturnToContinuationDecodeNextAction::ImplementReturnToContinuationObjcHelperExecution,
            None,
        )
    }

    fn skipped(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
    ) -> Self {
        Self::with_error(
            request,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Skipped,
            classification,
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionUnsupported,
            B8DebugReturnToContinuationDecodeNextAction::RunReturnToContinuationObjcHelperOnSupportedMacosHost,
            None,
        )
    }

    fn failed(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        error: B8DebugObjcRuntimeHelperHostExecutionErrorReport,
    ) -> Self {
        Self::with_error(
            request,
            B8DebugObjcRuntimeHelperHostExecutionStatus::Failed,
            error.error_classification,
            B8DebugObjcHelperExecutionBlocker::ObjcRuntimeHelperHostExecutionFailed,
            B8DebugReturnToContinuationDecodeNextAction::InspectReturnToContinuationObjcHelperExecutionFailure,
            Some(error),
        )
    }

    fn with_error(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        status: B8DebugObjcRuntimeHelperHostExecutionStatus,
        classification: B8DebugObjcRuntimeHelperErrorClassification,
        next_blocker: B8DebugObjcHelperExecutionBlocker,
        next_action: B8DebugReturnToContinuationDecodeNextAction,
        error: Option<B8DebugObjcRuntimeHelperHostExecutionErrorReport>,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_objc_helper_host_execution_v0",
            status,
            api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
            fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
            effect: B8DebugReturnToContinuationObjcHelperEffect::from_request(request),
            selector_name: request.selector_name().map(str::to_owned),
            argument_value: request.argument_value(),
            host_object_boundary:
                B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectBoundaryReport::from_request(
                    request,
                ),
            appkit_run_loop_boundary: None,
            output: None,
            next_source_pc: request.return_to,
            next_continuation: None,
            error: error.or(Some(
                B8DebugObjcRuntimeHelperHostExecutionErrorReport::classification_only(
                    classification,
                ),
            )),
            next_blocker: Some(next_blocker),
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
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperHostObservation {
    #[serde(default)]
    return_value: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct B8DebugReturnToContinuationObjcHelperAppKitRunLoopHostObservation {
    schema: String,
    observed_event: B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleEventReport,
    termination_policy: B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationPolicyReport,
}

impl B8DebugReturnToContinuationObjcHelperAppKitRunLoopHostObservation {
    fn validate(self) -> Result<Self, B8DebugObjcRuntimeHelperHostExecutionErrorReport> {
        if self.schema != "b8_return_to_continuation_appkit_run_loop_host_observation_v0" {
            return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                format!(
                    "Objective-C AppKit run-loop helper emitted unexpected schema {:?}",
                    self.schema
                ),
            ));
        }
        if self.observed_event.event
            != B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleEventKind::GuiWindowCreated
            || self.observed_event.title != B8_GUI_HELLO_WORLD_TITLE
            || self.observed_event.text != B8_GUI_HELLO_WORLD_TEXT
        {
            return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                format!(
                    "Objective-C AppKit run-loop helper emitted unexpected lifecycle event {:?}",
                    self.observed_event
                ),
            ));
        }
        if self.termination_policy
            != B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationPolicyReport::self_authored_fixture_timer()
        {
            return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                format!(
                    "Objective-C AppKit run-loop helper emitted unexpected termination policy {:?}",
                    self.termination_policy
                ),
            ));
        }

        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    selector_name: &'static str,
    argument_register: B8DebugRegisterName,
    argument_value: Option<u64>,
    source: B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectSource,
    process_model: B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectProcessModel,
    raw_argument_pointer_reuse: B8DebugReturnToContinuationObjcHelperSetDelegateRawPointerReuse,
    substitute_class_name: &'static str,
}

impl B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectBoundaryReport {
    fn from_request(request: &B8DebugReturnToContinuationObjcHelperRequestReport) -> Option<Self> {
        if request.selector_name() != Some(B8_GUI_HELLO_WORLD_SET_DELEGATE_SELECTOR_NAME) {
            return None;
        }

        Some(Self {
            schema: "b8_return_to_continuation_set_delegate_host_object_boundary_v0",
            status: B8DebugImportBoundaryStatus::Executed,
            selector_name: B8_GUI_HELLO_WORLD_SET_DELEGATE_SELECTOR_NAME,
            argument_register: request.argument_register()?,
            argument_value: request.argument_value(),
            source:
                B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectSource::ObjcAllocInitFixtureDelegateHostSubstitute,
            process_model:
                B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectProcessModel::SameHelperProcessFixtureSubstitute,
            raw_argument_pointer_reuse:
                B8DebugReturnToContinuationObjcHelperSetDelegateRawPointerReuse::NotReusedAcrossHelperProcesses,
            substitute_class_name: B8_GUI_HELLO_WORLD_DELEGATE_CLASS_NAME,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectSource {
    ObjcAllocInitFixtureDelegateHostSubstitute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperSetDelegateHostObjectProcessModel {
    SameHelperProcessFixtureSubstitute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperSetDelegateRawPointerReuse {
    NotReusedAcrossHelperProcesses,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperAppKitRunLoopBoundaryReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    selector_name: &'static str,
    receiver: B8DebugReturnToContinuationObjcHelperReceiver,
    execution_model: B8DebugReturnToContinuationObjcHelperAppKitRunLoopExecutionModel,
    lifecycle_scope: B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleScope,
    lifecycle_observation:
        Option<B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleObservationReport>,
    bounded_termination_policy:
        Option<B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationPolicyReport>,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationObjcHelperAppKitRunLoopBoundaryReport {
    fn executed_from_observation(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        observation: &B8DebugReturnToContinuationObjcHelperAppKitRunLoopHostObservation,
        next_action: B8DebugReturnToContinuationDecodeNextAction,
    ) -> Option<Self> {
        if !request.is_supported_b8_run_message() {
            return None;
        }

        Some(Self {
            schema: "b8_return_to_continuation_appkit_run_loop_boundary_v0",
            status: B8DebugImportBoundaryStatus::Executed,
            selector_name: B8_GUI_HELLO_WORLD_RUN_SELECTOR_NAME,
            receiver: B8DebugReturnToContinuationObjcHelperReceiver::from_argument(
                &request.receiver,
            ),
            execution_model:
                B8DebugReturnToContinuationObjcHelperAppKitRunLoopExecutionModel::NsApplicationRunLoopEntry,
            lifecycle_scope:
                B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleScope::SelfAuthoredB8GuiFixture,
            lifecycle_observation: Some(
                B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleObservationReport::from_observation(
                    observation,
                ),
            ),
            bounded_termination_policy: Some(observation.termination_policy),
            blocker: None,
            next_action,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperAppKitRunLoopExecutionModel {
    NsApplicationRunLoopEntry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleScope {
    SelfAuthoredB8GuiFixture,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleObservationReport {
    schema: &'static str,
    source: B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleObservationSource,
    delegate_class_name: &'static str,
    delegate_callback: B8DebugReturnToContinuationObjcHelperAppKitRunLoopDelegateCallback,
    observed_event: B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleEventReport,
}

impl B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleObservationReport {
    fn from_observation(
        observation: &B8DebugReturnToContinuationObjcHelperAppKitRunLoopHostObservation,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_appkit_run_loop_lifecycle_observation_v0",
            source:
                B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleObservationSource::FixtureDelegateApplicationDidFinishLaunchingStdoutEvent,
            delegate_class_name: B8_GUI_HELLO_WORLD_DELEGATE_CLASS_NAME,
            delegate_callback:
                B8DebugReturnToContinuationObjcHelperAppKitRunLoopDelegateCallback::ApplicationDidFinishLaunching,
            observed_event: observation.observed_event.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleObservationSource {
    FixtureDelegateApplicationDidFinishLaunchingStdoutEvent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum B8DebugReturnToContinuationObjcHelperAppKitRunLoopDelegateCallback {
    #[serde(rename = "applicationDidFinishLaunching:")]
    ApplicationDidFinishLaunching,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleEventReport {
    event: B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleEventKind,
    title: String,
    text: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperAppKitRunLoopLifecycleEventKind {
    GuiWindowCreated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationPolicyReport {
    trigger: B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationTrigger,
    delay_millis: u64,
    termination_request: B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationRequest,
}

impl B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationPolicyReport {
    const fn self_authored_fixture_timer() -> Self {
        Self {
            trigger:
                B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationTrigger::TimerAfterGuiWindowCreated,
            delay_millis: 100,
            termination_request:
                B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationRequest::NsAppTerminateNil,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationTrigger {
    TimerAfterGuiWindowCreated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperAppKitRunLoopTerminationRequest {
    NsAppTerminateNil,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperHostOutputReport {
    helper_output: B8DebugObjcRuntimeHelperOutput,
    representation: B8DebugReturnToContinuationObjcHelperOutputRepresentation,
    effect: B8DebugReturnToContinuationObjcHelperEffect,
    return_value: Option<u64>,
}

impl B8DebugReturnToContinuationObjcHelperHostOutputReport {
    fn from_set_activation_policy_observation(
        observation: B8DebugReturnToContinuationObjcHelperHostObservation,
    ) -> Result<Self, B8DebugObjcRuntimeHelperHostExecutionErrorReport> {
        let return_value = observation.return_value.ok_or_else(|| {
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                "Objective-C setActivationPolicy helper emitted no return value",
            )
        })?;
        Ok(Self {
            helper_output: B8DebugObjcRuntimeHelperOutput::ObjcHelperReturnValue,
            representation: B8DebugReturnToContinuationObjcHelperOutputRepresentation::BoolAsU64,
            effect: B8DebugReturnToContinuationObjcHelperEffect::SetActivationPolicy,
            return_value: Some(return_value),
        })
    }

    const fn from_set_delegate_observation(
        _observation: B8DebugReturnToContinuationObjcHelperHostObservation,
    ) -> Self {
        Self {
            helper_output: B8DebugObjcRuntimeHelperOutput::ObjcHelperVoidReturn,
            representation:
                B8DebugReturnToContinuationObjcHelperOutputRepresentation::VoidNoReturnValue,
            effect: B8DebugReturnToContinuationObjcHelperEffect::SetDelegate,
            return_value: None,
        }
    }

    const fn from_appkit_run_loop_observation(
        _observation: &B8DebugReturnToContinuationObjcHelperAppKitRunLoopHostObservation,
    ) -> Self {
        Self {
            helper_output: B8DebugObjcRuntimeHelperOutput::ObjcHelperVoidReturn,
            representation:
                B8DebugReturnToContinuationObjcHelperOutputRepresentation::VoidNoReturnValue,
            effect: B8DebugReturnToContinuationObjcHelperEffect::RunApplication,
            return_value: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperOutputRepresentation {
    #[serde(rename = "bool_as_u64")]
    BoolAsU64,
    VoidNoReturnValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperBridgeContractReport {
    schema: &'static str,
    status: B8DebugImportBoundaryStatus,
    api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary,
    fixture_scope: B8DebugObjcRuntimeHelperFixtureScope,
    input_contract: B8DebugReturnToContinuationObjcHelperInputContractReport,
    output_contract: B8DebugReturnToContinuationObjcHelperOutputContractReport,
    error_contract: B8DebugReturnToContinuationObjcHelperErrorContractReport,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
    next_action: B8DebugReturnToContinuationDecodeNextAction,
}

impl B8DebugReturnToContinuationObjcHelperBridgeContractReport {
    fn from_host_execution(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        available_or_blocked_state: B8DebugReturnToContinuationObjcHelperStateReport,
        host_execution: &B8DebugReturnToContinuationObjcHelperHostExecutionReport,
    ) -> Self {
        Self {
            schema: "b8_return_to_continuation_objc_helper_bridge_contract_v0",
            status: if host_execution.is_executed() {
                B8DebugImportBoundaryStatus::Executed
            } else if host_execution.is_skipped() {
                B8DebugImportBoundaryStatus::Skipped
            } else {
                B8DebugImportBoundaryStatus::Blocked
            },
            api_boundary: B8DebugObjcRuntimeHelperHostApiBoundary::PublicObjcRuntimeAppKit,
            fixture_scope: B8DebugObjcRuntimeHelperFixtureScope::SelfAuthoredB8GuiFixture,
            input_contract: B8DebugReturnToContinuationObjcHelperInputContractReport::from_request(
                request,
                available_or_blocked_state,
            ),
            output_contract:
                B8DebugReturnToContinuationObjcHelperOutputContractReport::from_host_execution(
                    host_execution,
                ),
            error_contract:
                B8DebugReturnToContinuationObjcHelperErrorContractReport::from_host_execution(
                    host_execution,
                ),
            blocker: host_execution.next_blocker,
            next_action: host_execution.next_action,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperInputContractReport {
    function: B8DebugObjcRuntimeHelperMessageSendFunction,
    receiver: B8DebugReturnToContinuationObjcHelperReceiver,
    selector_name: Option<String>,
    argument_model: B8DebugReturnToContinuationObjcHelperArgumentModel,
    argument_register: Option<B8DebugRegisterName>,
    argument_value: Option<u64>,
    required_capability: B8DebugObjcHelperExecutionCapability,
    calling_convention: B8DebugHelperCallingConvention,
    available_or_blocked_state: B8DebugReturnToContinuationObjcHelperStateReport,
}

impl B8DebugReturnToContinuationObjcHelperInputContractReport {
    fn from_request(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        available_or_blocked_state: B8DebugReturnToContinuationObjcHelperStateReport,
    ) -> Self {
        Self {
            function: B8DebugObjcRuntimeHelperMessageSendFunction::ObjcMsgSend,
            receiver: B8DebugReturnToContinuationObjcHelperReceiver::from_argument(
                &request.receiver,
            ),
            selector_name: request.selector_name().map(str::to_owned),
            argument_model: request.argument_model(),
            argument_register: request.argument_register(),
            argument_value: request.argument_value(),
            required_capability: request.required_capability,
            calling_convention: B8DebugHelperCallingConvention::X8664MacosSystemV,
            available_or_blocked_state,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperReceiver {
    NsAppSharedApplicationValue,
    Unknown,
}

impl B8DebugReturnToContinuationObjcHelperReceiver {
    fn from_argument(argument: &B8DebugReturnToContinuationCallArgumentReport) -> Self {
        if argument.materialized_state.as_ref().is_some_and(|state| {
            state.source
                == B8DebugReturnToContinuationMaterializedRegisterSource::ImportedGlobalPointee
                && state.value_source
                    == Some(
                        B8DebugReturnToContinuationMaterializedRegisterValueSource::ObjcSharedApplicationHelperReturnValue,
                    )
        }) {
            Self::NsAppSharedApplicationValue
        } else {
            Self::Unknown
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperOutputContractReport {
    status: B8DebugImportBoundaryStatus,
    effect: B8DebugReturnToContinuationObjcHelperEffect,
    return_value_handling: B8DebugReturnToContinuationObjcHelperReturnValueHandling,
    blocker: Option<B8DebugObjcHelperExecutionBlocker>,
}

impl B8DebugReturnToContinuationObjcHelperOutputContractReport {
    fn from_host_execution(
        host_execution: &B8DebugReturnToContinuationObjcHelperHostExecutionReport,
    ) -> Self {
        let status = if host_execution.is_executed() {
            B8DebugImportBoundaryStatus::Executed
        } else if host_execution.is_skipped() {
            B8DebugImportBoundaryStatus::Skipped
        } else {
            B8DebugImportBoundaryStatus::Blocked
        };
        let return_value_handling = if host_execution.is_executed() {
            match host_execution.effect {
                B8DebugReturnToContinuationObjcHelperEffect::SetActivationPolicy => {
                    B8DebugReturnToContinuationObjcHelperReturnValueHandling::CapturedAsX8664RaxReturnValue
                }
                B8DebugReturnToContinuationObjcHelperEffect::SetDelegate => {
                    B8DebugReturnToContinuationObjcHelperReturnValueHandling::NoX8664ReturnValueObserved
                }
                B8DebugReturnToContinuationObjcHelperEffect::RunApplication => {
                    B8DebugReturnToContinuationObjcHelperReturnValueHandling::NoX8664ReturnValueObserved
                }
                B8DebugReturnToContinuationObjcHelperEffect::Unknown => {
                    B8DebugReturnToContinuationObjcHelperReturnValueHandling::DeferredUntilHelperExecution
                }
            }
        } else {
            B8DebugReturnToContinuationObjcHelperReturnValueHandling::DeferredUntilHelperExecution
        };
        Self {
            status,
            effect: host_execution.effect,
            return_value_handling,
            blocker: if host_execution.is_executed() {
                None
            } else {
                host_execution.next_blocker
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperEffect {
    RunApplication,
    SetActivationPolicy,
    SetDelegate,
    Unknown,
}

impl B8DebugReturnToContinuationObjcHelperEffect {
    fn from_request(request: &B8DebugReturnToContinuationObjcHelperRequestReport) -> Self {
        match request.selector_name() {
            Some("setActivationPolicy:") => Self::SetActivationPolicy,
            Some(B8_GUI_HELLO_WORLD_SET_DELEGATE_SELECTOR_NAME) => Self::SetDelegate,
            Some(B8_GUI_HELLO_WORLD_RUN_SELECTOR_NAME) => Self::RunApplication,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugReturnToContinuationObjcHelperReturnValueHandling {
    #[serde(rename = "captured_as_x86_64_rax_return_value")]
    CapturedAsX8664RaxReturnValue,
    DeferredUntilHelperExecution,
    #[serde(rename = "no_x86_64_return_value_observed")]
    NoX8664ReturnValueObserved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperErrorContractReport {
    error_classification: Option<B8DebugObjcHelperExecutionBlocker>,
}

impl B8DebugReturnToContinuationObjcHelperErrorContractReport {
    fn from_host_execution(
        host_execution: &B8DebugReturnToContinuationObjcHelperHostExecutionReport,
    ) -> Self {
        Self {
            error_classification: if host_execution.error.is_some() {
                host_execution.next_blocker
            } else {
                None
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugReturnToContinuationObjcHelperStateReport {
    target_state: B8DebugValueMaterializationStatus,
    receiver_state: B8DebugValueMaterializationStatus,
    selector_state: B8DebugValueMaterializationStatus,
    argument_state: B8DebugValueMaterializationStatus,
    execution_state: B8DebugImportBoundaryStatus,
}

impl B8DebugReturnToContinuationObjcHelperStateReport {
    fn from_request_and_host_execution(
        request: &B8DebugReturnToContinuationObjcHelperRequestReport,
        host_execution: &B8DebugReturnToContinuationObjcHelperHostExecutionReport,
    ) -> Self {
        Self {
            target_state: B8DebugValueMaterializationStatus::Available,
            receiver_state: request.receiver.state,
            selector_state: request.selector.state,
            argument_state: request.argument_state(),
            execution_state: if host_execution.is_executed() {
                B8DebugImportBoundaryStatus::Executed
            } else if host_execution.is_skipped() {
                B8DebugImportBoundaryStatus::Skipped
            } else {
                B8DebugImportBoundaryStatus::Blocked
            },
        }
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
    let stdout =
        run_public_objc_runtime_helper_source(B8_OBJC_RUNTIME_SHARED_APPLICATION_HELPER_SOURCE)?;
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

fn run_public_objc_autorelease_pool_push_helper(
) -> Result<B8DebugObjcRuntimeHelperHostObservation, B8DebugObjcRuntimeHelperHostExecutionErrorReport>
{
    let stdout =
        run_public_objc_runtime_helper_source(B8_OBJC_RUNTIME_AUTORELEASE_POOL_PUSH_HELPER_SOURCE)?;
    let observation: B8DebugObjcRuntimeHelperHostObservation = serde_json::from_str(&stdout)
        .map_err(|error| {
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                format!(
                    "Objective-C autorelease pool helper emitted invalid JSON: {error}; stdout={stdout:?}"
                ),
            )
        })?;
    if observation.return_value == 0 {
        return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
            B8DebugObjcRuntimeHelperErrorClassification::EmptyHelperReturnValue,
            "Objective-C autorelease pool helper returned a null token",
        ));
    }

    Ok(observation)
}

fn run_public_objc_msg_send_set_activation_policy_helper() -> Result<
    B8DebugReturnToContinuationObjcHelperHostObservation,
    B8DebugObjcRuntimeHelperHostExecutionErrorReport,
> {
    let stdout =
        run_public_objc_runtime_helper_source(B8_OBJC_RUNTIME_SET_ACTIVATION_POLICY_HELPER_SOURCE)?;
    serde_json::from_str(&stdout).map_err(|error| {
        B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
            B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
            format!("Objective-C runtime helper emitted invalid JSON: {error}; stdout={stdout:?}"),
        )
    })
}

fn run_public_objc_msg_send_set_delegate_helper() -> Result<
    B8DebugReturnToContinuationObjcHelperHostObservation,
    B8DebugObjcRuntimeHelperHostExecutionErrorReport,
> {
    let stdout = run_public_objc_runtime_helper_source(B8_OBJC_RUNTIME_SET_DELEGATE_HELPER_SOURCE)?;
    serde_json::from_str(&stdout).map_err(|error| {
        B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
            B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
            format!(
                "Objective-C setDelegate helper emitted invalid JSON: {error}; stdout={stdout:?}"
            ),
        )
    })
}

fn run_public_objc_msg_send_appkit_run_loop_helper() -> Result<
    B8DebugReturnToContinuationObjcHelperAppKitRunLoopHostObservation,
    B8DebugObjcRuntimeHelperHostExecutionErrorReport,
> {
    let stdout =
        run_public_objc_runtime_helper_source(B8_OBJC_RUNTIME_APPKIT_RUN_LOOP_HELPER_SOURCE)?;
    let observation: B8DebugReturnToContinuationObjcHelperAppKitRunLoopHostObservation =
        serde_json::from_str(&stdout).map_err(|error| {
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                format!(
                    "Objective-C AppKit run-loop helper emitted invalid JSON: {error}; stdout={stdout:?}"
                ),
            )
        })?;

    observation.validate()
}

fn run_public_objc_alloc_init_fixture_delegate_helper(
) -> Result<B8DebugObjcRuntimeHelperHostObservation, B8DebugObjcRuntimeHelperHostExecutionErrorReport>
{
    let stdout =
        run_public_objc_runtime_helper_source(B8_OBJC_ALLOC_INIT_FIXTURE_DELEGATE_HELPER_SOURCE)?;
    let observation: B8DebugObjcRuntimeHelperHostObservation = serde_json::from_str(&stdout)
        .map_err(|error| {
            B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
                B8DebugObjcRuntimeHelperErrorClassification::InvalidHelperOutput,
                format!(
                    "Objective-C fixture delegate helper emitted invalid JSON: {error}; stdout={stdout:?}"
                ),
            )
        })?;
    if observation.return_value == 0 {
        return Err(B8DebugObjcRuntimeHelperHostExecutionErrorReport::message(
            B8DebugObjcRuntimeHelperErrorClassification::EmptyHelperReturnValue,
            "Objective-C fixture delegate helper returned a null object pointer",
        ));
    }

    Ok(observation)
}

fn run_public_objc_runtime_helper_source(
    source: &str,
) -> Result<String, B8DebugObjcRuntimeHelperHostExecutionErrorReport> {
    let source_path = temporary_objc_runtime_helper_path("m")?;
    let executable_path = temporary_objc_runtime_helper_path("exe")?;
    if let Err(error) = fs::write(&source_path, source) {
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

    Ok(String::from_utf8_lossy(&run_output.stdout).into_owned())
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

const B8_OBJC_RUNTIME_AUTORELEASE_POOL_PUSH_HELPER_SOURCE: &str = r#"
#import <objc/objc-auto.h>
#include <stdint.h>
#include <stdio.h>

extern void *objc_autoreleasePoolPush(void);
extern void objc_autoreleasePoolPop(void *pool);

int main(void) {
    void *pool = objc_autoreleasePoolPush();
    uintptr_t value = (uintptr_t)pool;
    objc_autoreleasePoolPop(pool);
    if (value == 0) {
        return 2;
    }
    printf("{\"schema\":\"b8_objc_autorelease_pool_push_host_observation_v0\",\"return_value\":%llu}\n",
           (unsigned long long)value);
    return 0;
}
"#;

const B8_OBJC_RUNTIME_SET_ACTIVATION_POLICY_HELPER_SOURCE: &str = r#"
#import <AppKit/AppKit.h>
#import <objc/message.h>
#import <objc/runtime.h>
#include <stdint.h>
#include <stdio.h>

int main(void) {
    @autoreleasepool {
        Class application_class = NSClassFromString(@"NSApplication");
        SEL shared_application = sel_registerName("sharedApplication");
        SEL set_activation_policy = sel_registerName("setActivationPolicy:");
        id (*send_id)(id, SEL) = (id (*)(id, SEL))objc_msgSend;
        BOOL (*send_bool_int)(id, SEL, NSInteger) =
            (BOOL (*)(id, SEL, NSInteger))objc_msgSend;
        id app = send_id((id)application_class, shared_application);
        uintptr_t value = (uintptr_t)app;
        if (value == 0) {
            return 2;
        }
        BOOL result = send_bool_int(app, set_activation_policy, 0);
        printf("{\"schema\":\"b8_return_to_continuation_objc_helper_host_observation_v0\",\"return_value\":%llu}\n",
               (unsigned long long)(result ? 1 : 0));
    }
    return 0;
}
"#;

const B8_OBJC_RUNTIME_SET_DELEGATE_HELPER_SOURCE: &str = r#"
#import <AppKit/AppKit.h>
#import <objc/message.h>
#import <objc/runtime.h>
#include <stdint.h>
#include <stdio.h>

@interface BaraGuiHelloWorldDelegate : NSObject <NSApplicationDelegate, NSWindowDelegate>
@end

@implementation BaraGuiHelloWorldDelegate
@end

int main(void) {
    @autoreleasepool {
        Class application_class = NSClassFromString(@"NSApplication");
        SEL shared_application = sel_registerName("sharedApplication");
        SEL set_delegate = sel_registerName("setDelegate:");
        SEL delegate_selector = sel_registerName("delegate");
        id (*send_id)(id, SEL) = (id (*)(id, SEL))objc_msgSend;
        void (*send_void_id)(id, SEL, id) = (void (*)(id, SEL, id))objc_msgSend;
        id app = send_id((id)application_class, shared_application);
        if ((uintptr_t)app == 0) {
            return 2;
        }
        id delegate = [[BaraGuiHelloWorldDelegate alloc] init];
        if ((uintptr_t)delegate == 0) {
            return 3;
        }
        send_void_id(app, set_delegate, delegate);
        id observed_delegate = send_id(app, delegate_selector);
        if (observed_delegate != delegate) {
            return 4;
        }
        printf("{\"schema\":\"b8_return_to_continuation_objc_helper_set_delegate_host_observation_v0\",\"return_value\":null}\n");
    }
    return 0;
}
"#;

const B8_OBJC_RUNTIME_APPKIT_RUN_LOOP_HELPER_SOURCE: &str = r#"
#import <AppKit/AppKit.h>
#include <stdio.h>

@interface BaraGuiHelloWorldDelegate : NSObject <NSApplicationDelegate, NSWindowDelegate> {
    NSWindow *_window;
}
@end

@implementation BaraGuiHelloWorldDelegate

- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    (void)notification;

    NSRect frame = NSMakeRect(200.0, 200.0, 360.0, 140.0);
    _window = [[NSWindow alloc]
        initWithContentRect:frame
                  styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable)
                    backing:NSBackingStoreBuffered
                      defer:NO];
    [_window setDelegate:self];
    [_window setTitle:@"Bara GUI Hello World"];

    NSTextField *label =
        [[NSTextField alloc] initWithFrame:NSMakeRect(20.0, 55.0, 320.0, 24.0)];
    [label setStringValue:@"hello world"];
    [label setEditable:NO];
    [label setBordered:NO];
    [label setDrawsBackground:NO];
    [label setAlignment:NSTextAlignmentCenter];
    [[_window contentView] addSubview:label];

    [NSApp activateIgnoringOtherApps:YES];
    [_window makeKeyAndOrderFront:nil];

    puts("{\"schema\":\"b8_return_to_continuation_appkit_run_loop_host_observation_v0\",\"observed_event\":{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"},\"termination_policy\":{\"trigger\":\"timer_after_gui_window_created\",\"delay_millis\":100,\"termination_request\":\"ns_app_terminate_nil\"}}");
    fflush(stdout);

    [NSTimer scheduledTimerWithTimeInterval:0.1
                                     target:self
                                   selector:@selector(terminateApplication:)
                                   userInfo:nil
                                    repeats:NO];
}

- (void)terminateApplication:(NSTimer *)timer {
    (void)timer;
    [NSApp terminate:nil];
}

- (void)windowWillClose:(NSNotification *)notification {
    (void)notification;
    [NSApp terminate:nil];
}

@end

int main(void) {
    @autoreleasepool {
        freopen("/dev/null", "w", stderr);

        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];

        BaraGuiHelloWorldDelegate *delegate =
            [[BaraGuiHelloWorldDelegate alloc] init];
        [NSApp setDelegate:delegate];
        [NSApp run];
    }

    return 0;
}
"#;

const B8_OBJC_ALLOC_INIT_FIXTURE_DELEGATE_HELPER_SOURCE: &str = r#"
#import <AppKit/AppKit.h>
#include <stdint.h>
#include <stdio.h>

@interface BaraGuiHelloWorldDelegate : NSObject <NSApplicationDelegate, NSWindowDelegate>
@end

@implementation BaraGuiHelloWorldDelegate
@end

int main(void) {
    @autoreleasepool {
        id delegate = [[BaraGuiHelloWorldDelegate alloc] init];
        uintptr_t value = (uintptr_t)delegate;
        if (value == 0) {
            return 2;
        }
        printf("{\"schema\":\"b8_return_to_continuation_objc_alloc_init_fixture_delegate_host_observation_v0\",\"return_value\":%llu}\n",
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
    ObjcHelperVoidReturn,
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
    NotRequired,
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

#[derive(Debug)]
pub(crate) enum B8DebugBundleError {
    ReadFile {
        path: PathBuf,
        source: std_io::Error,
    },
    WriteFile {
        path: PathBuf,
        source: std_io::Error,
    },
    CreateDir {
        path: PathBuf,
        source: std_io::Error,
    },
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

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
