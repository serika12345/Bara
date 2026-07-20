use bara_ir::X86Reg;

use crate::{GuestImageError, GuestProgramCounter, GuestRuntimeStateError, MacOsHostService};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeBoundaryBlocker {
    Loader(LoaderBoundaryBlocker),
    Dispatcher(DispatcherBoundaryBlocker),
    Helper(HelperBoundaryBlocker),
}

impl RuntimeBoundaryBlocker {
    pub const fn boundary(self) -> RuntimeBoundary {
        match self {
            Self::Loader(_) => RuntimeBoundary::Loader,
            Self::Dispatcher(_) => RuntimeBoundary::Dispatcher,
            Self::Helper(_) => RuntimeBoundary::Helper,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeBoundary {
    Loader,
    Dispatcher,
    Helper,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoaderBoundaryBlocker {
    InvalidGuestImage(GuestImageError),
    Unsupported(LoaderUnsupportedState),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoaderUnsupportedState {
    RelocationApplication,
    ImportAddressResolution,
    ExecutableImageMapping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatcherBoundaryBlocker {
    InvalidRuntimeState(GuestRuntimeStateError),
    Unsupported(DispatcherUnsupportedState),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatcherUnsupportedState {
    EntryProgramCounterMismatch {
        expected: GuestProgramCounter,
        actual: GuestProgramCounter,
    },
    TranslationArtifactUnavailable {
        at: GuestProgramCounter,
    },
    RegisterIndirectTarget {
        target: X86Reg,
        at: GuestProgramCounter,
    },
    HelperReturnContinuation {
        at: GuestProgramCounter,
    },
    EntryExecutionUnavailable {
        at: GuestProgramCounter,
    },
    EntryControlFlowContinuationUnavailable {
        at: GuestProgramCounter,
    },
    EntryHelperMaterializationUnavailable {
        at: GuestProgramCounter,
    },
    EntryRegisterMaterializationUnavailable {
        at: GuestProgramCounter,
    },
    EntryStackMaterializationUnavailable {
        at: GuestProgramCounter,
    },
    EntryPhaseUnsupported {
        at: GuestProgramCounter,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelperBoundaryBlocker {
    InvalidReturnState(GuestRuntimeStateError),
    Unsupported(MacOsHostService),
}

#[cfg(test)]
mod tests {
    use bara_ir::{X86Reg, X86Va};

    use crate::{GuestProgramCounter, GuestRuntimeStateError, MacOsHostService};

    use super::{
        DispatcherBoundaryBlocker, DispatcherUnsupportedState, HelperBoundaryBlocker,
        LoaderBoundaryBlocker, LoaderUnsupportedState, RuntimeBoundary, RuntimeBoundaryBlocker,
    };

    #[test]
    fn blockers_preserve_boundary_and_typed_reason() {
        let loader = RuntimeBoundaryBlocker::Loader(LoaderBoundaryBlocker::Unsupported(
            LoaderUnsupportedState::ImportAddressResolution,
        ));
        assert_eq!(loader.boundary(), RuntimeBoundary::Loader);
        assert!(matches!(
            loader,
            RuntimeBoundaryBlocker::Loader(LoaderBoundaryBlocker::Unsupported(
                LoaderUnsupportedState::ImportAddressResolution
            ))
        ));

        let dispatcher =
            RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::Unsupported(
                DispatcherUnsupportedState::RegisterIndirectTarget {
                    target: X86Reg::R14,
                    at: GuestProgramCounter::new(X86Va::new(0x1000)),
                },
            ));
        assert_eq!(dispatcher.boundary(), RuntimeBoundary::Dispatcher);
        assert!(matches!(
            dispatcher,
            RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::Unsupported(
                DispatcherUnsupportedState::RegisterIndirectTarget {
                    target: X86Reg::R14,
                    at,
                }
            )) if at == GuestProgramCounter::new(X86Va::new(0x1000))
        ));

        let helper = RuntimeBoundaryBlocker::Helper(HelperBoundaryBlocker::Unsupported(
            MacOsHostService::AppKitApplicationRun,
        ));
        assert_eq!(helper.boundary(), RuntimeBoundary::Helper);
        assert!(matches!(
            helper,
            RuntimeBoundaryBlocker::Helper(HelperBoundaryBlocker::Unsupported(
                MacOsHostService::AppKitApplicationRun
            ))
        ));

        let invalid_state =
            RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::InvalidRuntimeState(
                GuestRuntimeStateError::DuplicateRegister(X86Reg::Rax),
            ));
        assert!(matches!(
            invalid_state,
            RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::InvalidRuntimeState(
                GuestRuntimeStateError::DuplicateRegister(X86Reg::Rax)
            ))
        ));
    }
}
