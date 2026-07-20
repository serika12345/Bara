use std::{num::NonZeroUsize, str::FromStr};

use bara_arm64::{
    Arm64MachineCode, ArmPc, EmittedFunction, PcMapEntry, TranslationArtifact,
    TranslationCacheIdentity, TranslationSourceHash, TranslationSourceIdentity, TranslationTarget,
    TranslatorVersion,
};
use bara_ir::{X86Reg, X86Va};

use crate::{
    GuestProgramCounter, GuestRegisterState, GuestRuntimePhase, GuestRuntimeState, GuestStackState,
};

use super::{
    dispatch_direct_continuations, DirectBlockExit, DirectBlockInputContract,
    DirectContinuationArtifactProvider, DirectContinuationBlock, DirectContinuationDispatchOutcome,
    DirectExecutionBudget,
};

struct Blocks(Vec<DirectContinuationBlock>);

impl DirectContinuationArtifactProvider for Blocks {
    fn resolve_block(&mut self, at: GuestProgramCounter) -> Option<DirectContinuationBlock> {
        self.0.iter().find(|block| block.entry() == at).cloned()
    }
}

#[test]
fn direct_continuation_executes_two_blocks_and_returns() {
    let first = pc(0x1000);
    let second = pc(0x1010);
    let mut blocks = Blocks(vec![
        block(first, 1, DirectBlockExit::fallthrough(second)),
        block(second, 42, DirectBlockExit::return_to_caller()),
    ]);

    let outcome = dispatch_direct_continuations(
        &mut blocks,
        ready_state(first),
        DirectExecutionBudget::new(NonZeroUsize::new(2).expect("two is non-zero")),
    );

    assert!(matches!(
        outcome,
        DirectContinuationDispatchOutcome::Return(returned)
            if returned.executed_blocks().non_zero().is_some_and(|count| count.get() == 2)
                && returned.state().registers().value(X86Reg::Rax)
                    == Some(crate::GuestRegisterValue::non_zero_bits(
                        std::num::NonZeroU64::new(42).expect("42 is non-zero")
                    ))
    ));
}

#[test]
fn direct_continuation_blocks_on_budget_exhaustion_before_next_block() {
    let first = pc(0x1000);
    let second = pc(0x1010);
    let mut blocks = Blocks(vec![block(first, 1, DirectBlockExit::fallthrough(second))]);

    let outcome = dispatch_direct_continuations(
        &mut blocks,
        ready_state(first),
        DirectExecutionBudget::new(NonZeroUsize::new(1).expect("one is non-zero")),
    );

    assert!(matches!(
        outcome,
        DirectContinuationDispatchOutcome::Blocked(blocked)
            if matches!(
                blocked.blocker(),
                crate::RuntimeBoundaryBlocker::Dispatcher(
                    crate::DispatcherBoundaryBlocker::Unsupported(
                        crate::DispatcherUnsupportedState::ExecutionBudgetExhausted { at }
                    )
                ) if at == second
            )
    ));
}

#[test]
fn direct_continuation_blocks_on_unknown_and_indirect_targets() {
    let first = pc(0x1000);
    let missing = pc(0x1010);
    let mut unknown = Blocks(vec![block(first, 1, DirectBlockExit::fallthrough(missing))]);
    let budget = || DirectExecutionBudget::new(NonZeroUsize::new(2).expect("two is non-zero"));

    assert!(matches!(
        dispatch_direct_continuations(&mut unknown, ready_state(first), budget()),
        DirectContinuationDispatchOutcome::Blocked(blocked)
            if matches!(blocked.blocker(), crate::RuntimeBoundaryBlocker::Dispatcher(
                crate::DispatcherBoundaryBlocker::Unsupported(
                    crate::DispatcherUnsupportedState::UnknownDirectTarget { at }
                )) if at == missing)
    ));

    let mut indirect = Blocks(vec![block(
        first,
        1,
        DirectBlockExit::register_indirect(X86Reg::R14),
    )]);
    assert!(matches!(
        dispatch_direct_continuations(&mut indirect, ready_state(first), budget()),
        DirectContinuationDispatchOutcome::Blocked(blocked)
            if matches!(blocked.blocker(), crate::RuntimeBoundaryBlocker::Dispatcher(
                crate::DispatcherBoundaryBlocker::Unsupported(
                    crate::DispatcherUnsupportedState::RegisterIndirectTarget {
                        target: X86Reg::R14,
                        at
                    }
                )) if at == first)
    ));
}

fn pc(value: u64) -> GuestProgramCounter {
    GuestProgramCounter::new(X86Va::new(value))
}

fn ready_state(at: GuestProgramCounter) -> GuestRuntimeState {
    GuestRuntimeState::new(
        at,
        GuestRegisterState::empty(),
        GuestStackState::unmaterialized(),
        GuestRuntimePhase::Ready,
    )
    .expect("ready state is valid")
}

fn block(entry: GuestProgramCounter, value: u16, exit: DirectBlockExit) -> DirectContinuationBlock {
    let hash = TranslationSourceHash::from_str(&"22".repeat(32)).expect("hash is valid");
    let low = value << 5;
    let movz = 0xd2800000_u32 | u32::from(low);
    let emitted = EmittedFunction::new(
        Arm64MachineCode::new([movz.to_le_bytes(), 0xd65f03c0_u32.to_le_bytes()].concat())
            .expect("ARM64 block fragment is valid"),
        vec![PcMapEntry::new(entry.address(), ArmPc::new(0))],
    );
    let artifact = TranslationArtifact::new(
        TranslationSourceIdentity::new(hash),
        emitted,
        TranslationCacheIdentity::new(
            hash,
            TranslatorVersion::current(),
            TranslationTarget::Arm64MacOs,
        ),
    )
    .expect("artifact is valid");
    DirectContinuationBlock::new(
        entry,
        artifact,
        exit,
        DirectBlockInputContract::NoRegisterOrStackLiveIns,
    )
    .expect("test artifact is a standalone block fragment")
}
