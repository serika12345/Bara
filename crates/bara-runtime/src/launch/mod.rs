#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserSpaceLaunchPlan {
    image_mapping: UserSpaceImageMappingPlan,
    entry_trampoline: UserSpaceEntryTrampolinePlan,
    initial_stack: UserSpaceInitialStackPlan,
    helper_boundary: UserSpaceHelperBoundaryPlan,
    executable_memory: UserSpaceExecutableMemoryPlan,
    execution_strategy: UserSpaceExecutionStrategyPlan,
    integration_policy: UserSpaceIntegrationPolicy,
    process_boundary: UserSpaceProcessBoundary,
}

impl UserSpaceLaunchPlan {
    pub const fn mach_o_executable_image() -> Self {
        Self {
            image_mapping: UserSpaceImageMappingPlan::mach_o_executable_image(),
            entry_trampoline: UserSpaceEntryTrampolinePlan::mach_o_entry_point(),
            initial_stack: UserSpaceInitialStackPlan::argv_envp_initial_stack(),
            helper_boundary: UserSpaceHelperBoundaryPlan::imports_objc_os_api_requests(),
            executable_memory: UserSpaceExecutableMemoryPlan::public_os_api(),
            execution_strategy: UserSpaceExecutionStrategyPlan::user_space_runtime_selectable(),
            integration_policy: UserSpaceIntegrationPolicy::current_user_space_process(),
            process_boundary: UserSpaceProcessBoundary::current_user_space_process(),
        }
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

#[cfg(test)]
mod tests {
    use super::{
        UserSpaceEntryTrampolineTarget, UserSpaceExecutableMemoryAllocationApi,
        UserSpaceExecutableMemoryProtectionTransition, UserSpaceExecutableMemoryReleaseApi,
        UserSpaceExecutionStrategyAvailability, UserSpaceExecutionStrategyBoundary,
        UserSpaceHelperBoundaryContract, UserSpaceImageMappingSource,
        UserSpaceInitialStackContract, UserSpaceLaunchPlan, UserSpaceLaunchResponsibility,
        UserSpaceMemoryProtectionModel, UserSpacePrivateIntegrationRequirement,
        UserSpaceProcessScope,
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
}
