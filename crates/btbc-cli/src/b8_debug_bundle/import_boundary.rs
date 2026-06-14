use bara_ir::ProgramImageMetadata;
use bara_isa_x86::X86Bytes;
use bara_oracle::{
    decode_mach_o_chained_fixups_for_target, BinaryFormatProbeReport, BinaryInput,
    MachOChainedFixupTargetAddress, MachOChainedFixupsTargetReport, MachODyldInfoCommandKind,
    MachODylibImportCommandKind, MachOLinkeditDataCommandKind,
};
use serde::Serialize;

use super::report::B8DebugDecodeReport;
use super::{
    B8DebugHelperBoundaryBlockedReason, B8DebugHelperBoundaryRequestReport,
    B8DebugImportBoundaryStatus, B8DebugRegisterIndirectCallBoundaryReport,
    B8DebugTargetPointerLoadReport,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugImportBoundaryReport {
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
    pub(super) fn from_probe_and_decode_report(
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
            let (resolution, next_action, helper_boundary_request) = if public_metadata
                .has_chained_fixups()
            {
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
            let status = if helper_boundary_request.status == B8DebugImportBoundaryStatus::Executed
            {
                B8DebugImportBoundaryStatus::Executed
            } else {
                B8DebugImportBoundaryStatus::Blocked
            };
            let next_action = if status == B8DebugImportBoundaryStatus::Executed {
                B8DebugImportBoundaryNextAction::InspectNextDebugBundleBlocker
            } else {
                next_action
            };

            return Self {
                status,
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

    pub(super) fn helper_boundary_request(&self) -> B8DebugHelperBoundaryRequestReport {
        self.helper_boundary_request.clone()
    }
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
