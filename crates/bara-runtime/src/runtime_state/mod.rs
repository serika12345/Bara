use std::num::NonZeroU64;

use bara_ir::{X86Reg, X86Va};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestProgramCounter {
    address: X86Va,
}

impl GuestProgramCounter {
    pub const fn new(address: X86Va) -> Self {
        Self { address }
    }

    pub const fn address(self) -> X86Va {
        self.address
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestRegisterValue {
    bits: u64,
}

impl GuestRegisterValue {
    pub const fn zero() -> Self {
        Self { bits: 0 }
    }

    pub const fn non_zero_bits(bits: NonZeroU64) -> Self {
        Self { bits: bits.get() }
    }

    pub const fn guest_address(address: X86Va) -> Self {
        Self {
            bits: address.value(),
        }
    }

    pub const fn as_non_zero_bits(self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.bits)
    }

    pub const fn as_guest_address(self) -> X86Va {
        X86Va::new(self.bits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestRegisterStateEntry {
    register: X86Reg,
    value: GuestRegisterValue,
}

impl GuestRegisterStateEntry {
    pub const fn new(register: X86Reg, value: GuestRegisterValue) -> Self {
        Self { register, value }
    }

    pub const fn register(self) -> X86Reg {
        self.register
    }

    pub const fn value(self) -> GuestRegisterValue {
        self.value
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GuestRegisterState {
    entries: Vec<GuestRegisterStateEntry>,
}

impl GuestRegisterState {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_entries(
        entries: impl IntoIterator<Item = GuestRegisterStateEntry>,
    ) -> Result<Self, GuestRuntimeStateError> {
        let mut validated = Vec::new();
        for entry in entries {
            let register = entry.register();
            if register.is_partial_view() {
                return Err(GuestRuntimeStateError::PartialRegisterUnsupported(register));
            }
            if validated
                .iter()
                .any(|existing: &GuestRegisterStateEntry| existing.register() == register)
            {
                return Err(GuestRuntimeStateError::DuplicateRegister(register));
            }
            validated.push(entry);
        }

        Ok(Self { entries: validated })
    }

    pub fn entries(&self) -> &[GuestRegisterStateEntry] {
        &self.entries
    }

    pub fn value(&self, register: X86Reg) -> Option<GuestRegisterValue> {
        self.entries
            .iter()
            .copied()
            .find(|entry| entry.register() == register)
            .map(GuestRegisterStateEntry::value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestStackPointer {
    address: X86Va,
}

impl GuestStackPointer {
    pub const fn new(address: X86Va) -> Self {
        Self { address }
    }

    pub const fn address(self) -> X86Va {
        self.address
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestStackBounds {
    low: GuestStackPointer,
    high: GuestStackPointer,
}

impl GuestStackBounds {
    pub fn new(
        low: GuestStackPointer,
        high: GuestStackPointer,
    ) -> Result<Self, GuestRuntimeStateError> {
        if low.address() >= high.address() {
            return Err(GuestRuntimeStateError::EmptyOrReversedStackBounds);
        }

        Ok(Self { low, high })
    }

    pub const fn low(self) -> GuestStackPointer {
        self.low
    }

    pub const fn high(self) -> GuestStackPointer {
        self.high
    }

    const fn contains(self, pointer: GuestStackPointer) -> bool {
        self.low.address().value() <= pointer.address().value()
            && pointer.address().value() <= self.high.address().value()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestStackState {
    pointer: GuestStackPointer,
    bounds: GuestStackBounds,
}

impl GuestStackState {
    pub fn new(
        pointer: GuestStackPointer,
        bounds: GuestStackBounds,
    ) -> Result<Self, GuestRuntimeStateError> {
        if !bounds.contains(pointer) {
            return Err(GuestRuntimeStateError::StackPointerOutsideBounds);
        }

        Ok(Self { pointer, bounds })
    }

    pub const fn pointer(self) -> GuestStackPointer {
        self.pointer
    }

    pub const fn bounds(self) -> GuestStackBounds {
        self.bounds
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestHelperSuspendState {
    call_site: GuestProgramCounter,
    return_to: GuestProgramCounter,
}

impl GuestHelperSuspendState {
    pub fn new(
        call_site: GuestProgramCounter,
        return_to: GuestProgramCounter,
    ) -> Result<Self, GuestRuntimeStateError> {
        if call_site.address() >= return_to.address() {
            return Err(GuestRuntimeStateError::EmptyOrReversedHelperReturnRange);
        }

        Ok(Self {
            call_site,
            return_to,
        })
    }

    pub const fn call_site(self) -> GuestProgramCounter {
        self.call_site
    }

    pub const fn return_to(self) -> GuestProgramCounter {
        self.return_to
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestHelperReturnState {
    resume_at: GuestProgramCounter,
}

impl GuestHelperReturnState {
    pub const fn from_suspend(suspended: GuestHelperSuspendState) -> Self {
        Self {
            resume_at: suspended.return_to(),
        }
    }

    pub const fn resume_at(self) -> GuestProgramCounter {
        self.resume_at
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestRuntimePhase {
    Ready,
    HelperSuspended(GuestHelperSuspendState),
    HelperReturned(GuestHelperReturnState),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestRuntimeState {
    program_counter: GuestProgramCounter,
    registers: GuestRegisterState,
    stack: GuestStackState,
    phase: GuestRuntimePhase,
}

impl GuestRuntimeState {
    pub fn new(
        program_counter: GuestProgramCounter,
        registers: GuestRegisterState,
        stack: GuestStackState,
        phase: GuestRuntimePhase,
    ) -> Result<Self, GuestRuntimeStateError> {
        match phase {
            GuestRuntimePhase::Ready => {}
            GuestRuntimePhase::HelperSuspended(suspended)
                if suspended.call_site() != program_counter =>
            {
                return Err(GuestRuntimeStateError::ProgramCounterDoesNotMatchHelperCallSite);
            }
            GuestRuntimePhase::HelperReturned(returned)
                if returned.resume_at() != program_counter =>
            {
                return Err(GuestRuntimeStateError::ProgramCounterDoesNotMatchHelperReturn);
            }
            GuestRuntimePhase::HelperSuspended(_) | GuestRuntimePhase::HelperReturned(_) => {}
        }

        Ok(Self {
            program_counter,
            registers,
            stack,
            phase,
        })
    }

    pub const fn program_counter(&self) -> GuestProgramCounter {
        self.program_counter
    }

    pub const fn registers(&self) -> &GuestRegisterState {
        &self.registers
    }

    pub const fn stack(&self) -> GuestStackState {
        self.stack
    }

    pub const fn phase(&self) -> GuestRuntimePhase {
        self.phase
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestRuntimeStateError {
    DuplicateRegister(X86Reg),
    EmptyOrReversedHelperReturnRange,
    EmptyOrReversedStackBounds,
    PartialRegisterUnsupported(X86Reg),
    ProgramCounterDoesNotMatchHelperCallSite,
    ProgramCounterDoesNotMatchHelperReturn,
    StackPointerOutsideBounds,
}

#[cfg(test)]
mod tests;
