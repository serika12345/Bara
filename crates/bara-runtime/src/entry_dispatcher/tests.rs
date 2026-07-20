use std::str::FromStr;

use bara_arm64::{
    Arm64MachineCode, ArmPc, EmittedFunction, PcMapEntry, TranslationArtifact,
    TranslationCacheIdentity, TranslationSourceHash, TranslationSourceIdentity, TranslationTarget,
    TranslatorVersion,
};
use bara_ir::{
    ProgramImageMappedByteSegment, ProgramImageMappedBytes, ProgramImageMetadata,
    ProgramImageRange, X86Reg, X86Va,
};

use crate::{
    DispatcherBoundaryBlocker, DispatcherUnsupportedState, GuestHelperSuspendState,
    GuestImageMetadata, GuestProgramCounter, GuestRegisterState, GuestRegisterStateEntry,
    GuestRegisterValue, GuestRuntimePhase, GuestRuntimeState, GuestStackBounds, GuestStackPointer,
    GuestStackState, MachOExecutableCodeRange, MachOExecutableEntryPoint,
    MachOExecutableImagePreparation, MachOImage,
};

use super::{dispatch_entry_once, dispatch_entry_without_artifact, EntryDispatchOutcome};

#[test]
fn entry_dispatcher_executes_return_and_preserves_typed_final_state() {
    let preparation = preparation();
    let initial_state = ready_state(preparation.initial_program_counter());

    let outcome = dispatch_entry_once(&preparation, &return_42_artifact(), initial_state);

    assert!(matches!(outcome, EntryDispatchOutcome::Return(_)));
    if let EntryDispatchOutcome::Return(returned) = outcome {
        assert_eq!(
            returned.state().program_counter(),
            preparation.initial_program_counter()
        );
        assert_eq!(
            returned.state().registers().value(X86Reg::Rax),
            Some(crate::GuestRegisterValue::non_zero_bits(
                std::num::NonZeroU64::new(42).expect("42 is non-zero")
            ))
        );
    }
}

#[test]
fn entry_dispatcher_blocks_mismatched_entry_without_executing() {
    let preparation = preparation();
    let wrong_entry = GuestProgramCounter::new(X86Va::new(0x1_0000_0008));

    let outcome = dispatch_entry_once(
        &preparation,
        &return_42_artifact(),
        ready_state(wrong_entry),
    );

    assert!(matches!(outcome, EntryDispatchOutcome::Blocked(_)));
}

#[test]
fn entry_dispatcher_rejects_a_phase_that_the_entry_spine_cannot_materialize() {
    let preparation = preparation();
    let entry = preparation.initial_program_counter();
    let return_to = GuestProgramCounter::new(X86Va::new(entry.address().value() + 4));
    let suspended = GuestHelperSuspendState::new(entry, return_to).expect("range is valid");
    let mut initial_state = ready_state(entry);
    initial_state = GuestRuntimeState::new(
        entry,
        initial_state.registers().clone(),
        initial_state.stack(),
        GuestRuntimePhase::HelperSuspended(suspended),
    )
    .expect("suspended state is valid");

    let outcome = dispatch_entry_once(&preparation, &return_42_artifact(), initial_state.clone());

    assert!(matches!(
        outcome,
        EntryDispatchOutcome::Blocked(blocked)
            if blocked.blocker()
                == crate::RuntimeBoundaryBlocker::Dispatcher(
                    DispatcherBoundaryBlocker::Unsupported(
                        DispatcherUnsupportedState::EntryPhaseUnsupported { at: entry }
                    )
                )
    ));
}

#[test]
fn entry_dispatcher_rejects_registers_and_stack_it_cannot_materialize() {
    let preparation = preparation();
    let entry = preparation.initial_program_counter();
    let registers = GuestRegisterState::from_entries([GuestRegisterStateEntry::new(
        X86Reg::Rdi,
        GuestRegisterValue::zero(),
    )])
    .expect("test register state is valid");
    let with_register = GuestRuntimeState::new(
        entry,
        registers,
        GuestStackState::unmaterialized(),
        GuestRuntimePhase::Ready,
    )
    .expect("test state is valid");
    assert!(matches!(
        dispatch_entry_once(&preparation, &return_42_artifact(), with_register),
        EntryDispatchOutcome::Blocked(blocked)
            if blocked.blocker()
                == crate::RuntimeBoundaryBlocker::Dispatcher(
                    DispatcherBoundaryBlocker::Unsupported(
                        DispatcherUnsupportedState::EntryRegisterMaterializationUnavailable { at: entry }
                    )
                )
    ));

    let materialized_stack = GuestRuntimeState::new(
        entry,
        GuestRegisterState::empty(),
        materialized_stack(),
        GuestRuntimePhase::Ready,
    )
    .expect("test state is valid");
    assert!(matches!(
        dispatch_entry_once(&preparation, &return_42_artifact(), materialized_stack),
        EntryDispatchOutcome::Blocked(blocked)
            if blocked.blocker()
                == crate::RuntimeBoundaryBlocker::Dispatcher(
                    DispatcherBoundaryBlocker::Unsupported(
                        DispatcherUnsupportedState::EntryStackMaterializationUnavailable { at: entry }
                    )
                )
    ));
}

