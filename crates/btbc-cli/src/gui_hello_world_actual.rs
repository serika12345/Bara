use bara_oracle::{
    compare_observed_results, BinaryFormat, BinaryFormatProbeReport, BinaryFormatProbeStatus,
    CaseId, ComparisonReport, MachOMetadata, ObservedResult,
};
use bara_runtime::{
    UserSpaceBridgeBoundaryPlacement, UserSpaceBridgeCoreImplementation,
    UserSpaceEntryTrampolineTarget, UserSpaceExecutableMemoryAllocationApi,
    UserSpaceExecutableMemoryProtectionTransition, UserSpaceExecutableMemoryReleaseApi,
    UserSpaceExecutionStrategyAvailability, UserSpaceExecutionStrategyBoundary,
    UserSpaceFallbackEngineStatus, UserSpaceFallbackPolicyAction, UserSpaceFeedbackCycleState,
    UserSpaceHelperBoundaryContract, UserSpaceHelperBoundaryNextBlocker,
    UserSpaceHelperBoundaryPublicImport, UserSpaceHelperBoundaryResolution,
    UserSpaceHelperBoundaryStatus, UserSpaceHelperCapabilityConnection,
    UserSpaceHelperCapabilityContract, UserSpaceHelperCapabilityStatus,
    UserSpaceHelperObservationContract, UserSpaceImageMappingSource, UserSpaceInitialStackContract,
    UserSpaceLaunchPlan, UserSpaceLaunchResponsibility, UserSpaceLoaderEntryPointPlan,
    UserSpaceLoaderExecutionStatus, UserSpaceLoaderImportPlan, UserSpaceLoaderMetadataSource,
    UserSpaceLoaderObjcRuntimePlan, UserSpaceLoaderRelocationPlan,
    UserSpaceLoaderSegmentMappingPlan, UserSpaceMacosCodeSigningPolicy,
    UserSpaceMacosHardenedRuntimePolicy, UserSpaceMacosWriteXorExecutePolicy,
    UserSpaceMemoryProtectionModel, UserSpacePlatformExceptionModel,
    UserSpacePlatformMemoryProtectionModel, UserSpacePlatformSignalModel,
    UserSpacePlatformThreadModel, UserSpacePlatformTlsModel,
    UserSpacePrivateIntegrationRequirement, UserSpaceProcessScope, UserSpaceSourceIsaMode,
    UserSpaceSourceIsaProfile, UserSpaceSourceWidth,
};
use serde::Serialize;

