use std::num::NonZeroUsize;

use bara_arm64::{ArmPc, TranslationArtifact};
use bara_ir::X86Reg;

use crate::{
    run_translation_artifact_no_args_u64, DispatcherBoundaryBlocker, DispatcherUnsupportedState,
    GuestProgramCounter, GuestRegisterValue, GuestRuntimePhase, GuestRuntimeState,
    GuestRuntimeStateError, GuestStackState, RunError, RuntimeBoundaryBlocker,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectBlockExit {
    Fallthrough(GuestProgramCounter),
    ReturnToCaller,
    RegisterIndirect(X86Reg),
    DirectCall(GuestProgramCounter),
}

impl DirectBlockExit {
    pub const fn fallthrough(target: GuestProgramCounter) -> Self {
        Self::Fallthrough(target)
    }

    pub const fn return_to_caller() -> Self {
        Self::ReturnToCaller
    }

    pub const fn register_indirect(target: X86Reg) -> Self {
        Self::RegisterIndirect(target)
    }

    pub const fn direct_call(target: GuestProgramCounter) -> Self {
        Self::DirectCall(target)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectBlockInputContract {
    NoRegisterOrStackLiveIns,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectContinuationBlock {
    entry: GuestProgramCounter,
    artifact: TranslationArtifact,
    exit: DirectBlockExit,
}

impl DirectContinuationBlock {
    pub fn new(
        entry: GuestProgramCounter,
        artifact: TranslationArtifact,
        exit: DirectBlockExit,
        _input_contract: DirectBlockInputContract,
    ) -> Result<Self, DirectContinuationBlockError> {
        let emitted = artifact.emitted_function();
        if !emitted
            .pc_map()
            .iter()
            .any(|mapping| mapping.source() == entry.address() && mapping.target() == ArmPc::new(0))
        {
            return Err(DirectContinuationBlockError::ArtifactEntryMappingUnavailable);
        }
        if !emitted.branch_fixups().is_empty() {
            return Err(DirectContinuationBlockError::UnresolvedBranchFixup);
        }
        let helpers = emitted.host_trap_requests();
        if helpers.stdout_requested() || helpers.appkit_gui_hello_world_requested() {
            return Err(DirectContinuationBlockError::HelperRequestUnsupported);
        }
        Ok(Self {
            entry,
            artifact,
            exit,
        })
    }

    pub const fn entry(&self) -> GuestProgramCounter {
        self.entry
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectContinuationBlockError {
    ArtifactEntryMappingUnavailable,
    UnresolvedBranchFixup,
    HelperRequestUnsupported,
}

pub trait DirectContinuationArtifactProvider {
    fn resolve_block(&mut self, at: GuestProgramCounter) -> Option<DirectContinuationBlock>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectExecutionBudget {
    blocks: NonZeroUsize,
}

impl DirectExecutionBudget {
    pub const fn new(blocks: NonZeroUsize) -> Self {
        Self { blocks }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutedBlockCount {
    blocks: usize,
}

impl ExecutedBlockCount {
    const fn zero() -> Self {
        Self { blocks: 0 }
    }

    const fn incremented(self) -> Self {
        Self {
            blocks: self.blocks + 1,
        }
    }

    pub const fn non_zero(self) -> Option<NonZeroUsize> {
        NonZeroUsize::new(self.blocks)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectContinuationDispatchOutcome {
    Return(DirectContinuationReturn),
    Blocked(DirectContinuationBlocked),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectContinuationReturn {
    state: GuestRuntimeState,
    executed_blocks: ExecutedBlockCount,
}

impl DirectContinuationReturn {
    pub const fn state(&self) -> &GuestRuntimeState {
        &self.state
    }

    pub const fn executed_blocks(&self) -> ExecutedBlockCount {
        self.executed_blocks
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectContinuationBlocked {
    state: GuestRuntimeState,
    executed_blocks: ExecutedBlockCount,
    blocker: RuntimeBoundaryBlocker,
}

impl DirectContinuationBlocked {
    pub const fn state(&self) -> &GuestRuntimeState {
        &self.state
    }

    pub const fn executed_blocks(&self) -> ExecutedBlockCount {
        self.executed_blocks
    }

    pub const fn blocker(&self) -> RuntimeBoundaryBlocker {
        self.blocker
    }
}

pub fn dispatch_direct_continuations(
    provider: &mut impl DirectContinuationArtifactProvider,
    initial_state: GuestRuntimeState,
    budget: DirectExecutionBudget,
) -> DirectContinuationDispatchOutcome {
    let initial_program_counter = initial_state.program_counter();
    let mut state = initial_state;
    let mut remaining = budget.blocks.get();
    let mut executed = ExecutedBlockCount::zero();

    if state.phase() != GuestRuntimePhase::Ready || state.stack() != GuestStackState::Unmaterialized
    {
        return blocked(
            state,
            executed,
            DispatcherUnsupportedState::EntryPhaseUnsupported {
                at: initial_program_counter,
            },
        );
    }

    loop {
        let at = state.program_counter();
        let Some(block) = provider.resolve_block(at) else {
            return blocked(
                state,
                executed,
                DispatcherUnsupportedState::UnknownDirectTarget { at },
            );
        };
        if block.entry() != at {
            return blocked(
                state,
                executed,
                DispatcherUnsupportedState::ResolvedBlockEntryMismatch {
                    requested: at,
                    resolved: block.entry(),
                },
            );
        }
        let result = match run_translation_artifact_no_args_u64(&block.artifact) {
            Ok(result) => result,
            Err(RunError::ExecutableMemory(_) | RunError::UnsupportedHost) => {
                return blocked(
                    state,
                    executed,
                    DispatcherUnsupportedState::EntryExecutionUnavailable { at },
                );
            }
        };
        let value = std::num::NonZeroU64::new(result.return_value())
            .map(GuestRegisterValue::non_zero_bits)
            .unwrap_or_else(GuestRegisterValue::zero);
        state = match state.with_register_value(X86Reg::Rax, value) {
            Ok(state) => state,
            Err(error) => return invalid_state(state, executed, error),
        };
        executed = executed.incremented();
        remaining -= 1;

        match block.exit {
            DirectBlockExit::ReturnToCaller => {
                return DirectContinuationDispatchOutcome::Return(DirectContinuationReturn {
                    state,
                    executed_blocks: executed,
                });
            }
            DirectBlockExit::Fallthrough(target) => {
                state = match state.with_program_counter(target) {
                    Ok(state) => state,
                    Err(error) => return invalid_state(state, executed, error),
                };
                if remaining == 0 {
                    return blocked(
                        state,
                        executed,
                        DispatcherUnsupportedState::ExecutionBudgetExhausted { at: target },
                    );
                }
            }
            DirectBlockExit::RegisterIndirect(target) => {
                return blocked(
                    state,
                    executed,
                    DispatcherUnsupportedState::RegisterIndirectTarget { target, at },
                );
            }
            DirectBlockExit::DirectCall(target) => {
                return blocked(
                    state,
                    executed,
                    DispatcherUnsupportedState::DirectCallContinuationUnavailable { target, at },
                );
            }
        }
    }
}

fn blocked(
    state: GuestRuntimeState,
    executed_blocks: ExecutedBlockCount,
    unsupported: DispatcherUnsupportedState,
) -> DirectContinuationDispatchOutcome {
    DirectContinuationDispatchOutcome::Blocked(DirectContinuationBlocked {
        state,
        executed_blocks,
        blocker: RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::Unsupported(
            unsupported,
        )),
    })
}

fn invalid_state(
    state: GuestRuntimeState,
    executed_blocks: ExecutedBlockCount,
    error: GuestRuntimeStateError,
) -> DirectContinuationDispatchOutcome {
    DirectContinuationDispatchOutcome::Blocked(DirectContinuationBlocked {
        state,
        executed_blocks,
        blocker: RuntimeBoundaryBlocker::Dispatcher(
            DispatcherBoundaryBlocker::InvalidRuntimeState(error),
        ),
    })
}

#[cfg(test)]
mod tests;
