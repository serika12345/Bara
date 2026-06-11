#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserSpaceLaunchPlan {
    source_isa_profile: UserSpaceSourceIsaProfile,
    image_mapping: UserSpaceImageMappingPlan,
    entry_trampoline: UserSpaceEntryTrampolinePlan,
    initial_stack: UserSpaceInitialStackPlan,
    helper_boundary: UserSpaceHelperBoundaryPlan,
    bridge_boundary: UserSpaceBridgeBoundaryPlan,
    executable_memory: UserSpaceExecutableMemoryPlan,
    execution_strategy: UserSpaceExecutionStrategyPlan,
    integration_policy: UserSpaceIntegrationPolicy,
    process_boundary: UserSpaceProcessBoundary,
    platform_model: UserSpacePlatformModelPlan,
    macos_constraints: UserSpaceMacosConstraints,
    fallback_policy: UserSpaceFallbackPolicy,
    loader_execution: UserSpaceLoaderExecutionPlan,
}

impl UserSpaceLaunchPlan {
    pub const fn mach_o_executable_image() -> Self {
        Self {
            source_isa_profile: UserSpaceSourceIsaProfile::x86_64_long_mode(),
            image_mapping: UserSpaceImageMappingPlan::mach_o_executable_image(),
            entry_trampoline: UserSpaceEntryTrampolinePlan::mach_o_entry_point(),
            initial_stack: UserSpaceInitialStackPlan::argv_envp_initial_stack(),
            helper_boundary: UserSpaceHelperBoundaryPlan::imports_objc_os_api_requests(),
            bridge_boundary: UserSpaceBridgeBoundaryPlan::syscall_and_os_api_helpers(),
            executable_memory: UserSpaceExecutableMemoryPlan::public_os_api(),
            execution_strategy: UserSpaceExecutionStrategyPlan::user_space_runtime_selectable(),
            integration_policy: UserSpaceIntegrationPolicy::current_user_space_process(),
            process_boundary: UserSpaceProcessBoundary::current_user_space_process(),
            platform_model: UserSpacePlatformModelPlan::initial_gui_loader_model(),
            macos_constraints: UserSpaceMacosConstraints::public_documented_behavior(),
            fallback_policy: UserSpaceFallbackPolicy::ready_for_feedback_cycle(),
            loader_execution: UserSpaceLoaderExecutionPlan::public_mach_o_probe_plan(),
        }
    }

    pub const fn source_isa_profile(&self) -> &UserSpaceSourceIsaProfile {
        &self.source_isa_profile
    }

    pub const fn image_mapping(&self) -> &UserSpaceImageMappingPlan {
        &self.image_mapping
    }

    pub const fn entry_trampoline(&self) -> &UserSpaceEntryTrampolinePlan {
        &self.entry_trampoline
    }

    pub const fn initial_stack(&self) -> &UserSpaceInitialStackPlan {
        &self.initial_stack
    }

    pub const fn helper_boundary(&self) -> &UserSpaceHelperBoundaryPlan {
        &self.helper_boundary
    }

    pub const fn bridge_boundary(&self) -> &UserSpaceBridgeBoundaryPlan {
        &self.bridge_boundary
    }

    pub const fn executable_memory(&self) -> &UserSpaceExecutableMemoryPlan {
        &self.executable_memory
    }

    pub const fn execution_strategy(&self) -> &UserSpaceExecutionStrategyPlan {
        &self.execution_strategy
    }

    pub const fn integration_policy(&self) -> &UserSpaceIntegrationPolicy {
        &self.integration_policy
    }

    pub const fn process_boundary(&self) -> &UserSpaceProcessBoundary {
        &self.process_boundary
    }

    pub const fn platform_model(&self) -> &UserSpacePlatformModelPlan {
        &self.platform_model
    }

    pub const fn macos_constraints(&self) -> &UserSpaceMacosConstraints {
        &self.macos_constraints
    }

    pub const fn fallback_policy(&self) -> &UserSpaceFallbackPolicy {
        &self.fallback_policy
    }

