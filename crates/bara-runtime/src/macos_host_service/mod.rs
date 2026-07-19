use crate::{
    GuestHelperReturnState, GuestHelperSuspendState, GuestRegisterState, GuestRegisterValue,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacOsHostService {
    ObjectiveCMessageSend,
    AppKitApplicationRun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestCall {
    service: MacOsHostService,
    suspended: GuestHelperSuspendState,
    arguments: GuestRegisterState,
}

impl GuestCall {
    pub const fn new(
        service: MacOsHostService,
        suspended: GuestHelperSuspendState,
        arguments: GuestRegisterState,
    ) -> Self {
        Self {
            service,
            suspended,
            arguments,
        }
    }

    pub const fn service(&self) -> MacOsHostService {
        self.service
    }

    pub const fn suspended(&self) -> GuestHelperSuspendState {
        self.suspended
    }

    pub const fn arguments(&self) -> &GuestRegisterState {
        &self.arguments
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacOsHostServiceRequest {
    call: GuestCall,
}

impl MacOsHostServiceRequest {
    pub const fn from_guest_call(call: GuestCall) -> Self {
        Self { call }
    }

    pub const fn service(&self) -> MacOsHostService {
        self.call.service()
    }

    pub const fn call(&self) -> &GuestCall {
        &self.call
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestReturnValue {
    NoValue,
    Rax(GuestRegisterValue),
}

impl GuestReturnValue {
    pub const fn rax(value: GuestRegisterValue) -> Self {
        Self::Rax(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestReturn {
    request: MacOsHostServiceRequest,
    state: GuestHelperReturnState,
    value: GuestReturnValue,
}

impl GuestReturn {
    pub fn new(request: MacOsHostServiceRequest, value: GuestReturnValue) -> Self {
        let state = GuestHelperReturnState::from_suspend(request.call().suspended());
        Self {
            request,
            state,
            value,
        }
    }

    pub const fn request(&self) -> &MacOsHostServiceRequest {
        &self.request
    }

    pub const fn state(&self) -> GuestHelperReturnState {
        self.state
    }

    pub const fn value(&self) -> GuestReturnValue {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use bara_ir::X86Va;

    use crate::{
        GuestHelperSuspendState, GuestProgramCounter, GuestRegisterState, GuestRegisterStateEntry,
        GuestRegisterValue,
    };
    use bara_ir::X86Reg;

    use super::{
        GuestCall, GuestReturn, GuestReturnValue, MacOsHostService, MacOsHostServiceRequest,
    };

    #[test]
    fn guest_call_becomes_concrete_macos_request_and_typed_guest_return() {
        let suspended = GuestHelperSuspendState::new(
            GuestProgramCounter::new(X86Va::new(0x1000)),
            GuestProgramCounter::new(X86Va::new(0x1003)),
        )
        .expect("helper call has a forward return PC");
        let arguments = GuestRegisterState::from_entries([GuestRegisterStateEntry::new(
            X86Reg::Rdi,
            GuestRegisterValue::guest_address(X86Va::new(0x4000)),
        )])
        .expect("argument register is full-width");
        let call = GuestCall::new(
            MacOsHostService::ObjectiveCMessageSend,
            suspended,
            arguments.clone(),
        );

        let request = MacOsHostServiceRequest::from_guest_call(call.clone());
        assert_eq!(request.service(), MacOsHostService::ObjectiveCMessageSend);
        assert_eq!(request.call(), &call);
        assert_eq!(request.call().arguments(), &arguments);

        let guest_return = GuestReturn::new(
            request.clone(),
            GuestReturnValue::rax(GuestRegisterValue::guest_address(X86Va::new(0x5000))),
        );
        assert_eq!(guest_return.request(), &request);
        assert_eq!(
            guest_return.state().resume_at(),
            GuestProgramCounter::new(X86Va::new(0x1003))
        );
        assert_eq!(
            guest_return.value(),
            GuestReturnValue::rax(GuestRegisterValue::guest_address(X86Va::new(0x5000)))
        );
    }
}
