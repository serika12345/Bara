use bara_oracle::{
    BinaryFormat, BinaryFormatProbeReport, BinaryFormatProbeStatus, CaseId, MachOMetadata,
    ObservedResult,
};
use bara_runtime::{
    UserSpaceBridgeBoundaryPlacement, UserSpaceBridgeCoreImplementation,
    UserSpaceEntryTrampolineTarget, UserSpaceExecutableMemoryAllocationApi,
    UserSpaceExecutableMemoryProtectionTransition, UserSpaceExecutableMemoryReleaseApi,
    UserSpaceExecutionStrategyAvailability, UserSpaceExecutionStrategyBoundary,
    UserSpaceHelperBoundaryContract, UserSpaceImageMappingSource, UserSpaceInitialStackContract,
    UserSpaceLaunchPlan, UserSpaceLaunchResponsibility, UserSpaceMemoryProtectionModel,
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
        let observed_result = ObservedResult::new(
            case_id.clone(),
            1,
            0,
            String::new(),
            String::from(classification.stderr_message()),
        );
        let launch_report = GuiHelloWorldActualLaunchReport::blocked_by_initial_blocker(
            case_id,
            input_metadata,
            classification_plan,
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
    blocker: GuiHelloWorldActualBlocker,
}

impl GuiHelloWorldActualLaunchReport {
    fn blocked_by_initial_blocker(
        case_id: CaseId,
        input_metadata: GuiHelloWorldActualInputMetadata,
        classification_plan: GuiHelloWorldInitialBlockerPlan,
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
            blocker: GuiHelloWorldActualBlocker::from_classification_plan(&classification_plan),
        }
    }
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
    bridge_boundary: GuiHelloWorldActualBridgeBoundaryPreparation,
    integration_policy: GuiHelloWorldActualIntegrationPolicy,
    process_boundary: GuiHelloWorldActualProcessBoundary,
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
            bridge_boundary: GuiHelloWorldActualBridgeBoundaryPreparation::from_plan(
                plan.bridge_boundary(),
            ),
            integration_policy: GuiHelloWorldActualIntegrationPolicy::from_policy(
                plan.integration_policy(),
            ),
            process_boundary: GuiHelloWorldActualProcessBoundary::from_boundary(
                plan.process_boundary(),
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
}

impl GuiHelloWorldActualHelperBoundaryPreparation {
    const fn from_plan(plan: &bara_runtime::UserSpaceHelperBoundaryPlan) -> Self {
        Self {
            responsibility: GuiHelloWorldActualRuntimePreparationResponsibility::from_runtime(
                plan.responsibility(),
            ),
            contract: GuiHelloWorldActualHelperBoundaryContract::from_runtime(plan.contract()),
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
    #[serde(rename = "unsupported_import")]
    Import,
    #[serde(rename = "unsupported_loader_feature")]
    LoaderFeature,
    #[serde(rename = "unsupported_objc_runtime_boundary")]
    ObjcRuntimeBoundary,
}

impl GuiHelloWorldActualBlockerClassification {
    const fn stderr_message(self) -> &'static str {
        match self {
            Self::Import => "unsupported_boundary: unsupported_import",
            Self::LoaderFeature => "unsupported_boundary: unsupported_loader_feature",
            Self::ObjcRuntimeBoundary => "unsupported_boundary: unsupported_objc_runtime_boundary",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldUnsupportedLaunchBoundary {
    #[serde(rename = "import")]
    Import,
    #[serde(rename = "loader")]
    Loader,
    #[serde(rename = "objc_runtime")]
    ObjcRuntime,
}

impl GuiHelloWorldUnsupportedLaunchBoundary {
    const fn classification(self) -> GuiHelloWorldActualBlockerClassification {
        match self {
            Self::Import => GuiHelloWorldActualBlockerClassification::Import,
            Self::Loader => GuiHelloWorldActualBlockerClassification::LoaderFeature,
            Self::ObjcRuntime => GuiHelloWorldActualBlockerClassification::ObjcRuntimeBoundary,
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::Import => {
                "Bara does not yet resolve the GUI fixture's public AppKit import boundary."
            }
            Self::Loader => {
                "Bara does not yet load a complete x86_64 Mach-O GUI executable with dynamic loader, AppKit import, and Objective-C runtime requirements."
            }
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
                GuiHelloWorldUnsupportedLaunchBoundary::Loader,
                vec![
                    GuiHelloWorldUnsupportedLaunchBoundary::Import,
                    GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime,
                ],
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

#[cfg(test)]
mod tests {
    use bara_oracle::{probe_public_binary_format, BinaryInput, CaseId, ObservedResult};

    use super::{
        b8_gui_hello_world_actual_launch_attempt, GuiHelloWorldActualBlockerClassification,
        GuiHelloWorldInitialBlockerPlan, GuiHelloWorldUnsupportedLaunchBoundary,
    };

    #[test]
    fn gui_hello_world_actual_attempt_reports_loader_blocker() {
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
                String::from("unsupported_boundary: unsupported_loader_feature"),
            )
        );
        assert_eq!(
            serde_json::to_string(attempt.launch_report()).expect("launch report serializes"),
            "{\"schema\":\"b8_gui_hello_world_actual_launch_report_v0\",\"case_id\":\"b8_gui_hello_world\",\"actual_runtime\":\"bara_arm64_user_space\",\"status\":\"blocked\",\"input\":{\"kind\":\"mach_o_executable_image\",\"source_isa\":\"x86_64\",\"binary_format\":\"mach_o\",\"target_triple\":\"x86_64-apple-macos13\",\"gui_framework\":\"appkit\",\"probe\":{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\"},\"loader_metadata\":{\"source\":\"public_mach_o_probe\",\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}},\"sections\":{\"status\":\"modeled_from_lc_segment_64_section_table\"},\"imports\":{\"status\":\"modeled_from_dylib_load_commands\"},\"relocations\":{\"status\":\"modeled_from_linkedit_relocation_and_bind_commands\"}}},\"runtime_preparation\":{\"source\":\"bara_runtime_user_space_launch_plan\",\"status\":\"planned_not_executed\",\"source_isa_profile\":{\"mode\":\"x86_64_long_mode\",\"address_size\":\"bits_64\",\"default_operand_size\":\"bits_32\",\"stack_width\":\"bits_64\"},\"image_mapping\":{\"responsibility\":\"loader\",\"source\":\"mach_o_executable_image\",\"memory_protection\":\"public_os_virtual_memory\"},\"executable_memory\":{\"responsibility\":\"runtime\",\"allocation_api\":\"mmap_private_anonymous\",\"protection_transition\":\"mprotect_read_write_to_read_execute\",\"release_api\":\"munmap\"},\"execution_strategy\":{\"responsibility\":\"runtime\",\"boundary\":\"user_space_runtime\",\"jit\":\"selectable\",\"aot\":\"selectable\",\"fallback_interpreter\":\"selectable\"},\"entry_trampoline\":{\"responsibility\":\"runtime\",\"target\":\"mach_o_entry_point\"},\"initial_stack\":{\"responsibility\":\"runtime\",\"contract\":\"argv_envp_initial_stack\"},\"helper_boundary\":{\"responsibility\":\"helper_boundary\",\"contract\":\"imports_objc_os_api_requests\"},\"bridge_boundary\":{\"responsibility\":\"helper_boundary\",\"syscall_bridge\":\"helper_boundary\",\"os_api_bridge\":\"helper_boundary\",\"core_ir_implementation\":\"not_embedded\",\"arm64_emit_implementation\":\"not_embedded\"},\"integration_policy\":{\"process_scope\":\"current_user_space_process\",\"kernel_extension\":\"not_required\",\"private_kernel_hook\":\"not_required\",\"private_dyld_behavior\":\"not_required\"},\"process_boundary\":{\"loader\":\"current_user_space_process\",\"translation_cache\":\"current_user_space_process\",\"runtime_helper\":\"current_user_space_process\",\"artifact_cache\":\"current_user_space_process\"}},\"blocker\":{\"classification\":\"unsupported_loader_feature\",\"boundary\":\"loader\",\"selected_by\":\"first_unsupported_launch_boundary\",\"candidate_boundaries\":[{\"boundary\":\"loader\",\"classification\":\"unsupported_loader_feature\"},{\"boundary\":\"import\",\"classification\":\"unsupported_import\"},{\"boundary\":\"objc_runtime\",\"classification\":\"unsupported_objc_runtime_boundary\"}],\"message\":\"Bara does not yet load a complete x86_64 Mach-O GUI executable with dynamic loader, AppKit import, and Objective-C runtime requirements.\"}}"
        );
    }

    #[test]
    fn initial_blocker_plan_selects_loader_before_import_and_objc_runtime() {
        let plan = GuiHelloWorldInitialBlockerPlan::current();

        assert_eq!(
            plan.selected_classification(),
            GuiHelloWorldActualBlockerClassification::LoaderFeature
        );
        assert_eq!(
            plan.candidate_boundaries(),
            vec![
                super::GuiHelloWorldActualBlockerCandidate::from_boundary(
                    GuiHelloWorldUnsupportedLaunchBoundary::Loader
                ),
                super::GuiHelloWorldActualBlockerCandidate::from_boundary(
                    GuiHelloWorldUnsupportedLaunchBoundary::Import
                ),
                super::GuiHelloWorldActualBlockerCandidate::from_boundary(
                    GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime
                ),
            ]
        );
    }

    #[test]
    fn initial_blocker_plan_has_stable_import_and_objc_runtime_classifications() {
        let import_plan = GuiHelloWorldInitialBlockerPlan::with_first_boundary(
            GuiHelloWorldUnsupportedLaunchBoundary::Import,
        );
        let objc_runtime_plan = GuiHelloWorldInitialBlockerPlan::with_first_boundary(
            GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime,
        );

        assert_eq!(
            import_plan.selected_classification(),
            GuiHelloWorldActualBlockerClassification::Import
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