#[test]
fn entry_dispatcher_blocks_when_artifact_has_no_entry_mapping() {
    let preparation = preparation();
    let entry = preparation.initial_program_counter();
    let artifact = artifact_with_pc_map(X86Va::new(entry.address().value() + 4));

    let outcome = dispatch_entry_once(&preparation, &artifact, ready_state(entry));

    assert!(matches!(
        outcome,
        EntryDispatchOutcome::Blocked(blocked)
            if blocked.blocker()
                == crate::RuntimeBoundaryBlocker::Dispatcher(
                    DispatcherBoundaryBlocker::Unsupported(
                        DispatcherUnsupportedState::TranslationArtifactUnavailable { at: entry }
                    )
                )
    ));
}

#[test]
fn entry_dispatcher_preserves_the_entry_state_when_translation_is_unavailable() {
    let preparation = preparation();
    let entry = preparation.initial_program_counter();
    let initial_state = ready_state(entry);

    let outcome = dispatch_entry_without_artifact(&preparation, initial_state.clone());

    assert!(matches!(
        outcome,
        EntryDispatchOutcome::Blocked(blocked)
            if blocked.initial_state() == &initial_state
                && blocked.state() == &initial_state
                && blocked.blocker()
                    == crate::RuntimeBoundaryBlocker::Dispatcher(
                        DispatcherBoundaryBlocker::Unsupported(
                            DispatcherUnsupportedState::TranslationArtifactUnavailable { at: entry }
                        )
                    )
    ));
}

fn preparation() -> MachOExecutableImagePreparation {
    let range = ProgramImageRange::new(X86Va::new(0x1_0000_0000), X86Va::new(0x1_0000_0010))
        .expect("test range is valid");
    let mapped = ProgramImageMappedByteSegment::new(range, vec![0x90; 0x10])
        .expect("mapped bytes cover range");
    let program_metadata = ProgramImageMetadata::new_with_mapped_bytes(
        Default::default(),
        ProgramImageMappedBytes::from_segments([mapped]),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let metadata = GuestImageMetadata::from_program_image_metadata(&program_metadata);
    let image = MachOImage::executable_from_code_range(
        MachOExecutableEntryPoint::new(range.start()),
        MachOExecutableCodeRange::new(range),
        metadata,
    )
    .expect("entry is in code range");
    MachOExecutableImagePreparation::try_from_snapshot(image.executable_snapshot())
        .expect("image is statically prepared")
}

fn ready_state(program_counter: GuestProgramCounter) -> GuestRuntimeState {
    GuestRuntimeState::new(
        program_counter,
        GuestRegisterState::empty(),
        GuestStackState::unmaterialized(),
        GuestRuntimePhase::Ready,
    )
    .expect("ready state is valid")
}

fn materialized_stack() -> GuestStackState {
    let low = GuestStackPointer::new(X86Va::new(0x7000));
    let high = GuestStackPointer::new(X86Va::new(0x8000));
    let bounds = GuestStackBounds::new(low, high).expect("stack bounds are valid");
    GuestStackState::new(high, bounds).expect("stack pointer is in bounds")
}

fn return_42_artifact() -> TranslationArtifact {
    artifact_with_pc_map(X86Va::new(0x1_0000_0000))
}

fn artifact_with_pc_map(source: X86Va) -> TranslationArtifact {
    let hash = TranslationSourceHash::from_str(&"11".repeat(32)).expect("hash is valid");
    let emitted = EmittedFunction::new(
        Arm64MachineCode::new(vec![0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6])
            .expect("ARM64 code is valid"),
        vec![PcMapEntry::new(source, ArmPc::new(0))],
    );
    TranslationArtifact::new(
        TranslationSourceIdentity::new(hash),
        emitted,
        TranslationCacheIdentity::new(
            hash,
            TranslatorVersion::current(),
            TranslationTarget::Arm64MacOs,
        ),
    )
    .expect("artifact is valid")
}
