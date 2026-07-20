use bara_runtime::{
    DirectContinuationDispatchOutcome, DispatcherBoundaryBlocker, DispatcherUnsupportedState,
    RuntimeBoundaryBlocker,
};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugDirectContinuationReport {
    schema: &'static str,
    outcome: B8DebugDirectContinuationOutcome,
    executed_blocks: usize,
    final_program_counter: u64,
    blocker: Option<B8DebugDirectContinuationBlocker>,
}

impl B8DebugDirectContinuationReport {
    pub(super) fn from_outcome(outcome: &DirectContinuationDispatchOutcome) -> Self {
        match outcome {
            DirectContinuationDispatchOutcome::Return(returned) => Self {
                schema: "b8_debug_direct_continuation_v0",
                outcome: B8DebugDirectContinuationOutcome::Return,
                executed_blocks: returned
                    .executed_blocks()
                    .non_zero()
                    .map_or(0, std::num::NonZeroUsize::get),
                final_program_counter: returned.state().program_counter().address().value(),
                blocker: None,
            },
            DirectContinuationDispatchOutcome::Blocked(blocked) => Self {
                schema: "b8_debug_direct_continuation_v0",
                outcome: B8DebugDirectContinuationOutcome::Blocked,
                executed_blocks: blocked
                    .executed_blocks()
                    .non_zero()
                    .map_or(0, std::num::NonZeroUsize::get),
                final_program_counter: blocked.state().program_counter().address().value(),
                blocker: Some(B8DebugDirectContinuationBlocker::from_runtime(
                    blocked.blocker(),
                )),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugDirectContinuationOutcome {
    Return,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugDirectContinuationBlocker {
    BudgetExhausted,
    UnknownDirectTarget,
    RegisterIndirectTarget,
    DirectCallUnavailable,
    ResolvedEntryMismatch,
    ExecutionUnavailable,
    InvalidRuntimeState,
    UnsupportedRuntimeBoundary,
}

impl B8DebugDirectContinuationBlocker {
    const fn from_runtime(blocker: RuntimeBoundaryBlocker) -> Self {
        match blocker {
            RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::Unsupported(reason)) => {
                match reason {
                    DispatcherUnsupportedState::ExecutionBudgetExhausted { .. } => {
                        Self::BudgetExhausted
                    }
                    DispatcherUnsupportedState::UnknownDirectTarget { .. } => {
                        Self::UnknownDirectTarget
                    }
                    DispatcherUnsupportedState::RegisterIndirectTarget { .. } => {
                        Self::RegisterIndirectTarget
                    }
                    DispatcherUnsupportedState::DirectCallContinuationUnavailable { .. } => {
                        Self::DirectCallUnavailable
                    }
                    DispatcherUnsupportedState::ResolvedBlockEntryMismatch { .. } => {
                        Self::ResolvedEntryMismatch
                    }
                    DispatcherUnsupportedState::EntryExecutionUnavailable { .. } => {
                        Self::ExecutionUnavailable
                    }
                    _ => Self::UnsupportedRuntimeBoundary,
                }
            }
            RuntimeBoundaryBlocker::Dispatcher(DispatcherBoundaryBlocker::InvalidRuntimeState(
                _,
            )) => Self::InvalidRuntimeState,
            RuntimeBoundaryBlocker::Loader(_) | RuntimeBoundaryBlocker::Helper(_) => {
                Self::UnsupportedRuntimeBoundary
            }
        }
    }
}
