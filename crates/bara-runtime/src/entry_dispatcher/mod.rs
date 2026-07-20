use bara_arm64::TranslationArtifact;
use bara_ir::X86Reg;

use crate::{
    run_translation_artifact_no_args_u64, DispatcherBoundaryBlocker, DispatcherUnsupportedState,
    GuestRegisterValue, GuestRuntimePhase, GuestRuntimeState, MachOExecutableImagePreparation,
    RunError, RuntimeBoundaryBlocker,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntryDispatchOutcome {
    Continue(EntryDispatchContinuation),
    HelperSuspend(EntryDispatchHelperSuspend),
    Return(EntryDispatchReturn),
    Blocked(EntryDispatchBlocked),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryDispatchContinuation {
    initial_state: GuestRuntimeState,
    state: GuestRuntimeState,
}

impl EntryDispatchContinuation {
    pub const fn state(&self) -> &GuestRuntimeState {
        &self.state
    }

    pub const fn initial_state(&self) -> &GuestRuntimeState {
        &self.initial_state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryDispatchHelperSuspend {
    initial_state: GuestRuntimeState,
    state: GuestRuntimeState,
}

impl EntryDispatchHelperSuspend {
    pub const fn state(&self) -> &GuestRuntimeState {
        &self.state
    }

    pub const fn initial_state(&self) -> &GuestRuntimeState {
        &self.initial_state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryDispatchReturn {
    initial_state: GuestRuntimeState,
    state: GuestRuntimeState,
}

impl EntryDispatchReturn {
    pub const fn state(&self) -> &GuestRuntimeState {
        &self.state
    }

    pub const fn initial_state(&self) -> &GuestRuntimeState {
        &self.initial_state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryDispatchBlocked {
    initial_state: GuestRuntimeState,
    state: GuestRuntimeState,
    blocker: RuntimeBoundaryBlocker,
}

impl EntryDispatchBlocked {
    pub const fn state(&self) -> &GuestRuntimeState {
        &self.state
    }

    pub const fn initial_state(&self) -> &GuestRuntimeState {
        &self.initial_state
    }

    pub const fn blocker(&self) -> RuntimeBoundaryBlocker {
        self.blocker
    }
}

pub fn dispatch_entry_once(
    preparation: &MachOExecutableImagePreparation,
    artifact: &TranslationArtifact,
    initial_state: GuestRuntimeState,
) -> EntryDispatchOutcome {
    let input_state = initial_state.clone();
    let expected = preparation.initial_program_counter();
    let actual = initial_state.program_counter();
    if actual != expected {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::EntryProgramCounterMismatch { expected, actual },
        );
    }

    if initial_state.phase() != GuestRuntimePhase::Ready {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::EntryPhaseUnsupported { at: actual },
        );
    }
    if !initial_state.registers().entries().is_empty() {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::EntryRegisterMaterializationUnavailable { at: actual },
        );
    }
    if initial_state.stack() != crate::GuestStackState::Unmaterialized {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::EntryStackMaterializationUnavailable { at: actual },
        );
    }

    let entry_is_available = artifact
        .emitted_function()
        .pc_map()
        .iter()
        .any(|entry| entry.source() == actual.address());
    if !entry_is_available {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::TranslationArtifactUnavailable { at: actual },
        );
    }

    let emitted = artifact.emitted_function();
    let helper_requests = emitted.host_trap_requests();
    if !emitted.branch_fixups().is_empty() {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::EntryControlFlowContinuationUnavailable { at: actual },
        );
    }
    if helper_requests.stdout_requested() || helper_requests.appkit_gui_hello_world_requested() {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::EntryHelperMaterializationUnavailable { at: actual },
        );
    }

    match run_translation_artifact_no_args_u64(artifact) {
        Ok(result) => {
            let value = std::num::NonZeroU64::new(result.return_value())
                .map(GuestRegisterValue::non_zero_bits)
                .unwrap_or_else(GuestRegisterValue::zero);
            match initial_state.with_register_value(X86Reg::Rax, value) {
                Ok(state) => EntryDispatchOutcome::Return(EntryDispatchReturn {
                    initial_state: input_state,
                    state,
                }),
                Err(error) => EntryDispatchOutcome::Blocked(EntryDispatchBlocked {
                    initial_state: input_state,
                    state: initial_state,
                    blocker: RuntimeBoundaryBlocker::Dispatcher(
                        DispatcherBoundaryBlocker::InvalidRuntimeState(error),
                    ),
                }),
            }
        }
        Err(RunError::ExecutableMemory(_) | RunError::UnsupportedHost) => blocked(
            initial_state,
            DispatcherUnsupportedState::EntryExecutionUnavailable { at: actual },
        ),
    }
}

pub fn dispatch_entry_without_artifact(
    preparation: &MachOExecutableImagePreparation,
    initial_state: GuestRuntimeState,
) -> EntryDispatchOutcome {
    let expected = preparation.initial_program_counter();
    let actual = initial_state.program_counter();
    if actual != expected {
        return blocked(
            initial_state,
            DispatcherUnsupportedState::EntryProgramCounterMismatch { expected, actual },
        );
    }
    blocked(
        initial_state,
        DispatcherUnsupportedState::TranslationArtifactUnavailable { at: actual },
    )
}

fn blocked(
    state: GuestRuntimeState,
    unsupported: DispatcherUnsupportedState,
) -> EntryDispatchOutcome {
    let initial_state = state.clone();
    EntryDispatchOutcome::Blocked(EntryDispatchBlocked {
        initial_state,
        state,
        blocker: RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::Unsupported(
            unsupported,
        )),
    })
}

#[cfg(test)]
mod tests;
