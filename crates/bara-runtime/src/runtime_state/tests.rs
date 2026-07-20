use std::num::NonZeroU64;

use bara_ir::{X86Reg, X86Va};

use super::{
    GuestHelperReturnState, GuestHelperSuspendState, GuestProgramCounter, GuestRegisterState,
    GuestRegisterStateEntry, GuestRegisterValue, GuestRuntimePhase, GuestRuntimeState,
    GuestRuntimeStateError, GuestStackBounds, GuestStackPointer, GuestStackState,
};

fn pc(address: u64) -> GuestProgramCounter {
    GuestProgramCounter::new(X86Va::new(address))
}

fn stack() -> GuestStackState {
    GuestStackState::new(
        GuestStackPointer::new(X86Va::new(0x2ff0)),
        GuestStackBounds::new(
            GuestStackPointer::new(X86Va::new(0x2000)),
            GuestStackPointer::new(X86Va::new(0x3000)),
        )
        .expect("stack bounds are ordered"),
    )
    .expect("stack pointer is inside its bounds")
}

#[test]
fn runtime_state_constructs_running_and_helper_boundary_phases() {
    let registers = GuestRegisterState::from_entries([
        GuestRegisterStateEntry::new(X86Reg::Rax, GuestRegisterValue::zero()),
        GuestRegisterStateEntry::new(
            X86Reg::Rdi,
            GuestRegisterValue::guest_address(X86Va::new(0x4000)),
        ),
    ])
    .expect("registers are unique full-width values");
    let ready = GuestRuntimeState::new(
        pc(0x1000),
        registers.clone(),
        stack(),
        GuestRuntimePhase::Ready,
    )
    .expect("ready state has no phase-specific PC invariant");

    assert_eq!(ready.program_counter(), pc(0x1000));
    assert_eq!(ready.registers(), &registers);
    assert_eq!(ready.phase(), GuestRuntimePhase::Ready);

    let suspended = GuestHelperSuspendState::new(pc(0x1000), pc(0x1005))
        .expect("helper call has a forward return PC");
    let suspended_state = GuestRuntimeState::new(
        pc(0x1000),
        registers.clone(),
        stack(),
        GuestRuntimePhase::HelperSuspended(suspended),
    )
    .expect("suspended state is located at its call site");
    assert_eq!(
        suspended_state.phase(),
        GuestRuntimePhase::HelperSuspended(suspended)
    );

    let returned = GuestHelperReturnState::from_suspend(suspended);
    let returned_state = GuestRuntimeState::new(
        pc(0x1005),
        registers,
        stack(),
        GuestRuntimePhase::HelperReturned(returned),
    )
    .expect("returned state is located at its continuation");
    assert_eq!(
        returned_state.phase(),
        GuestRuntimePhase::HelperReturned(returned)
    );
}

#[test]
fn register_state_rejects_partial_and_duplicate_registers() {
    assert_eq!(
        GuestRegisterState::from_entries([GuestRegisterStateEntry::new(
            X86Reg::Eax,
            GuestRegisterValue::zero(),
        )]),
        Err(GuestRuntimeStateError::PartialRegisterUnsupported(
            X86Reg::Eax
        ))
    );

    assert_eq!(
        GuestRegisterState::from_entries([
            GuestRegisterStateEntry::new(X86Reg::Rax, GuestRegisterValue::zero()),
            GuestRegisterStateEntry::new(
                X86Reg::Rax,
                GuestRegisterValue::guest_address(X86Va::new(1)),
            ),
        ]),
        Err(GuestRuntimeStateError::DuplicateRegister(X86Reg::Rax))
    );
}

#[test]
fn register_value_represents_every_non_zero_bit_pattern_without_a_raw_public_primitive() {
    let bits = NonZeroU64::new(0xffff_ffff_ffff_fffe).expect("test value is non-zero");

    assert_eq!(
        GuestRegisterValue::non_zero_bits(bits).as_non_zero_bits(),
        Some(bits)
    );
    assert_eq!(GuestRegisterValue::zero().as_non_zero_bits(), None);
}

#[test]
fn stack_and_helper_phase_validation_return_typed_errors() {
    assert_eq!(
        GuestStackBounds::new(
            GuestStackPointer::new(X86Va::new(0x3000)),
            GuestStackPointer::new(X86Va::new(0x2000)),
        ),
        Err(GuestRuntimeStateError::EmptyOrReversedStackBounds)
    );
    let bounds = GuestStackBounds::new(
        GuestStackPointer::new(X86Va::new(0x2000)),
        GuestStackPointer::new(X86Va::new(0x3000)),
    )
    .expect("stack bounds are ordered");
    assert_eq!(
        GuestStackState::new(GuestStackPointer::new(X86Va::new(0x1ff8)), bounds),
        Err(GuestRuntimeStateError::StackPointerOutsideBounds)
    );

    let suspended = GuestHelperSuspendState::new(pc(0x1000), pc(0x1005))
        .expect("helper call has a forward return PC");
    assert_eq!(
        GuestHelperSuspendState::new(pc(0x1005), pc(0x1000)),
        Err(GuestRuntimeStateError::EmptyOrReversedHelperReturnRange)
    );
    assert_eq!(
        GuestRuntimeState::new(
            pc(0x1001),
            GuestRegisterState::empty(),
            stack(),
            GuestRuntimePhase::HelperSuspended(suspended),
        ),
        Err(GuestRuntimeStateError::ProgramCounterDoesNotMatchHelperCallSite)
    );
    assert_eq!(
        GuestRuntimeState::new(
            pc(0x1004),
            GuestRegisterState::empty(),
            stack(),
            GuestRuntimePhase::HelperReturned(GuestHelperReturnState::from_suspend(suspended)),
        ),
        Err(GuestRuntimeStateError::ProgramCounterDoesNotMatchHelperReturn)
    );
}

#[test]
fn runtime_state_can_preserve_an_explicitly_unmaterialized_guest_stack() {
    let state = GuestRuntimeState::new(
        pc(0x1000),
        GuestRegisterState::empty(),
        GuestStackState::unmaterialized(),
        GuestRuntimePhase::Ready,
    )
    .expect("an unmaterialized stack is an explicit pre-process-state phase");

    assert_eq!(state.stack(), GuestStackState::Unmaterialized);
    assert_eq!(state.stack().pointer(), None);
    assert_eq!(state.stack().bounds(), None);
}