use crate::x86_64_mach_o_fixture::{b8_gui_hello_world_case_id, X8664MachOFixtureError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GuiHelloWorldActualLaunchBundle {
    observed_result: ObservedResult,
    launch_report: GuiHelloWorldActualLaunchReport,
}

impl GuiHelloWorldActualLaunchBundle {
    fn blocked_by_initial_blocker(
        case_id: CaseId,
        input_metadata: GuiHelloWorldActualInputMetadata,
        classification_plan: GuiHelloWorldInitialBlockerPlan,
    ) -> Self {
        let classification = classification_plan.selected_classification();
        let launch_result =
            GuiHelloWorldActualLaunchResult::blocked_by_classification(classification);
        let observed_result = launch_result.to_observed_result(case_id.clone());
        let launch_report = GuiHelloWorldActualLaunchReport::blocked_by_initial_blocker(
            case_id,
            input_metadata,
            classification_plan,
            launch_result,
        );

        Self {
            observed_result,
            launch_report,
        }
    }

    pub(crate) const fn observed_result(&self) -> &ObservedResult {
        &self.observed_result
    }

    pub(crate) const fn launch_report(&self) -> &GuiHelloWorldActualLaunchReport {
        &self.launch_report
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct GuiHelloWorldActualLaunchReport {
    schema: &'static str,
    case_id: CaseId,
    actual_runtime: GuiHelloWorldActualRuntime,
    status: GuiHelloWorldActualLaunchStatus,
    input: GuiHelloWorldActualInput,
    runtime_preparation: GuiHelloWorldActualRuntimePreparation,
    launch_result: GuiHelloWorldActualLaunchResult,
    blocker: GuiHelloWorldActualBlocker,
}

impl GuiHelloWorldActualLaunchReport {
    fn blocked_by_initial_blocker(
        case_id: CaseId,
        input_metadata: GuiHelloWorldActualInputMetadata,
        classification_plan: GuiHelloWorldInitialBlockerPlan,
        launch_result: GuiHelloWorldActualLaunchResult,
    ) -> Self {
        Self {
            schema: "b8_gui_hello_world_actual_launch_report_v0",
            case_id,
            actual_runtime: GuiHelloWorldActualRuntime::BaraArm64UserSpace,
            status: GuiHelloWorldActualLaunchStatus::Blocked,
            input: GuiHelloWorldActualInput::from_metadata(input_metadata),
            runtime_preparation: GuiHelloWorldActualRuntimePreparation::from_plan(
                &UserSpaceLaunchPlan::mach_o_executable_image(),
            ),
            launch_result,
            blocker: GuiHelloWorldActualBlocker::from_classification_plan(&classification_plan),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct GuiHelloWorldFeedbackReport {
    schema: &'static str,
    case_id: CaseId,
    status: GuiHelloWorldFeedbackStatus,
    comparison: ComparisonReport,
    current_blocker: GuiHelloWorldActualBlocker,
    loader_execution_plan: GuiHelloWorldActualLoaderExecutionPreparation,
    helper_boundary_plan: GuiHelloWorldActualHelperBoundaryPreparation,
    helper_capability_plan: GuiHelloWorldActualHelperCapabilityPreparation,
    next_action: GuiHelloWorldFeedbackNextAction,
}

impl GuiHelloWorldFeedbackReport {
    fn from_expected_and_actual(
        expected: &ObservedResult,
        actual: &GuiHelloWorldActualLaunchBundle,
    ) -> Self {
        let comparison = compare_observed_results(expected, actual.observed_result());
        let status = GuiHelloWorldFeedbackStatus::from_comparison(&comparison);

        Self {
            schema: "b8_gui_hello_world_feedback_report_v0",
            case_id: expected.case_id().clone(),
            status,
            comparison,
            current_blocker: actual.launch_report.blocker.clone(),
            loader_execution_plan: actual.launch_report.runtime_preparation.loader_execution,
            helper_boundary_plan: actual.launch_report.runtime_preparation.helper_boundary,
            helper_capability_plan: actual.launch_report.runtime_preparation.helper_capability,
            next_action: GuiHelloWorldFeedbackNextAction::ConnectAppKitLifecycleHelperExecution,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldFeedbackStatus {
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "matched")]
    Matched,
}

impl GuiHelloWorldFeedbackStatus {
    fn from_comparison(comparison: &ComparisonReport) -> Self {
        if comparison.is_match() {
            Self::Matched
        } else {
            Self::Blocked
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldFeedbackNextAction {
    #[serde(rename = "connect_appkit_lifecycle_helper_execution")]
    ConnectAppKitLifecycleHelperExecution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualRuntime {
    #[serde(rename = "bara_arm64_user_space")]
    BaraArm64UserSpace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLaunchStatus {
    #[serde(rename = "blocked")]
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualLaunchResult {
    exit_status: i32,
    return_value: u64,
    stdout: &'static str,
    stderr: &'static str,
}

impl GuiHelloWorldActualLaunchResult {
    const fn blocked_by_classification(
        classification: GuiHelloWorldActualBlockerClassification,
    ) -> Self {
        Self {
            exit_status: 1,
            return_value: 0,
            stdout: "",
            stderr: classification.stderr_message(),
        }
    }

    fn to_observed_result(self, case_id: CaseId) -> ObservedResult {
        ObservedResult::new(
            case_id,
            self.exit_status,
            self.return_value,
            self.stdout.to_owned(),
            self.stderr.to_owned(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualRuntimePreparation {
    source: GuiHelloWorldActualRuntimePreparationSource,
    status: GuiHelloWorldActualRuntimePreparationStatus,
    source_isa_profile: GuiHelloWorldActualSourceIsaProfile,
    image_mapping: GuiHelloWorldActualImageMappingPreparation,
    executable_memory: GuiHelloWorldActualExecutableMemoryPreparation,
    execution_strategy: GuiHelloWorldActualExecutionStrategyPreparation,
    entry_trampoline: GuiHelloWorldActualEntryTrampolinePreparation,
    initial_stack: GuiHelloWorldActualInitialStackPreparation,
    helper_boundary: GuiHelloWorldActualHelperBoundaryPreparation,
    helper_capability: GuiHelloWorldActualHelperCapabilityPreparation,
    bridge_boundary: GuiHelloWorldActualBridgeBoundaryPreparation,
    integration_policy: GuiHelloWorldActualIntegrationPolicy,
    process_boundary: GuiHelloWorldActualProcessBoundary,
    platform_model: GuiHelloWorldActualPlatformModel,
    macos_constraints: GuiHelloWorldActualMacosConstraints,
    fallback_policy: GuiHelloWorldActualFallbackPolicy,
    loader_execution: GuiHelloWorldActualLoaderExecutionPreparation,
}

impl GuiHelloWorldActualRuntimePreparation {
    const fn from_plan(plan: &UserSpaceLaunchPlan) -> Self {
        Self {
            source: GuiHelloWorldActualRuntimePreparationSource::BaraRuntimeUserSpaceLaunchPlan,
            status: GuiHelloWorldActualRuntimePreparationStatus::PlannedNotExecuted,
            source_isa_profile: GuiHelloWorldActualSourceIsaProfile::from_profile(
                plan.source_isa_profile(),
            ),
            image_mapping: GuiHelloWorldActualImageMappingPreparation::from_plan(
                plan.image_mapping(),
            ),
            executable_memory: GuiHelloWorldActualExecutableMemoryPreparation::from_plan(
                plan.executable_memory(),
            ),
            execution_strategy: GuiHelloWorldActualExecutionStrategyPreparation::from_plan(
                plan.execution_strategy(),
            ),
            entry_trampoline: GuiHelloWorldActualEntryTrampolinePreparation::from_plan(
                plan.entry_trampoline(),
            ),
            initial_stack: GuiHelloWorldActualInitialStackPreparation::from_plan(
                plan.initial_stack(),
            ),
            helper_boundary: GuiHelloWorldActualHelperBoundaryPreparation::from_plan(
                plan.helper_boundary(),
            ),
            helper_capability: GuiHelloWorldActualHelperCapabilityPreparation::from_plan(
                plan.helper_capability(),
            ),
            bridge_boundary: GuiHelloWorldActualBridgeBoundaryPreparation::from_plan(
                plan.bridge_boundary(),
            ),
            integration_policy: GuiHelloWorldActualIntegrationPolicy::from_policy(
                plan.integration_policy(),
            ),
            process_boundary: GuiHelloWorldActualProcessBoundary::from_boundary(
                plan.process_boundary(),
            ),
            platform_model: GuiHelloWorldActualPlatformModel::from_plan(plan.platform_model()),
            macos_constraints: GuiHelloWorldActualMacosConstraints::from_constraints(
                plan.macos_constraints(),
            ),
            fallback_policy: GuiHelloWorldActualFallbackPolicy::from_policy(plan.fallback_policy()),
            loader_execution: GuiHelloWorldActualLoaderExecutionPreparation::from_plan(
                plan.loader_execution(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualRuntimePreparationSource {
    #[serde(rename = "bara_runtime_user_space_launch_plan")]
    BaraRuntimeUserSpaceLaunchPlan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualRuntimePreparationStatus {
    #[serde(rename = "planned_not_executed")]
    PlannedNotExecuted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualSourceIsaProfile {
    mode: GuiHelloWorldActualSourceIsaMode,
    address_size: GuiHelloWorldActualSourceWidth,
    default_operand_size: GuiHelloWorldActualSourceWidth,
    stack_width: GuiHelloWorldActualSourceWidth,
}

impl GuiHelloWorldActualSourceIsaProfile {
    const fn from_profile(profile: &UserSpaceSourceIsaProfile) -> Self {
        Self {
            mode: GuiHelloWorldActualSourceIsaMode::from_runtime((*profile).mode()),
            address_size: GuiHelloWorldActualSourceWidth::from_runtime((*profile).address_size()),
            default_operand_size: GuiHelloWorldActualSourceWidth::from_runtime(
                (*profile).default_operand_size(),
            ),
            stack_width: GuiHelloWorldActualSourceWidth::from_runtime((*profile).stack_width()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualSourceIsaMode {
    #[serde(rename = "x86_64_long_mode")]
    X8664LongMode,
    #[serde(rename = "x86_32_protected_mode")]
    X8632ProtectedMode,
}

impl GuiHelloWorldActualSourceIsaMode {
    const fn from_runtime(mode: UserSpaceSourceIsaMode) -> Self {
        match mode {
            UserSpaceSourceIsaMode::X8664LongMode => Self::X8664LongMode,
            UserSpaceSourceIsaMode::X8632ProtectedMode => Self::X8632ProtectedMode,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualSourceWidth {
    #[serde(rename = "bits_32")]
    Bits32,
    #[serde(rename = "bits_64")]
    Bits64,
}

impl GuiHelloWorldActualSourceWidth {
    const fn from_runtime(width: UserSpaceSourceWidth) -> Self {
        match width {
            UserSpaceSourceWidth::Bits32 => Self::Bits32,
            UserSpaceSourceWidth::Bits64 => Self::Bits64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualImageMappingPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    source: GuiHelloWorldActualImageMappingSource,
    memory_protection: GuiHelloWorldActualMemoryProtectionModel,
}

impl GuiHelloWorldActualImageMappingPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceImageMappingPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            source: GuiHelloWorldActualImageMappingSource::from_runtime(plan.source()),
            memory_protection: GuiHelloWorldActualMemoryProtectionModel::from_runtime(
                plan.memory_protection(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualExecutableMemoryPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    allocation_api: GuiHelloWorldActualExecutableMemoryAllocationApi,
    protection_transition: GuiHelloWorldActualExecutableMemoryProtectionTransition,
    release_api: GuiHelloWorldActualExecutableMemoryReleaseApi,
}

impl GuiHelloWorldActualExecutableMemoryPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceExecutableMemoryPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            allocation_api: GuiHelloWorldActualExecutableMemoryAllocationApi::from_runtime(
                plan.allocation_api(),
            ),
            protection_transition:
                GuiHelloWorldActualExecutableMemoryProtectionTransition::from_runtime(
                    plan.protection_transition(),
                ),
            release_api: GuiHelloWorldActualExecutableMemoryReleaseApi::from_runtime(
                plan.release_api(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualExecutionStrategyPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    boundary: GuiHelloWorldActualExecutionStrategyBoundary,
    jit: GuiHelloWorldActualExecutionStrategyAvailability,
    aot: GuiHelloWorldActualExecutionStrategyAvailability,
    fallback_interpreter: GuiHelloWorldActualExecutionStrategyAvailability,
}

impl GuiHelloWorldActualExecutionStrategyPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceExecutionStrategyPlan) -> Self {
        let strategies = plan.strategies();

        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            boundary: GuiHelloWorldActualExecutionStrategyBoundary::from_runtime(plan.boundary()),
            jit: GuiHelloWorldActualExecutionStrategyAvailability::from_runtime(strategies.jit()),
            aot: GuiHelloWorldActualExecutionStrategyAvailability::from_runtime(strategies.aot()),
            fallback_interpreter: GuiHelloWorldActualExecutionStrategyAvailability::from_runtime(
                strategies.fallback_interpreter(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualEntryTrampolinePreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    target: GuiHelloWorldActualEntryTrampolineTarget,
}

impl GuiHelloWorldActualEntryTrampolinePreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceEntryTrampolinePlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            target: GuiHelloWorldActualEntryTrampolineTarget::from_runtime(plan.target()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualInitialStackPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    contract: GuiHelloWorldActualInitialStackContract,
}

impl GuiHelloWorldActualInitialStackPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceInitialStackPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            contract: GuiHelloWorldActualInitialStackContract::from_runtime(plan.contract()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualHelperBoundaryPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    contract: GuiHelloWorldActualHelperBoundaryContract,
    public_import: GuiHelloWorldActualHelperBoundaryPublicImport,
    import_resolution: GuiHelloWorldActualHelperBoundaryResolution,
    objc_runtime: GuiHelloWorldActualHelperBoundaryResolution,
    os_api_requests: GuiHelloWorldActualHelperBoundaryResolution,
    next_blocker: GuiHelloWorldActualHelperBoundaryNextBlocker,
    status: GuiHelloWorldActualHelperBoundaryStatus,
}

impl GuiHelloWorldActualHelperBoundaryPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceHelperBoundaryPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            contract: GuiHelloWorldActualHelperBoundaryContract::from_runtime(plan.contract()),
            public_import: GuiHelloWorldActualHelperBoundaryPublicImport::from_runtime(
                plan.public_import(),
            ),
            import_resolution: GuiHelloWorldActualHelperBoundaryResolution::from_runtime(
                plan.import_resolution(),
            ),
            objc_runtime: GuiHelloWorldActualHelperBoundaryResolution::from_runtime(
                plan.objc_runtime(),
            ),
            os_api_requests: GuiHelloWorldActualHelperBoundaryResolution::from_runtime(
                plan.os_api_requests(),
            ),
            next_blocker: GuiHelloWorldActualHelperBoundaryNextBlocker::from_runtime(
                plan.next_blocker(),
            ),
            status: GuiHelloWorldActualHelperBoundaryStatus::from_runtime(plan.status()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualHelperCapabilityPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    contract: GuiHelloWorldActualHelperCapabilityContract,
    objc_runtime_bridge: GuiHelloWorldActualHelperCapabilityConnection,
    appkit_lifecycle_event: GuiHelloWorldActualHelperCapabilityConnection,
    observation: GuiHelloWorldActualHelperObservationContract,
    status: GuiHelloWorldActualHelperCapabilityStatus,
}

impl GuiHelloWorldActualHelperCapabilityPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceHelperCapabilityPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            contract: GuiHelloWorldActualHelperCapabilityContract::from_runtime(plan.contract()),
            objc_runtime_bridge: GuiHelloWorldActualHelperCapabilityConnection::from_runtime(
                plan.objc_runtime_bridge(),
            ),
            appkit_lifecycle_event: GuiHelloWorldActualHelperCapabilityConnection::from_runtime(
                plan.appkit_lifecycle_event(),
            ),
            observation: GuiHelloWorldActualHelperObservationContract::from_runtime(
                plan.observation(),
            ),
            status: GuiHelloWorldActualHelperCapabilityStatus::from_runtime(plan.status()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualLoaderExecutionPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    metadata_source: GuiHelloWorldActualLoaderExecutionMetadataSource,
    entry_point: GuiHelloWorldActualLoaderExecutionEntryPoint,
    segment_mapping: GuiHelloWorldActualLoaderExecutionSegmentMapping,
    imports: GuiHelloWorldActualLoaderExecutionImportPlan,
    relocations: GuiHelloWorldActualLoaderExecutionRelocationPlan,
    objc_runtime: GuiHelloWorldActualLoaderExecutionObjcRuntimePlan,
    status: GuiHelloWorldActualLoaderExecutionStatus,
}

impl GuiHelloWorldActualLoaderExecutionPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceLoaderExecutionPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            metadata_source: GuiHelloWorldActualLoaderExecutionMetadataSource::from_runtime(
                plan.metadata_source(),
            ),
            entry_point: GuiHelloWorldActualLoaderExecutionEntryPoint::from_runtime(
                plan.entry_point(),
            ),
            segment_mapping: GuiHelloWorldActualLoaderExecutionSegmentMapping::from_runtime(
                plan.segment_mapping(),
            ),
            imports: GuiHelloWorldActualLoaderExecutionImportPlan::from_runtime(plan.imports()),
            relocations: GuiHelloWorldActualLoaderExecutionRelocationPlan::from_runtime(
                plan.relocations(),
            ),
            objc_runtime: GuiHelloWorldActualLoaderExecutionObjcRuntimePlan::from_runtime(
                plan.objc_runtime(),
            ),
            status: GuiHelloWorldActualLoaderExecutionStatus::from_runtime(plan.status()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderExecutionMetadataSource {
    #[serde(rename = "public_mach_o_probe")]
    PublicMachOProbe,
}

impl GuiHelloWorldActualLoaderExecutionMetadataSource {
    const fn from_runtime(source: UserSpaceLoaderMetadataSource) -> Self {
        match source {
            UserSpaceLoaderMetadataSource::PublicMachOProbe => Self::PublicMachOProbe,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderExecutionEntryPoint {
    #[serde(rename = "lc_main_entryoff")]
    LcMainEntryoff,
}

impl GuiHelloWorldActualLoaderExecutionEntryPoint {
    const fn from_runtime(entry_point: UserSpaceLoaderEntryPointPlan) -> Self {
        match entry_point {
            UserSpaceLoaderEntryPointPlan::LcMainEntryoff => Self::LcMainEntryoff,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderExecutionSegmentMapping {
    #[serde(rename = "lc_segment_64_file_ranges")]
    LcSegment64FileRanges,
}

impl GuiHelloWorldActualLoaderExecutionSegmentMapping {
    const fn from_runtime(segment_mapping: UserSpaceLoaderSegmentMappingPlan) -> Self {
        match segment_mapping {
            UserSpaceLoaderSegmentMappingPlan::LcSegment64FileRanges => Self::LcSegment64FileRanges,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderExecutionImportPlan {
    #[serde(rename = "dylib_load_commands_to_helper_boundary")]
    DylibLoadCommandsToHelperBoundary,
}

impl GuiHelloWorldActualLoaderExecutionImportPlan {
    const fn from_runtime(imports: UserSpaceLoaderImportPlan) -> Self {
        match imports {
            UserSpaceLoaderImportPlan::DylibLoadCommandsToHelperBoundary => {
                Self::DylibLoadCommandsToHelperBoundary
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderExecutionRelocationPlan {
    #[serde(rename = "linkedit_rebase_bind_metadata")]
    LinkeditRebaseBindMetadata,
}

impl GuiHelloWorldActualLoaderExecutionRelocationPlan {
    const fn from_runtime(relocations: UserSpaceLoaderRelocationPlan) -> Self {
        match relocations {
            UserSpaceLoaderRelocationPlan::LinkeditRebaseBindMetadata => {
                Self::LinkeditRebaseBindMetadata
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderExecutionObjcRuntimePlan {
    #[serde(rename = "helper_boundary")]
    HelperBoundary,
}

impl GuiHelloWorldActualLoaderExecutionObjcRuntimePlan {
    const fn from_runtime(objc_runtime: UserSpaceLoaderObjcRuntimePlan) -> Self {
        match objc_runtime {
            UserSpaceLoaderObjcRuntimePlan::HelperBoundary => Self::HelperBoundary,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderExecutionStatus {
    #[serde(rename = "planned_not_executed")]
    PlannedNotExecuted,
}

impl GuiHelloWorldActualLoaderExecutionStatus {
    const fn from_runtime(status: UserSpaceLoaderExecutionStatus) -> Self {
        match status {
            UserSpaceLoaderExecutionStatus::PlannedNotExecuted => Self::PlannedNotExecuted,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualRuntimePreparationResponsibility {
    #[serde(rename = "helper_boundary")]
    HelperBoundary,
    #[serde(rename = "loader")]
    Loader,
    #[serde(rename = "runtime")]
    Runtime,
}

impl GuiHelloWorldActualRuntimePreparationResponsibility {
    const fn from_runtime(responsibility: UserSpaceLaunchResponsibility) -> Self {
        match responsibility {
            UserSpaceLaunchResponsibility::HelperBoundary => Self::HelperBoundary,
            UserSpaceLaunchResponsibility::Loader => Self::Loader,
            UserSpaceLaunchResponsibility::Runtime => Self::Runtime,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualImageMappingSource {
    #[serde(rename = "mach_o_executable_image")]
    MachOExecutableImage,
}

impl GuiHelloWorldActualImageMappingSource {
    const fn from_runtime(source: UserSpaceImageMappingSource) -> Self {
        match source {
            UserSpaceImageMappingSource::MachOExecutableImage => Self::MachOExecutableImage,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualMemoryProtectionModel {
    #[serde(rename = "public_os_virtual_memory")]
    PublicOsVirtualMemory,
}

impl GuiHelloWorldActualMemoryProtectionModel {
    const fn from_runtime(model: UserSpaceMemoryProtectionModel) -> Self {
        match model {
            UserSpaceMemoryProtectionModel::PublicOsVirtualMemory => Self::PublicOsVirtualMemory,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualExecutableMemoryAllocationApi {
    #[serde(rename = "mmap_private_anonymous")]
    MmapPrivateAnonymous,
}

impl GuiHelloWorldActualExecutableMemoryAllocationApi {
    const fn from_runtime(api: UserSpaceExecutableMemoryAllocationApi) -> Self {
        match api {
            UserSpaceExecutableMemoryAllocationApi::MmapPrivateAnonymous => {
                Self::MmapPrivateAnonymous
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualExecutableMemoryProtectionTransition {
    #[serde(rename = "mprotect_read_write_to_read_execute")]
    MprotectReadWriteToReadExecute,
}

impl GuiHelloWorldActualExecutableMemoryProtectionTransition {
    const fn from_runtime(api: UserSpaceExecutableMemoryProtectionTransition) -> Self {
        match api {
            UserSpaceExecutableMemoryProtectionTransition::MprotectReadWriteToReadExecute => {
                Self::MprotectReadWriteToReadExecute
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualExecutableMemoryReleaseApi {
    #[serde(rename = "munmap")]
    Munmap,
}

impl GuiHelloWorldActualExecutableMemoryReleaseApi {
    const fn from_runtime(api: UserSpaceExecutableMemoryReleaseApi) -> Self {
        match api {
            UserSpaceExecutableMemoryReleaseApi::Munmap => Self::Munmap,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualExecutionStrategyBoundary {
    #[serde(rename = "user_space_runtime")]
    UserSpaceRuntime,
}

impl GuiHelloWorldActualExecutionStrategyBoundary {
    const fn from_runtime(boundary: UserSpaceExecutionStrategyBoundary) -> Self {
        match boundary {
            UserSpaceExecutionStrategyBoundary::UserSpaceRuntime => Self::UserSpaceRuntime,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualExecutionStrategyAvailability {
    #[serde(rename = "selectable")]
    Selectable,
}

impl GuiHelloWorldActualExecutionStrategyAvailability {
    const fn from_runtime(availability: UserSpaceExecutionStrategyAvailability) -> Self {
        match availability {
            UserSpaceExecutionStrategyAvailability::Selectable => Self::Selectable,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualEntryTrampolineTarget {
    #[serde(rename = "mach_o_entry_point")]
    MachOEntryPoint,
}

impl GuiHelloWorldActualEntryTrampolineTarget {
    const fn from_runtime(target: UserSpaceEntryTrampolineTarget) -> Self {
        match target {
            UserSpaceEntryTrampolineTarget::MachOEntryPoint => Self::MachOEntryPoint,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualInitialStackContract {
    #[serde(rename = "argv_envp_initial_stack")]
    ArgvEnvpInitialStack,
}

impl GuiHelloWorldActualInitialStackContract {
    const fn from_runtime(contract: UserSpaceInitialStackContract) -> Self {
        match contract {
            UserSpaceInitialStackContract::ArgvEnvpInitialStack => Self::ArgvEnvpInitialStack,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperBoundaryContract {
    #[serde(rename = "imports_objc_os_api_requests")]
    ImportsObjcOsApiRequests,
}

impl GuiHelloWorldActualHelperBoundaryContract {
    const fn from_runtime(contract: UserSpaceHelperBoundaryContract) -> Self {
        match contract {
            UserSpaceHelperBoundaryContract::ImportsObjcOsApiRequests => {
                Self::ImportsObjcOsApiRequests
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperBoundaryPublicImport {
    #[serde(rename = "appkit_framework")]
    AppKitFramework,
}

impl GuiHelloWorldActualHelperBoundaryPublicImport {
    const fn from_runtime(public_import: UserSpaceHelperBoundaryPublicImport) -> Self {
        match public_import {
            UserSpaceHelperBoundaryPublicImport::AppKitFramework => Self::AppKitFramework,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperBoundaryResolution {
    #[serde(rename = "helper_capability_required")]
    HelperCapabilityRequired,
}

impl GuiHelloWorldActualHelperBoundaryResolution {
    const fn from_runtime(resolution: UserSpaceHelperBoundaryResolution) -> Self {
        match resolution {
            UserSpaceHelperBoundaryResolution::HelperCapabilityRequired => {
                Self::HelperCapabilityRequired
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperBoundaryNextBlocker {
    #[serde(rename = "unsupported_import")]
    UnsupportedImport,
    #[serde(rename = "unsupported_objc_runtime_boundary")]
    UnsupportedObjcRuntimeBoundary,
}

impl GuiHelloWorldActualHelperBoundaryNextBlocker {
    const fn from_runtime(next_blocker: UserSpaceHelperBoundaryNextBlocker) -> Self {
        match next_blocker {
            UserSpaceHelperBoundaryNextBlocker::UnsupportedImport => Self::UnsupportedImport,
            UserSpaceHelperBoundaryNextBlocker::UnsupportedObjcRuntimeBoundary => {
                Self::UnsupportedObjcRuntimeBoundary
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperBoundaryStatus {
    #[serde(rename = "planned_not_executed")]
    PlannedNotExecuted,
}

impl GuiHelloWorldActualHelperBoundaryStatus {
    const fn from_runtime(status: UserSpaceHelperBoundaryStatus) -> Self {
        match status {
            UserSpaceHelperBoundaryStatus::PlannedNotExecuted => Self::PlannedNotExecuted,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperCapabilityContract {
    #[serde(rename = "appkit_gui_lifecycle_event")]
    AppKitGuiLifecycleEvent,
}

impl GuiHelloWorldActualHelperCapabilityContract {
    const fn from_runtime(contract: UserSpaceHelperCapabilityContract) -> Self {
        match contract {
            UserSpaceHelperCapabilityContract::AppKitGuiLifecycleEvent => {
                Self::AppKitGuiLifecycleEvent
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperCapabilityConnection {
    #[serde(rename = "planned")]
    Planned,
}

impl GuiHelloWorldActualHelperCapabilityConnection {
    const fn from_runtime(connection: UserSpaceHelperCapabilityConnection) -> Self {
        match connection {
            UserSpaceHelperCapabilityConnection::Planned => Self::Planned,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperObservationContract {
    #[serde(rename = "stdout_lifecycle_event")]
    StdoutLifecycleEvent,
}

impl GuiHelloWorldActualHelperObservationContract {
    const fn from_runtime(contract: UserSpaceHelperObservationContract) -> Self {
        match contract {
            UserSpaceHelperObservationContract::StdoutLifecycleEvent => Self::StdoutLifecycleEvent,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualHelperCapabilityStatus {
    #[serde(rename = "planned_not_executed")]
    PlannedNotExecuted,
}

impl GuiHelloWorldActualHelperCapabilityStatus {
    const fn from_runtime(status: UserSpaceHelperCapabilityStatus) -> Self {
        match status {
            UserSpaceHelperCapabilityStatus::PlannedNotExecuted => Self::PlannedNotExecuted,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualBridgeBoundaryPreparation {
    responsibility: GuiHelloWorldActualRuntimePreparationResponsibility,
    syscall_bridge: GuiHelloWorldActualBridgeBoundaryPlacement,
    os_api_bridge: GuiHelloWorldActualBridgeBoundaryPlacement,
    core_ir_implementation: GuiHelloWorldActualBridgeCoreImplementation,
    arm64_emit_implementation: GuiHelloWorldActualBridgeCoreImplementation,
}

impl GuiHelloWorldActualBridgeBoundaryPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceBridgeBoundaryPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            syscall_bridge: GuiHelloWorldActualBridgeBoundaryPlacement::from_runtime(
                plan.syscall_bridge(),
            ),
            os_api_bridge: GuiHelloWorldActualBridgeBoundaryPlacement::from_runtime(
                plan.os_api_bridge(),
            ),
            core_ir_implementation: GuiHelloWorldActualBridgeCoreImplementation::from_runtime(
                plan.core_ir_implementation(),
            ),
            arm64_emit_implementation: GuiHelloWorldActualBridgeCoreImplementation::from_runtime(
                plan.arm64_emit_implementation(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualIntegrationPolicy {
    process_scope: GuiHelloWorldActualProcessScope,
    kernel_extension: GuiHelloWorldActualPrivateIntegrationRequirement,
    private_kernel_hook: GuiHelloWorldActualPrivateIntegrationRequirement,
    private_dyld_behavior: GuiHelloWorldActualPrivateIntegrationRequirement,
}

impl GuiHelloWorldActualIntegrationPolicy {
    const fn from_policy(policy: &bara_runtime::UserSpaceIntegrationPolicy) -> Self {
        Self {
            process_scope: GuiHelloWorldActualProcessScope::from_runtime(policy.process_scope()),
            kernel_extension: GuiHelloWorldActualPrivateIntegrationRequirement::from_runtime(
                policy.kernel_extension(),
            ),
            private_kernel_hook: GuiHelloWorldActualPrivateIntegrationRequirement::from_runtime(
                policy.private_kernel_hook(),
            ),
            private_dyld_behavior: GuiHelloWorldActualPrivateIntegrationRequirement::from_runtime(
                policy.private_dyld_behavior(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualProcessScope {
    #[serde(rename = "current_user_space_process")]
    CurrentUserSpaceProcess,
}

impl GuiHelloWorldActualProcessScope {
    const fn from_runtime(scope: UserSpaceProcessScope) -> Self {
        match scope {
            UserSpaceProcessScope::CurrentUserSpaceProcess => Self::CurrentUserSpaceProcess,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualPrivateIntegrationRequirement {
    #[serde(rename = "not_required")]
    NotRequired,
}

impl GuiHelloWorldActualPrivateIntegrationRequirement {
    const fn from_runtime(requirement: UserSpacePrivateIntegrationRequirement) -> Self {
        match requirement {
            UserSpacePrivateIntegrationRequirement::NotRequired => Self::NotRequired,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBridgeBoundaryPlacement {
    #[serde(rename = "helper_boundary")]
    HelperBoundary,
}

impl GuiHelloWorldActualBridgeBoundaryPlacement {
    const fn from_runtime(placement: UserSpaceBridgeBoundaryPlacement) -> Self {
        match placement {
            UserSpaceBridgeBoundaryPlacement::HelperBoundary => Self::HelperBoundary,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBridgeCoreImplementation {
    #[serde(rename = "not_embedded")]
    NotEmbedded,
}

impl GuiHelloWorldActualBridgeCoreImplementation {
    const fn from_runtime(implementation: UserSpaceBridgeCoreImplementation) -> Self {
        match implementation {
            UserSpaceBridgeCoreImplementation::NotEmbedded => Self::NotEmbedded,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualProcessBoundary {
    loader: GuiHelloWorldActualProcessScope,
    translation_cache: GuiHelloWorldActualProcessScope,
    runtime_helper: GuiHelloWorldActualProcessScope,
    artifact_cache: GuiHelloWorldActualProcessScope,
}

impl GuiHelloWorldActualProcessBoundary {
    const fn from_boundary(boundary: &bara_runtime::UserSpaceProcessBoundary) -> Self {
        Self {
            loader: GuiHelloWorldActualProcessScope::from_runtime(boundary.loader()),
            translation_cache: GuiHelloWorldActualProcessScope::from_runtime(
                boundary.translation_cache(),
            ),
            runtime_helper: GuiHelloWorldActualProcessScope::from_runtime(
                boundary.runtime_helper(),
            ),
            artifact_cache: GuiHelloWorldActualProcessScope::from_runtime(
                boundary.artifact_cache(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualPlatformModel {
    signal_model: GuiHelloWorldActualPlatformSignalModel,
    exception_model: GuiHelloWorldActualPlatformExceptionModel,
    thread_model: GuiHelloWorldActualPlatformThreadModel,
    tls_model: GuiHelloWorldActualPlatformTlsModel,
    memory_protection: GuiHelloWorldActualPlatformMemoryProtectionModel,
}

impl GuiHelloWorldActualPlatformModel {
    const fn from_plan(plan: &bara_runtime::UserSpacePlatformModelPlan) -> Self {
        Self {
            signal_model: GuiHelloWorldActualPlatformSignalModel::from_runtime(plan.signal_model()),
            exception_model: GuiHelloWorldActualPlatformExceptionModel::from_runtime(
                plan.exception_model(),
            ),
            thread_model: GuiHelloWorldActualPlatformThreadModel::from_runtime(plan.thread_model()),
            tls_model: GuiHelloWorldActualPlatformTlsModel::from_runtime(plan.tls_model()),
            memory_protection: GuiHelloWorldActualPlatformMemoryProtectionModel::from_runtime(
                plan.memory_protection(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualPlatformSignalModel {
    #[serde(rename = "user_space_loader_boundary")]
    UserSpaceLoaderBoundary,
}

impl GuiHelloWorldActualPlatformSignalModel {
    const fn from_runtime(model: UserSpacePlatformSignalModel) -> Self {
        match model {
            UserSpacePlatformSignalModel::UserSpaceLoaderBoundary => Self::UserSpaceLoaderBoundary,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualPlatformExceptionModel {
    #[serde(rename = "user_space_loader_boundary")]
    UserSpaceLoaderBoundary,
}

impl GuiHelloWorldActualPlatformExceptionModel {
    const fn from_runtime(model: UserSpacePlatformExceptionModel) -> Self {
        match model {
            UserSpacePlatformExceptionModel::UserSpaceLoaderBoundary => {
                Self::UserSpaceLoaderBoundary
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualPlatformThreadModel {
    #[serde(rename = "initial_thread_only")]
    InitialThreadOnly,
}

impl GuiHelloWorldActualPlatformThreadModel {
    const fn from_runtime(model: UserSpacePlatformThreadModel) -> Self {
        match model {
            UserSpacePlatformThreadModel::InitialThreadOnly => Self::InitialThreadOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualPlatformTlsModel {
    #[serde(rename = "deferred")]
    Deferred,
}

impl GuiHelloWorldActualPlatformTlsModel {
    const fn from_runtime(model: UserSpacePlatformTlsModel) -> Self {
        match model {
            UserSpacePlatformTlsModel::Deferred => Self::Deferred,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualPlatformMemoryProtectionModel {
    #[serde(rename = "public_os_virtual_memory")]
    PublicOsVirtualMemory,
}

impl GuiHelloWorldActualPlatformMemoryProtectionModel {
    const fn from_runtime(model: UserSpacePlatformMemoryProtectionModel) -> Self {
        match model {
            UserSpacePlatformMemoryProtectionModel::PublicOsVirtualMemory => {
                Self::PublicOsVirtualMemory
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualMacosConstraints {
    code_signing: GuiHelloWorldActualMacosCodeSigningPolicy,
    write_xor_execute: GuiHelloWorldActualMacosWriteXorExecutePolicy,
    hardened_runtime: GuiHelloWorldActualMacosHardenedRuntimePolicy,
}

impl GuiHelloWorldActualMacosConstraints {
    const fn from_constraints(constraints: &bara_runtime::UserSpaceMacosConstraints) -> Self {
        Self {
            code_signing: GuiHelloWorldActualMacosCodeSigningPolicy::from_runtime(
                constraints.code_signing(),
            ),
            write_xor_execute: GuiHelloWorldActualMacosWriteXorExecutePolicy::from_runtime(
                constraints.write_xor_execute(),
            ),
            hardened_runtime: GuiHelloWorldActualMacosHardenedRuntimePolicy::from_runtime(
                constraints.hardened_runtime(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualMacosCodeSigningPolicy {
    #[serde(rename = "no_private_signing_bypass")]
    NoPrivateSigningBypass,
}

impl GuiHelloWorldActualMacosCodeSigningPolicy {
    const fn from_runtime(policy: UserSpaceMacosCodeSigningPolicy) -> Self {
        match policy {
            UserSpaceMacosCodeSigningPolicy::NoPrivateSigningBypass => Self::NoPrivateSigningBypass,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualMacosWriteXorExecutePolicy {
    #[serde(rename = "public_mmap_mprotect_transition")]
    PublicMmapMprotectTransition,
}

impl GuiHelloWorldActualMacosWriteXorExecutePolicy {
    const fn from_runtime(policy: UserSpaceMacosWriteXorExecutePolicy) -> Self {
        match policy {
            UserSpaceMacosWriteXorExecutePolicy::PublicMmapMprotectTransition => {
                Self::PublicMmapMprotectTransition
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualMacosHardenedRuntimePolicy {
    #[serde(rename = "documented_host_policy_only")]
    DocumentedHostPolicyOnly,
}

impl GuiHelloWorldActualMacosHardenedRuntimePolicy {
    const fn from_runtime(policy: UserSpaceMacosHardenedRuntimePolicy) -> Self {
        match policy {
            UserSpaceMacosHardenedRuntimePolicy::DocumentedHostPolicyOnly => {
                Self::DocumentedHostPolicyOnly
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualFallbackPolicy {
    unimplemented_instruction: GuiHelloWorldActualFallbackPolicyAction,
    unknown_indirect_target: GuiHelloWorldActualFallbackPolicyAction,
    unsupported_loader_feature: GuiHelloWorldActualFallbackPolicyAction,
    interpreter: GuiHelloWorldActualFallbackEngineStatus,
    external_engine: GuiHelloWorldActualFallbackEngineStatus,
    feedback_cycle: GuiHelloWorldActualFeedbackCycleState,
}

impl GuiHelloWorldActualFallbackPolicy {
    const fn from_policy(policy: &bara_runtime::UserSpaceFallbackPolicy) -> Self {
        Self {
            unimplemented_instruction: GuiHelloWorldActualFallbackPolicyAction::from_runtime(
                policy.unimplemented_instruction(),
            ),
            unknown_indirect_target: GuiHelloWorldActualFallbackPolicyAction::from_runtime(
                policy.unknown_indirect_target(),
            ),
            unsupported_loader_feature: GuiHelloWorldActualFallbackPolicyAction::from_runtime(
                policy.unsupported_loader_feature(),
            ),
            interpreter: GuiHelloWorldActualFallbackEngineStatus::from_runtime(
                policy.interpreter(),
            ),
            external_engine: GuiHelloWorldActualFallbackEngineStatus::from_runtime(
                policy.external_engine(),
            ),
            feedback_cycle: GuiHelloWorldActualFeedbackCycleState::from_runtime(
                policy.feedback_cycle(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualFallbackPolicyAction {
    #[serde(rename = "stable_blocker_classification")]
    StableBlockerClassification,
}

impl GuiHelloWorldActualFallbackPolicyAction {
    const fn from_runtime(action: UserSpaceFallbackPolicyAction) -> Self {
        match action {
            UserSpaceFallbackPolicyAction::StableBlockerClassification => {
                Self::StableBlockerClassification
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualFallbackEngineStatus {
    #[serde(rename = "candidate_not_implemented")]
    CandidateNotImplemented,
    #[serde(rename = "candidate_not_connected")]
    CandidateNotConnected,
}

impl GuiHelloWorldActualFallbackEngineStatus {
    const fn from_runtime(status: UserSpaceFallbackEngineStatus) -> Self {
        match status {
            UserSpaceFallbackEngineStatus::CandidateNotImplemented => Self::CandidateNotImplemented,
            UserSpaceFallbackEngineStatus::CandidateNotConnected => Self::CandidateNotConnected,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualFeedbackCycleState {
    #[serde(rename = "ready_not_started")]
    ReadyNotStarted,
}

impl GuiHelloWorldActualFeedbackCycleState {
    const fn from_runtime(state: UserSpaceFeedbackCycleState) -> Self {
        match state {
            UserSpaceFeedbackCycleState::ReadyNotStarted => Self::ReadyNotStarted,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualInput {
    kind: GuiHelloWorldActualInputKind,
    source_isa: GuiHelloWorldActualSourceIsa,
    binary_format: GuiHelloWorldActualBinaryFormat,
    target_triple: GuiHelloWorldActualTargetTriple,
    gui_framework: GuiHelloWorldActualFramework,
    probe: GuiHelloWorldActualInputProbe,
    loader_metadata: GuiHelloWorldActualLoaderMetadata,
}

impl GuiHelloWorldActualInput {
    fn from_metadata(metadata: GuiHelloWorldActualInputMetadata) -> Self {
        Self {
            kind: GuiHelloWorldActualInputKind::MachOExecutableImage,
            source_isa: GuiHelloWorldActualSourceIsa::X8664,
            binary_format: GuiHelloWorldActualBinaryFormat::MachO,
            target_triple: GuiHelloWorldActualTargetTriple::X8664AppleMacos13,
            gui_framework: GuiHelloWorldActualFramework::AppKit,
            probe: metadata.probe,
            loader_metadata: metadata.loader_metadata,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualInputKind {
    #[serde(rename = "mach_o_executable_image")]
    MachOExecutableImage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualSourceIsa {
    #[serde(rename = "x86_64")]
    X8664,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBinaryFormat {
    #[serde(rename = "mach_o")]
    MachO,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualTargetTriple {
    #[serde(rename = "x86_64-apple-macos13")]
    X8664AppleMacos13,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualFramework {
    #[serde(rename = "appkit")]
    AppKit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualInputProbe {
    format: BinaryFormat,
    status: BinaryFormatProbeStatus,
}

impl GuiHelloWorldActualInputProbe {
    const fn from_report(report: &BinaryFormatProbeReport) -> Self {
        Self {
            format: report.format(),
            status: report.status(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualInputMetadata {
    probe: GuiHelloWorldActualInputProbe,
    loader_metadata: GuiHelloWorldActualLoaderMetadata,
}

impl GuiHelloWorldActualInputMetadata {
    fn from_report(report: &BinaryFormatProbeReport) -> Self {
        Self {
            probe: GuiHelloWorldActualInputProbe::from_report(report),
            loader_metadata: GuiHelloWorldActualLoaderMetadata::from_report(report),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualLoaderMetadata {
    source: GuiHelloWorldActualLoaderMetadataSource,
    mach_o: MachOMetadata,
    sections: GuiHelloWorldActualDeferredLoaderMetadata,
    imports: GuiHelloWorldActualDeferredLoaderMetadata,
    relocations: GuiHelloWorldActualDeferredLoaderMetadata,
}

impl GuiHelloWorldActualLoaderMetadata {
    fn from_report(report: &BinaryFormatProbeReport) -> Self {
        Self {
            source: GuiHelloWorldActualLoaderMetadataSource::PublicMachOProbe,
            mach_o: report.metadata().mach_o_metadata().clone(),
            sections:
                GuiHelloWorldActualDeferredLoaderMetadata::modeled_from_lc_segment_64_section_table(
                ),
            imports: GuiHelloWorldActualDeferredLoaderMetadata::modeled_from_dylib_load_commands(),
            relocations:
                GuiHelloWorldActualDeferredLoaderMetadata::modeled_from_linkedit_relocation_and_bind_commands(
                ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderMetadataSource {
    #[serde(rename = "public_mach_o_probe")]
    PublicMachOProbe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualDeferredLoaderMetadata {
    status: GuiHelloWorldActualDeferredLoaderMetadataStatus,
}

impl GuiHelloWorldActualDeferredLoaderMetadata {
    const fn modeled_from_lc_segment_64_section_table() -> Self {
        Self {
            status:
                GuiHelloWorldActualDeferredLoaderMetadataStatus::ModeledFromLcSegment64SectionTable,
        }
    }

    const fn modeled_from_dylib_load_commands() -> Self {
        Self {
            status: GuiHelloWorldActualDeferredLoaderMetadataStatus::ModeledFromDylibLoadCommands,
        }
    }

    const fn modeled_from_linkedit_relocation_and_bind_commands() -> Self {
        Self {
            status:
                GuiHelloWorldActualDeferredLoaderMetadataStatus::ModeledFromLinkeditRelocationAndBindCommands,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualDeferredLoaderMetadataStatus {
    #[serde(rename = "modeled_from_lc_segment_64_section_table")]
    ModeledFromLcSegment64SectionTable,
    #[serde(rename = "modeled_from_dylib_load_commands")]
    ModeledFromDylibLoadCommands,
    #[serde(rename = "modeled_from_linkedit_relocation_and_bind_commands")]
    ModeledFromLinkeditRelocationAndBindCommands,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualBlocker {
    classification: GuiHelloWorldActualBlockerClassification,
    boundary: GuiHelloWorldUnsupportedLaunchBoundary,
    selected_by: GuiHelloWorldActualBlockerSelectionRule,
    candidate_boundaries: Vec<GuiHelloWorldActualBlockerCandidate>,
    message: &'static str,
}

impl GuiHelloWorldActualBlocker {
    fn from_classification_plan(plan: &GuiHelloWorldInitialBlockerPlan) -> Self {
        let boundary = plan.selected_boundary();
        Self {
            classification: boundary.classification(),
            boundary,
            selected_by: GuiHelloWorldActualBlockerSelectionRule::FirstUnsupportedLaunchBoundary,
            candidate_boundaries: plan.candidate_boundaries(),
            message: boundary.message(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBlockerClassification {
    #[serde(rename = "unsupported_objc_runtime_boundary")]
    ObjcRuntimeBoundary,
}

impl GuiHelloWorldActualBlockerClassification {
    const fn stderr_message(self) -> &'static str {
        match self {
            Self::ObjcRuntimeBoundary => "unsupported_boundary: unsupported_objc_runtime_boundary",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldUnsupportedLaunchBoundary {
    #[serde(rename = "objc_runtime")]
    ObjcRuntime,
}

impl GuiHelloWorldUnsupportedLaunchBoundary {
    const fn classification(self) -> GuiHelloWorldActualBlockerClassification {
        match self {
            Self::ObjcRuntime => GuiHelloWorldActualBlockerClassification::ObjcRuntimeBoundary,
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::ObjcRuntime => {
                "Bara does not yet provide an Objective-C runtime helper boundary for the AppKit GUI fixture."
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBlockerSelectionRule {
    #[serde(rename = "first_unsupported_launch_boundary")]
    FirstUnsupportedLaunchBoundary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualBlockerCandidate {
    boundary: GuiHelloWorldUnsupportedLaunchBoundary,
    classification: GuiHelloWorldActualBlockerClassification,
}

impl GuiHelloWorldActualBlockerCandidate {
    const fn from_boundary(boundary: GuiHelloWorldUnsupportedLaunchBoundary) -> Self {
        Self {
            boundary,
            classification: boundary.classification(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GuiHelloWorldInitialBlockerPlan {
    unsupported_boundaries: NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries,
}

impl GuiHelloWorldInitialBlockerPlan {
    fn current() -> Self {
        Self {
            unsupported_boundaries: NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries::new(
                GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime,
                Vec::new(),
            ),
        }
    }

    const fn selected_boundary(&self) -> GuiHelloWorldUnsupportedLaunchBoundary {
        self.unsupported_boundaries.first()
    }

    const fn selected_classification(&self) -> GuiHelloWorldActualBlockerClassification {
        self.selected_boundary().classification()
    }

    fn candidate_boundaries(&self) -> Vec<GuiHelloWorldActualBlockerCandidate> {
        self.unsupported_boundaries
            .to_vec()
            .into_iter()
            .map(GuiHelloWorldActualBlockerCandidate::from_boundary)
            .collect()
    }

    #[cfg(test)]
    fn with_first_boundary(first: GuiHelloWorldUnsupportedLaunchBoundary) -> Self {
        Self {
            unsupported_boundaries: NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries::new(
                first,
                Vec::new(),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries {
    first: GuiHelloWorldUnsupportedLaunchBoundary,
    remaining: Vec<GuiHelloWorldUnsupportedLaunchBoundary>,
}

impl NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries {
    const fn new(
        first: GuiHelloWorldUnsupportedLaunchBoundary,
        remaining: Vec<GuiHelloWorldUnsupportedLaunchBoundary>,
    ) -> Self {
        Self { first, remaining }
    }

    const fn first(&self) -> GuiHelloWorldUnsupportedLaunchBoundary {
        self.first
    }

    fn to_vec(&self) -> Vec<GuiHelloWorldUnsupportedLaunchBoundary> {
        let mut boundaries = vec![self.first];
        boundaries.extend(self.remaining.iter().copied());
        boundaries
    }
}

pub(crate) fn b8_gui_hello_world_actual_launch_attempt(
    input_probe_report: &BinaryFormatProbeReport,
) -> Result<GuiHelloWorldActualLaunchBundle, X8664MachOFixtureError> {
    Ok(GuiHelloWorldActualLaunchBundle::blocked_by_initial_blocker(
        b8_gui_hello_world_case_id()?,
        GuiHelloWorldActualInputMetadata::from_report(input_probe_report),
        GuiHelloWorldInitialBlockerPlan::current(),
    ))
}

pub(crate) fn b8_gui_hello_world_feedback_report(
    expected: &ObservedResult,
    actual: &GuiHelloWorldActualLaunchBundle,
) -> GuiHelloWorldFeedbackReport {
    GuiHelloWorldFeedbackReport::from_expected_and_actual(expected, actual)
}

#[cfg(test)]
mod tests {
    use bara_oracle::{probe_public_binary_format, BinaryInput, CaseId, ObservedResult};

    use super::{
        b8_gui_hello_world_actual_launch_attempt, b8_gui_hello_world_feedback_report,
        GuiHelloWorldActualBlockerClassification, GuiHelloWorldInitialBlockerPlan,
        GuiHelloWorldUnsupportedLaunchBoundary,
    };

    #[test]
    fn gui_hello_world_actual_attempt_reports_objc_runtime_blocker() {
        let probe_report = mach_o_execute_header_probe();
        let attempt = b8_gui_hello_world_actual_launch_attempt(&probe_report)
            .expect("built-in B8 GUI Hello World case id is valid");

        assert_eq!(
            attempt.observed_result(),
            &ObservedResult::new(
                CaseId::new("b8_gui_hello_world").expect("case id is non-empty"),
                1,
                0,
                String::new(),
                String::from("unsupported_boundary: unsupported_objc_runtime_boundary"),
            )
        );
        assert_eq!(
            serde_json::to_string(attempt.launch_report()).expect("launch report serializes"),
            include_str!("../../../tests/expected/b8_gui_hello_world.bara.launch-report.json")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn gui_hello_world_feedback_report_keeps_comparison_and_current_blocker() {
        let probe_report = mach_o_execute_header_probe();
        let attempt = b8_gui_hello_world_actual_launch_attempt(&probe_report)
            .expect("built-in B8 GUI Hello World case id is valid");
        let expected = ObservedResult::new(
            CaseId::new("b8_gui_hello_world").expect("case id is non-empty"),
            0,
            0,
            "{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}\n".to_owned(),
            String::new(),
        );

        let feedback = b8_gui_hello_world_feedback_report(&expected, &attempt);

        assert_eq!(
            serde_json::to_string(&feedback).expect("feedback report serializes"),
            include_str!("../../../tests/expected/b8_gui_hello_world.bara.feedback-report.json")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn initial_blocker_plan_promotes_helper_boundary_objc_runtime() {
        let plan = GuiHelloWorldInitialBlockerPlan::current();

        assert_eq!(
            plan.selected_classification(),
            GuiHelloWorldActualBlockerClassification::ObjcRuntimeBoundary
        );
        assert_eq!(
            plan.candidate_boundaries(),
            vec![super::GuiHelloWorldActualBlockerCandidate::from_boundary(
                GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime
            ),]
        );
    }

    #[test]
    fn initial_blocker_plan_has_stable_objc_runtime_classification() {
        let objc_runtime_plan = GuiHelloWorldInitialBlockerPlan::with_first_boundary(
            GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime,
        );

        assert_eq!(
            objc_runtime_plan.selected_classification(),
            GuiHelloWorldActualBlockerClassification::ObjcRuntimeBoundary
        );
    }

    fn mach_o_execute_header_probe() -> bara_oracle::BinaryFormatProbeReport {
        let input = BinaryInput::from_hex(
            "cffaedfe07000001030000000200000000000000000000000000000000000000",
        )
        .expect("minimal Mach-O executable header hex parses");
        probe_public_binary_format(&input).expect("minimal Mach-O executable header probes")
    }
}