    pub const fn loader_execution(&self) -> &UserSpaceLoaderExecutionPlan {
        &self.loader_execution
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceSourceIsaProfile {
    mode: UserSpaceSourceIsaMode,
    address_size: UserSpaceSourceWidth,
    default_operand_size: UserSpaceSourceWidth,
    stack_width: UserSpaceSourceWidth,
}

impl UserSpaceSourceIsaProfile {
    pub const fn x86_64_long_mode() -> Self {
        Self {
            mode: UserSpaceSourceIsaMode::X8664LongMode,
            address_size: UserSpaceSourceWidth::Bits64,
            default_operand_size: UserSpaceSourceWidth::Bits32,
            stack_width: UserSpaceSourceWidth::Bits64,
        }
    }

    pub const fn x86_32_protected_mode() -> Self {
        Self {
            mode: UserSpaceSourceIsaMode::X8632ProtectedMode,
            address_size: UserSpaceSourceWidth::Bits32,
            default_operand_size: UserSpaceSourceWidth::Bits32,
            stack_width: UserSpaceSourceWidth::Bits32,
        }
    }

    pub const fn mode(self) -> UserSpaceSourceIsaMode {
        self.mode
    }

    pub const fn address_size(self) -> UserSpaceSourceWidth {
        self.address_size
    }

    pub const fn default_operand_size(self) -> UserSpaceSourceWidth {
        self.default_operand_size
    }

    pub const fn stack_width(self) -> UserSpaceSourceWidth {
        self.stack_width
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceSourceIsaMode {
    X8664LongMode,
    X8632ProtectedMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceSourceWidth {
    Bits32,
    Bits64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceImageMappingPlan {
    responsibility: UserSpaceLaunchResponsibility,
    source: UserSpaceImageMappingSource,
    memory_protection: UserSpaceMemoryProtectionModel,
}

impl UserSpaceImageMappingPlan {
    const fn mach_o_executable_image() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::Loader,
            source: UserSpaceImageMappingSource::MachOExecutableImage,
            memory_protection: UserSpaceMemoryProtectionModel::PublicOsVirtualMemory,
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn source(self) -> UserSpaceImageMappingSource {
        self.source
    }

    pub const fn memory_protection(self) -> UserSpaceMemoryProtectionModel {
        self.memory_protection
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceEntryTrampolinePlan {
    responsibility: UserSpaceLaunchResponsibility,
    target: UserSpaceEntryTrampolineTarget,
}

impl UserSpaceEntryTrampolinePlan {
    const fn mach_o_entry_point() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::Runtime,
            target: UserSpaceEntryTrampolineTarget::MachOEntryPoint,
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn target(self) -> UserSpaceEntryTrampolineTarget {
        self.target
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceInitialStackPlan {
    responsibility: UserSpaceLaunchResponsibility,
    contract: UserSpaceInitialStackContract,
}

impl UserSpaceInitialStackPlan {
    const fn argv_envp_initial_stack() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::Runtime,
            contract: UserSpaceInitialStackContract::ArgvEnvpInitialStack,
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn contract(self) -> UserSpaceInitialStackContract {
        self.contract
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceHelperBoundaryPlan {
    responsibility: UserSpaceLaunchResponsibility,
    contract: UserSpaceHelperBoundaryContract,
}

impl UserSpaceHelperBoundaryPlan {
    const fn imports_objc_os_api_requests() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::HelperBoundary,
            contract: UserSpaceHelperBoundaryContract::ImportsObjcOsApiRequests,
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn contract(self) -> UserSpaceHelperBoundaryContract {
        self.contract
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLaunchResponsibility {
    HelperBoundary,
    Loader,
    Runtime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceImageMappingSource {
    MachOExecutableImage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceMemoryProtectionModel {
    PublicOsVirtualMemory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceEntryTrampolineTarget {
    MachOEntryPoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceInitialStackContract {
    ArgvEnvpInitialStack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceHelperBoundaryContract {
    ImportsObjcOsApiRequests,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceBridgeBoundaryPlan {
    responsibility: UserSpaceLaunchResponsibility,
    syscall_bridge: UserSpaceBridgeBoundaryPlacement,
    os_api_bridge: UserSpaceBridgeBoundaryPlacement,
    core_ir_implementation: UserSpaceBridgeCoreImplementation,
    arm64_emit_implementation: UserSpaceBridgeCoreImplementation,
}

impl UserSpaceBridgeBoundaryPlan {
    const fn syscall_and_os_api_helpers() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::HelperBoundary,
            syscall_bridge: UserSpaceBridgeBoundaryPlacement::HelperBoundary,
            os_api_bridge: UserSpaceBridgeBoundaryPlacement::HelperBoundary,
            core_ir_implementation: UserSpaceBridgeCoreImplementation::NotEmbedded,
            arm64_emit_implementation: UserSpaceBridgeCoreImplementation::NotEmbedded,
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn syscall_bridge(self) -> UserSpaceBridgeBoundaryPlacement {
        self.syscall_bridge
    }

    pub const fn os_api_bridge(self) -> UserSpaceBridgeBoundaryPlacement {
        self.os_api_bridge
    }

    pub const fn core_ir_implementation(self) -> UserSpaceBridgeCoreImplementation {
        self.core_ir_implementation
    }

    pub const fn arm64_emit_implementation(self) -> UserSpaceBridgeCoreImplementation {
        self.arm64_emit_implementation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceBridgeBoundaryPlacement {
    HelperBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceBridgeCoreImplementation {
    NotEmbedded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceExecutableMemoryPlan {
    responsibility: UserSpaceLaunchResponsibility,
    allocation_api: UserSpaceExecutableMemoryAllocationApi,
    protection_transition: UserSpaceExecutableMemoryProtectionTransition,
    release_api: UserSpaceExecutableMemoryReleaseApi,
}

impl UserSpaceExecutableMemoryPlan {
    const fn public_os_api() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::Runtime,
            allocation_api: UserSpaceExecutableMemoryAllocationApi::MmapPrivateAnonymous,
            protection_transition:
                UserSpaceExecutableMemoryProtectionTransition::MprotectReadWriteToReadExecute,
            release_api: UserSpaceExecutableMemoryReleaseApi::Munmap,
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn allocation_api(self) -> UserSpaceExecutableMemoryAllocationApi {
        self.allocation_api
    }

    pub const fn protection_transition(self) -> UserSpaceExecutableMemoryProtectionTransition {
        self.protection_transition
    }

    pub const fn release_api(self) -> UserSpaceExecutableMemoryReleaseApi {
        self.release_api
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceExecutableMemoryAllocationApi {
    MmapPrivateAnonymous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceExecutableMemoryProtectionTransition {
    MprotectReadWriteToReadExecute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceExecutableMemoryReleaseApi {
    Munmap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceExecutionStrategyPlan {
    responsibility: UserSpaceLaunchResponsibility,
    boundary: UserSpaceExecutionStrategyBoundary,
    strategies: UserSpaceExecutionStrategySet,
}

impl UserSpaceExecutionStrategyPlan {
    const fn user_space_runtime_selectable() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::Runtime,
            boundary: UserSpaceExecutionStrategyBoundary::UserSpaceRuntime,
            strategies: UserSpaceExecutionStrategySet::all_selectable(),
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn boundary(self) -> UserSpaceExecutionStrategyBoundary {
        self.boundary
    }

    pub const fn strategies(self) -> UserSpaceExecutionStrategySet {
        self.strategies
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceExecutionStrategyBoundary {
    UserSpaceRuntime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceExecutionStrategySet {
    jit: UserSpaceExecutionStrategyAvailability,
    aot: UserSpaceExecutionStrategyAvailability,
    fallback_interpreter: UserSpaceExecutionStrategyAvailability,
}

impl UserSpaceExecutionStrategySet {
    const fn all_selectable() -> Self {
        Self {
            jit: UserSpaceExecutionStrategyAvailability::Selectable,
            aot: UserSpaceExecutionStrategyAvailability::Selectable,
            fallback_interpreter: UserSpaceExecutionStrategyAvailability::Selectable,
        }
    }

    pub const fn jit(self) -> UserSpaceExecutionStrategyAvailability {
        self.jit
    }

    pub const fn aot(self) -> UserSpaceExecutionStrategyAvailability {
        self.aot
    }

    pub const fn fallback_interpreter(self) -> UserSpaceExecutionStrategyAvailability {
        self.fallback_interpreter
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceExecutionStrategyAvailability {
    Selectable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceIntegrationPolicy {
    process_scope: UserSpaceProcessScope,
    kernel_extension: UserSpacePrivateIntegrationRequirement,
    private_kernel_hook: UserSpacePrivateIntegrationRequirement,
    private_dyld_behavior: UserSpacePrivateIntegrationRequirement,
}

impl UserSpaceIntegrationPolicy {
    const fn current_user_space_process() -> Self {
        Self {
            process_scope: UserSpaceProcessScope::CurrentUserSpaceProcess,
            kernel_extension: UserSpacePrivateIntegrationRequirement::NotRequired,
            private_kernel_hook: UserSpacePrivateIntegrationRequirement::NotRequired,
            private_dyld_behavior: UserSpacePrivateIntegrationRequirement::NotRequired,
        }
    }

    pub const fn process_scope(self) -> UserSpaceProcessScope {
        self.process_scope
    }

    pub const fn kernel_extension(self) -> UserSpacePrivateIntegrationRequirement {
        self.kernel_extension
    }

    pub const fn private_kernel_hook(self) -> UserSpacePrivateIntegrationRequirement {
        self.private_kernel_hook
    }

    pub const fn private_dyld_behavior(self) -> UserSpacePrivateIntegrationRequirement {
        self.private_dyld_behavior
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceProcessScope {
    CurrentUserSpaceProcess,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpacePrivateIntegrationRequirement {
    NotRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceProcessBoundary {
    loader: UserSpaceProcessScope,
    translation_cache: UserSpaceProcessScope,
    runtime_helper: UserSpaceProcessScope,
    artifact_cache: UserSpaceProcessScope,
}

impl UserSpaceProcessBoundary {
    const fn current_user_space_process() -> Self {
        Self {
            loader: UserSpaceProcessScope::CurrentUserSpaceProcess,
            translation_cache: UserSpaceProcessScope::CurrentUserSpaceProcess,
            runtime_helper: UserSpaceProcessScope::CurrentUserSpaceProcess,
            artifact_cache: UserSpaceProcessScope::CurrentUserSpaceProcess,
        }
    }

    pub const fn loader(self) -> UserSpaceProcessScope {
        self.loader
    }

    pub const fn translation_cache(self) -> UserSpaceProcessScope {
        self.translation_cache
    }

    pub const fn runtime_helper(self) -> UserSpaceProcessScope {
        self.runtime_helper
    }

    pub const fn artifact_cache(self) -> UserSpaceProcessScope {
        self.artifact_cache
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpacePlatformModelPlan {
    signal_model: UserSpacePlatformSignalModel,
    exception_model: UserSpacePlatformExceptionModel,
    thread_model: UserSpacePlatformThreadModel,
    tls_model: UserSpacePlatformTlsModel,
    memory_protection: UserSpacePlatformMemoryProtectionModel,
}

impl UserSpacePlatformModelPlan {
    const fn initial_gui_loader_model() -> Self {
        Self {
            signal_model: UserSpacePlatformSignalModel::UserSpaceLoaderBoundary,
            exception_model: UserSpacePlatformExceptionModel::UserSpaceLoaderBoundary,
            thread_model: UserSpacePlatformThreadModel::InitialThreadOnly,
            tls_model: UserSpacePlatformTlsModel::Deferred,
            memory_protection: UserSpacePlatformMemoryProtectionModel::PublicOsVirtualMemory,
        }
    }

    pub const fn signal_model(self) -> UserSpacePlatformSignalModel {
        self.signal_model
    }

    pub const fn exception_model(self) -> UserSpacePlatformExceptionModel {
        self.exception_model
    }

    pub const fn thread_model(self) -> UserSpacePlatformThreadModel {
        self.thread_model
    }

    pub const fn tls_model(self) -> UserSpacePlatformTlsModel {
        self.tls_model
    }

    pub const fn memory_protection(self) -> UserSpacePlatformMemoryProtectionModel {
        self.memory_protection
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpacePlatformSignalModel {
    UserSpaceLoaderBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpacePlatformExceptionModel {
    UserSpaceLoaderBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpacePlatformThreadModel {
    InitialThreadOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpacePlatformTlsModel {
    Deferred,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpacePlatformMemoryProtectionModel {
    PublicOsVirtualMemory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceMacosConstraints {
    code_signing: UserSpaceMacosCodeSigningPolicy,
    write_xor_execute: UserSpaceMacosWriteXorExecutePolicy,
    hardened_runtime: UserSpaceMacosHardenedRuntimePolicy,
}

impl UserSpaceMacosConstraints {
    const fn public_documented_behavior() -> Self {
        Self {
            code_signing: UserSpaceMacosCodeSigningPolicy::NoPrivateSigningBypass,
            write_xor_execute: UserSpaceMacosWriteXorExecutePolicy::PublicMmapMprotectTransition,
            hardened_runtime: UserSpaceMacosHardenedRuntimePolicy::DocumentedHostPolicyOnly,
        }
    }

    pub const fn code_signing(self) -> UserSpaceMacosCodeSigningPolicy {
        self.code_signing
    }

    pub const fn write_xor_execute(self) -> UserSpaceMacosWriteXorExecutePolicy {
        self.write_xor_execute
    }

    pub const fn hardened_runtime(self) -> UserSpaceMacosHardenedRuntimePolicy {
        self.hardened_runtime
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceMacosCodeSigningPolicy {
    NoPrivateSigningBypass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceMacosWriteXorExecutePolicy {
    PublicMmapMprotectTransition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceMacosHardenedRuntimePolicy {
    DocumentedHostPolicyOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceFallbackPolicy {
    unimplemented_instruction: UserSpaceFallbackPolicyAction,
    unknown_indirect_target: UserSpaceFallbackPolicyAction,
    unsupported_loader_feature: UserSpaceFallbackPolicyAction,
    interpreter: UserSpaceFallbackEngineStatus,
    external_engine: UserSpaceFallbackEngineStatus,
    feedback_cycle: UserSpaceFeedbackCycleState,
}

impl UserSpaceFallbackPolicy {
    const fn ready_for_feedback_cycle() -> Self {
        Self {
            unimplemented_instruction: UserSpaceFallbackPolicyAction::StableBlockerClassification,
            unknown_indirect_target: UserSpaceFallbackPolicyAction::StableBlockerClassification,
            unsupported_loader_feature: UserSpaceFallbackPolicyAction::StableBlockerClassification,
            interpreter: UserSpaceFallbackEngineStatus::CandidateNotImplemented,
            external_engine: UserSpaceFallbackEngineStatus::CandidateNotConnected,
            feedback_cycle: UserSpaceFeedbackCycleState::ReadyNotStarted,
        }
    }

    pub const fn unimplemented_instruction(self) -> UserSpaceFallbackPolicyAction {
        self.unimplemented_instruction
    }

    pub const fn unknown_indirect_target(self) -> UserSpaceFallbackPolicyAction {
        self.unknown_indirect_target
    }

    pub const fn unsupported_loader_feature(self) -> UserSpaceFallbackPolicyAction {
        self.unsupported_loader_feature
    }

    pub const fn interpreter(self) -> UserSpaceFallbackEngineStatus {
        self.interpreter
    }

    pub const fn external_engine(self) -> UserSpaceFallbackEngineStatus {
        self.external_engine
    }

    pub const fn feedback_cycle(self) -> UserSpaceFeedbackCycleState {
        self.feedback_cycle
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceFallbackPolicyAction {
    StableBlockerClassification,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceFallbackEngineStatus {
    CandidateNotImplemented,
    CandidateNotConnected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceFeedbackCycleState {
    ReadyNotStarted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserSpaceLoaderExecutionPlan {
    responsibility: UserSpaceLaunchResponsibility,
    metadata_source: UserSpaceLoaderMetadataSource,
    entry_point: UserSpaceLoaderEntryPointPlan,
    segment_mapping: UserSpaceLoaderSegmentMappingPlan,
    imports: UserSpaceLoaderImportPlan,
    relocations: UserSpaceLoaderRelocationPlan,
    objc_runtime: UserSpaceLoaderObjcRuntimePlan,
    status: UserSpaceLoaderExecutionStatus,
}

impl UserSpaceLoaderExecutionPlan {
    const fn public_mach_o_probe_plan() -> Self {
        Self {
            responsibility: UserSpaceLaunchResponsibility::Loader,
            metadata_source: UserSpaceLoaderMetadataSource::PublicMachOProbe,
            entry_point: UserSpaceLoaderEntryPointPlan::LcMainEntryoff,
            segment_mapping: UserSpaceLoaderSegmentMappingPlan::LcSegment64FileRanges,
            imports: UserSpaceLoaderImportPlan::DylibLoadCommandsToHelperBoundary,
            relocations: UserSpaceLoaderRelocationPlan::LinkeditRebaseBindMetadata,
            objc_runtime: UserSpaceLoaderObjcRuntimePlan::HelperBoundary,
            status: UserSpaceLoaderExecutionStatus::PlannedNotExecuted,
        }
    }

    pub const fn responsibility(self) -> UserSpaceLaunchResponsibility {
        self.responsibility
    }

    pub const fn metadata_source(self) -> UserSpaceLoaderMetadataSource {
        self.metadata_source
    }

    pub const fn entry_point(self) -> UserSpaceLoaderEntryPointPlan {
        self.entry_point
    }

    pub const fn segment_mapping(self) -> UserSpaceLoaderSegmentMappingPlan {
        self.segment_mapping
    }

    pub const fn imports(self) -> UserSpaceLoaderImportPlan {
        self.imports
    }

    pub const fn relocations(self) -> UserSpaceLoaderRelocationPlan {
        self.relocations
    }

    pub const fn objc_runtime(self) -> UserSpaceLoaderObjcRuntimePlan {
        self.objc_runtime
    }

    pub const fn status(self) -> UserSpaceLoaderExecutionStatus {
        self.status
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLoaderMetadataSource {
    PublicMachOProbe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLoaderEntryPointPlan {
    LcMainEntryoff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLoaderSegmentMappingPlan {
    LcSegment64FileRanges,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLoaderImportPlan {
    DylibLoadCommandsToHelperBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLoaderRelocationPlan {
    LinkeditRebaseBindMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLoaderObjcRuntimePlan {
    HelperBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSpaceLoaderExecutionStatus {
    PlannedNotExecuted,
}

#[cfg(test)]
mod tests {
    use super::{
        UserSpaceBridgeBoundaryPlacement, UserSpaceBridgeCoreImplementation,
        UserSpaceEntryTrampolineTarget, UserSpaceExecutableMemoryAllocationApi,
        UserSpaceExecutableMemoryProtectionTransition, UserSpaceExecutableMemoryReleaseApi,
        UserSpaceExecutionStrategyAvailability, UserSpaceExecutionStrategyBoundary,
        UserSpaceFallbackEngineStatus, UserSpaceFallbackPolicyAction, UserSpaceFeedbackCycleState,
        UserSpaceHelperBoundaryContract, UserSpaceImageMappingSource,
        UserSpaceInitialStackContract, UserSpaceLaunchPlan, UserSpaceLaunchResponsibility,
        UserSpaceLoaderEntryPointPlan, UserSpaceLoaderExecutionStatus, UserSpaceLoaderImportPlan,
        UserSpaceLoaderMetadataSource, UserSpaceLoaderObjcRuntimePlan,
        UserSpaceLoaderRelocationPlan, UserSpaceLoaderSegmentMappingPlan,
        UserSpaceMacosCodeSigningPolicy, UserSpaceMacosHardenedRuntimePolicy,
        UserSpaceMacosWriteXorExecutePolicy, UserSpaceMemoryProtectionModel,
        UserSpacePlatformExceptionModel, UserSpacePlatformMemoryProtectionModel,
        UserSpacePlatformSignalModel, UserSpacePlatformThreadModel, UserSpacePlatformTlsModel,
        UserSpacePrivateIntegrationRequirement, UserSpaceProcessScope, UserSpaceSourceIsaMode,
        UserSpaceSourceIsaProfile, UserSpaceSourceWidth,
    };

    #[test]
    fn user_space_launch_plan_splits_loader_runtime_stack_and_helper_responsibilities() {
        let plan = UserSpaceLaunchPlan::mach_o_executable_image();

        assert_eq!(
            plan.image_mapping().responsibility(),
            UserSpaceLaunchResponsibility::Loader
        );
        assert_eq!(
            plan.image_mapping().source(),
            UserSpaceImageMappingSource::MachOExecutableImage
        );
        assert_eq!(
            plan.image_mapping().memory_protection(),
            UserSpaceMemoryProtectionModel::PublicOsVirtualMemory
        );
        assert_eq!(
            plan.entry_trampoline().responsibility(),
            UserSpaceLaunchResponsibility::Runtime
        );
        assert_eq!(
            plan.entry_trampoline().target(),
            UserSpaceEntryTrampolineTarget::MachOEntryPoint
        );
        assert_eq!(
            plan.initial_stack().responsibility(),
            UserSpaceLaunchResponsibility::Runtime
        );
        assert_eq!(
            plan.initial_stack().contract(),
            UserSpaceInitialStackContract::ArgvEnvpInitialStack
        );
        assert_eq!(
            plan.helper_boundary().responsibility(),
            UserSpaceLaunchResponsibility::HelperBoundary
        );
        assert_eq!(
            plan.helper_boundary().contract(),
            UserSpaceHelperBoundaryContract::ImportsObjcOsApiRequests
        );
    }

    #[test]
    fn user_space_launch_plan_requires_no_private_kernel_or_dyld_integration() {
        let policy = *UserSpaceLaunchPlan::mach_o_executable_image().integration_policy();

        assert_eq!(
            policy.process_scope(),
            UserSpaceProcessScope::CurrentUserSpaceProcess
        );
        assert_eq!(
            policy.kernel_extension(),
            UserSpacePrivateIntegrationRequirement::NotRequired
        );
        assert_eq!(
            policy.private_kernel_hook(),
            UserSpacePrivateIntegrationRequirement::NotRequired
        );
        assert_eq!(
            policy.private_dyld_behavior(),
            UserSpacePrivateIntegrationRequirement::NotRequired
        );
    }

    #[test]
    fn user_space_launch_plan_keeps_loader_caches_and_helpers_in_process() {
        let boundary = *UserSpaceLaunchPlan::mach_o_executable_image().process_boundary();

        assert_eq!(
            boundary.loader(),
            UserSpaceProcessScope::CurrentUserSpaceProcess
        );
        assert_eq!(
            boundary.translation_cache(),
            UserSpaceProcessScope::CurrentUserSpaceProcess
        );
        assert_eq!(
            boundary.runtime_helper(),
            UserSpaceProcessScope::CurrentUserSpaceProcess
        );
        assert_eq!(
            boundary.artifact_cache(),
            UserSpaceProcessScope::CurrentUserSpaceProcess
        );
    }

    #[test]
    fn user_space_launch_plan_limits_executable_memory_to_public_os_api() {
        let executable_memory = *UserSpaceLaunchPlan::mach_o_executable_image().executable_memory();

        assert_eq!(
            executable_memory.responsibility(),
            UserSpaceLaunchResponsibility::Runtime
        );
        assert_eq!(
            executable_memory.allocation_api(),
            UserSpaceExecutableMemoryAllocationApi::MmapPrivateAnonymous
        );
        assert_eq!(
            executable_memory.protection_transition(),
            UserSpaceExecutableMemoryProtectionTransition::MprotectReadWriteToReadExecute
        );
        assert_eq!(
            executable_memory.release_api(),
            UserSpaceExecutableMemoryReleaseApi::Munmap
        );
    }

    #[test]
    fn user_space_launch_plan_selects_execution_strategies_from_runtime_boundary() {
        let execution_strategy =
            *UserSpaceLaunchPlan::mach_o_executable_image().execution_strategy();
        let strategies = execution_strategy.strategies();

        assert_eq!(
            execution_strategy.responsibility(),
            UserSpaceLaunchResponsibility::Runtime
        );
        assert_eq!(
            execution_strategy.boundary(),
            UserSpaceExecutionStrategyBoundary::UserSpaceRuntime
        );
        assert_eq!(
            strategies.jit(),
            UserSpaceExecutionStrategyAvailability::Selectable
        );
        assert_eq!(
            strategies.aot(),
            UserSpaceExecutionStrategyAvailability::Selectable
        );
        assert_eq!(
            strategies.fallback_interpreter(),
            UserSpaceExecutionStrategyAvailability::Selectable
        );
    }

    #[test]
    fn user_space_launch_plan_keeps_syscall_and_os_api_bridges_at_helper_boundary() {
        let bridge_boundary = *UserSpaceLaunchPlan::mach_o_executable_image().bridge_boundary();

        assert_eq!(
            bridge_boundary.responsibility(),
            UserSpaceLaunchResponsibility::HelperBoundary
        );
        assert_eq!(
            bridge_boundary.syscall_bridge(),
            UserSpaceBridgeBoundaryPlacement::HelperBoundary
        );
        assert_eq!(
            bridge_boundary.os_api_bridge(),
            UserSpaceBridgeBoundaryPlacement::HelperBoundary
        );
        assert_eq!(
            bridge_boundary.core_ir_implementation(),
            UserSpaceBridgeCoreImplementation::NotEmbedded
        );
        assert_eq!(
            bridge_boundary.arm64_emit_implementation(),
            UserSpaceBridgeCoreImplementation::NotEmbedded
        );
    }

    #[test]
    fn user_space_launch_plan_keeps_source_isa_mode_and_widths_typed() {
        let source_isa = *UserSpaceLaunchPlan::mach_o_executable_image().source_isa_profile();

        assert_eq!(source_isa.mode(), UserSpaceSourceIsaMode::X8664LongMode);
        assert_eq!(source_isa.address_size(), UserSpaceSourceWidth::Bits64);
        assert_eq!(
            source_isa.default_operand_size(),
            UserSpaceSourceWidth::Bits32
        );
        assert_eq!(source_isa.stack_width(), UserSpaceSourceWidth::Bits64);
    }

    #[test]
    fn source_isa_profile_can_model_x86_32_widths_without_changing_b8_target() {
        let source_isa = UserSpaceSourceIsaProfile::x86_32_protected_mode();

        assert_eq!(
            source_isa.mode(),
            UserSpaceSourceIsaMode::X8632ProtectedMode
        );
        assert_eq!(source_isa.address_size(), UserSpaceSourceWidth::Bits32);
        assert_eq!(
            source_isa.default_operand_size(),
            UserSpaceSourceWidth::Bits32
        );
        assert_eq!(source_isa.stack_width(), UserSpaceSourceWidth::Bits32);
    }

    #[test]
    fn user_space_launch_plan_models_loader_platform_boundaries() {
        let platform = *UserSpaceLaunchPlan::mach_o_executable_image().platform_model();

        assert_eq!(
            platform.signal_model(),
            UserSpacePlatformSignalModel::UserSpaceLoaderBoundary
        );
        assert_eq!(
            platform.exception_model(),
            UserSpacePlatformExceptionModel::UserSpaceLoaderBoundary
        );
        assert_eq!(
            platform.thread_model(),
            UserSpacePlatformThreadModel::InitialThreadOnly
        );
        assert_eq!(platform.tls_model(), UserSpacePlatformTlsModel::Deferred);
        assert_eq!(
            platform.memory_protection(),
            UserSpacePlatformMemoryProtectionModel::PublicOsVirtualMemory
        );
    }

    #[test]
    fn user_space_launch_plan_records_public_macos_execution_constraints() {
        let constraints = *UserSpaceLaunchPlan::mach_o_executable_image().macos_constraints();

        assert_eq!(
            constraints.code_signing(),
            UserSpaceMacosCodeSigningPolicy::NoPrivateSigningBypass
        );
        assert_eq!(
            constraints.write_xor_execute(),
            UserSpaceMacosWriteXorExecutePolicy::PublicMmapMprotectTransition
        );
        assert_eq!(
            constraints.hardened_runtime(),
            UserSpaceMacosHardenedRuntimePolicy::DocumentedHostPolicyOnly
        );
    }

    #[test]
    fn user_space_launch_plan_is_ready_for_rosetta_comparison_feedback_cycle() {
        let fallback = *UserSpaceLaunchPlan::mach_o_executable_image().fallback_policy();

        assert_eq!(
            fallback.unimplemented_instruction(),
            UserSpaceFallbackPolicyAction::StableBlockerClassification
        );
        assert_eq!(
            fallback.unknown_indirect_target(),
            UserSpaceFallbackPolicyAction::StableBlockerClassification
        );
        assert_eq!(
            fallback.unsupported_loader_feature(),
            UserSpaceFallbackPolicyAction::StableBlockerClassification
        );
        assert_eq!(
            fallback.interpreter(),
            UserSpaceFallbackEngineStatus::CandidateNotImplemented
        );
        assert_eq!(
            fallback.external_engine(),
            UserSpaceFallbackEngineStatus::CandidateNotConnected
        );
        assert_eq!(
            fallback.feedback_cycle(),
            UserSpaceFeedbackCycleState::ReadyNotStarted
        );
    }

    #[test]
    fn user_space_launch_plan_models_initial_mach_o_loader_execution_plan() {
        let loader_execution = *UserSpaceLaunchPlan::mach_o_executable_image().loader_execution();

        assert_eq!(
            loader_execution.responsibility(),
            UserSpaceLaunchResponsibility::Loader
        );
        assert_eq!(
            loader_execution.metadata_source(),
            UserSpaceLoaderMetadataSource::PublicMachOProbe
        );
        assert_eq!(
            loader_execution.entry_point(),
            UserSpaceLoaderEntryPointPlan::LcMainEntryoff
        );
        assert_eq!(
            loader_execution.segment_mapping(),
            UserSpaceLoaderSegmentMappingPlan::LcSegment64FileRanges
        );
        assert_eq!(
            loader_execution.imports(),
            UserSpaceLoaderImportPlan::DylibLoadCommandsToHelperBoundary
        );
        assert_eq!(
            loader_execution.relocations(),
            UserSpaceLoaderRelocationPlan::LinkeditRebaseBindMetadata
        );
        assert_eq!(
            loader_execution.objc_runtime(),
            UserSpaceLoaderObjcRuntimePlan::HelperBoundary
        );
        assert_eq!(
            loader_execution.status(),
            UserSpaceLoaderExecutionStatus::PlannedNotExecuted
        );
    }
}
