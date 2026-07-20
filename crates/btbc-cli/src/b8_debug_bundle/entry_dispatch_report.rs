use bara_runtime::{EntryDispatchOutcome, GuestRuntimePhase, GuestRuntimeState, GuestStackState};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugEntryDispatchReport {
    schema: &'static str,
    outcome: B8DebugEntryDispatchOutcome,
    initial_state: B8DebugGuestRuntimeStateReport,
    final_state: B8DebugGuestRuntimeStateReport,
    next_action: B8DebugEntryDispatchNextAction,
}

impl B8DebugEntryDispatchReport {
    pub(super) fn from_outcome(outcome: &EntryDispatchOutcome) -> Self {
        let (kind, initial_state, final_state, next_action) = match outcome {
            EntryDispatchOutcome::Continue(continued) => (
                B8DebugEntryDispatchOutcome::Continue,
                continued.initial_state(),
                continued.state(),
                B8DebugEntryDispatchNextAction::ConnectDirectContinuationLoop,
            ),
            EntryDispatchOutcome::HelperSuspend(suspended) => (
                B8DebugEntryDispatchOutcome::HelperSuspend,
                suspended.initial_state(),
                suspended.state(),
                B8DebugEntryDispatchNextAction::ConnectMacOsHostService,
            ),
            EntryDispatchOutcome::Return(returned) => (
                B8DebugEntryDispatchOutcome::Return,
                returned.initial_state(),
                returned.state(),
                B8DebugEntryDispatchNextAction::InspectNextDebugBundleBlocker,
            ),
            EntryDispatchOutcome::Blocked(blocked) => (
                B8DebugEntryDispatchOutcome::Blocked,
                blocked.initial_state(),
                blocked.state(),
                B8DebugEntryDispatchNextAction::InspectRuntimeDispatcherBlocker,
            ),
        };
        Self {
            schema: "b8_debug_entry_dispatch_v0",
            outcome: kind,
            initial_state: B8DebugGuestRuntimeStateReport::from_state(initial_state),
            final_state: B8DebugGuestRuntimeStateReport::from_state(final_state),
            next_action,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugEntryDispatchOutcome {
    Continue,
    HelperSuspend,
    Return,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugEntryDispatchNextAction {
    ConnectDirectContinuationLoop,
    ConnectMacOsHostService,
    InspectNextDebugBundleBlocker,
    InspectRuntimeDispatcherBlocker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct B8DebugGuestRuntimeStateReport {
    program_counter: u64,
    phase: B8DebugGuestRuntimePhase,
    stack: B8DebugGuestStackMaterialization,
    rax: Option<u64>,
}

impl B8DebugGuestRuntimeStateReport {
    fn from_state(state: &GuestRuntimeState) -> Self {
        Self {
            program_counter: state.program_counter().address().value(),
            phase: B8DebugGuestRuntimePhase::from_phase(state.phase()),
            stack: B8DebugGuestStackMaterialization::from_stack(state.stack()),
            rax: state
                .registers()
                .value(bara_ir::X86Reg::Rax)
                .map(|value| value.as_guest_address().value()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugGuestRuntimePhase {
    Ready,
    HelperSuspended,
    HelperReturned,
}

impl B8DebugGuestRuntimePhase {
    const fn from_phase(phase: GuestRuntimePhase) -> Self {
        match phase {
            GuestRuntimePhase::Ready => Self::Ready,
            GuestRuntimePhase::HelperSuspended(_) => Self::HelperSuspended,
            GuestRuntimePhase::HelperReturned(_) => Self::HelperReturned,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum B8DebugGuestStackMaterialization {
    Materialized,
    Unmaterialized,
}

impl B8DebugGuestStackMaterialization {
    const fn from_stack(stack: GuestStackState) -> Self {
        match stack {
            GuestStackState::Materialized { .. } => Self::Materialized,
            GuestStackState::Unmaterialized => Self::Unmaterialized,
        }
    }
}
