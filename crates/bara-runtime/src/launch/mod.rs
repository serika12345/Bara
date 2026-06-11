#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserSpaceLaunchPlan {
    image_mapping: UserSpaceImageMappingPlan,
    entry_trampoline: UserSpaceEntryTrampolinePlan,
    initial_stack: UserSpaceInitialStackPlan,
    helper_boundary: UserSpaceHelperBoundaryPlan,
    integration_policy: UserSpaceIntegrationPolicy,
}

impl UserSpaceLaunchPlan {
    pub const fn mach_o_executable_image() -> Self {
        Self {
            image_mapping: UserSpaceImageMappingPlan::mach_o_executable_image(),
            entry_trampoline: UserSpaceEntryTrampolinePlan::mach_o_entry_point(),
            initial_stack: UserSpaceInitialStackPlan::argv_envp_initial_stack(),
            helper_boundary: UserSpaceHelperBoundaryPlan::imports_objc_os_api_requests(),
            integration_policy: UserSpaceIntegrationPolicy::current_user_space_process(),
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

    pub const fn integration_policy(&self) -> &UserSpaceIntegrationPolicy {
        &self.integration_policy
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

#[cfg(test)]
mod tests {
    use super::{
        UserSpaceEntryTrampolineTarget, UserSpaceHelperBoundaryContract,
        UserSpaceImageMappingSource, UserSpaceInitialStackContract, UserSpaceLaunchPlan,
        UserSpaceLaunchResponsibility, UserSpaceMemoryProtectionModel,
        UserSpacePrivateIntegrationRequirement, UserSpaceProcessScope,
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
}
