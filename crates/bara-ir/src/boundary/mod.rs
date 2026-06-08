use crate::program::X86Va;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryRequest {
    Helper(HelperRequest),
    Syscall(SyscallRequest),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelperRequest {
    CallExternal(ExternalCallRequest),
}

impl HelperRequest {
    pub const fn runtime_helper(self) -> RuntimeHelper {
        match self {
            Self::CallExternal(_) => RuntimeHelper::CallExternal,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostHelperRequest {
    WriteStdout,
}

impl HostHelperRequest {
    pub const fn abi(self) -> HostHelperAbi {
        match self {
            Self::WriteStdout => HostHelperAbi::new(
                HostHelperName::WriteStdout,
                HostHelperSignature::PtrLenToUnit,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostHelperAbi {
    name: HostHelperName,
    signature: HostHelperSignature,
}

impl HostHelperAbi {
    pub const fn new(name: HostHelperName, signature: HostHelperSignature) -> Self {
        Self { name, signature }
    }

    pub const fn name(self) -> HostHelperName {
        self.name
    }

    pub const fn signature(self) -> HostHelperSignature {
        self.signature
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostHelperName {
    WriteStdout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostHelperSignature {
    PtrLenToUnit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeHelper {
    CallExternal,
    Unimplemented,
    Exit,
}

impl RuntimeHelper {
    pub const fn abi(self) -> RuntimeHelperAbi {
        match self {
            Self::CallExternal => RuntimeHelperAbi::new(
                RuntimeHelperName::HelperCallExternal,
                RuntimeHelperSignature::StateExternalSymbolToUnit,
            ),
            Self::Unimplemented => RuntimeHelperAbi::new(
                RuntimeHelperName::HelperUnimplemented,
                RuntimeHelperSignature::StateUnimplementedReasonToUnit,
            ),
            Self::Exit => RuntimeHelperAbi::new(
                RuntimeHelperName::HelperExit,
                RuntimeHelperSignature::StateExitCodeToNever,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeHelperAbi {
    name: RuntimeHelperName,
    signature: RuntimeHelperSignature,
}

impl RuntimeHelperAbi {
    pub const fn new(name: RuntimeHelperName, signature: RuntimeHelperSignature) -> Self {
        Self { name, signature }
    }

    pub const fn minimal_b4_set() -> [Self; 3] {
        [
            RuntimeHelper::CallExternal.abi(),
            RuntimeHelper::Unimplemented.abi(),
            RuntimeHelper::Exit.abi(),
        ]
    }

    pub const fn name(self) -> RuntimeHelperName {
        self.name
    }

    pub const fn signature(self) -> RuntimeHelperSignature {
        self.signature
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeHelperName {
    HelperCallExternal,
    HelperUnimplemented,
    HelperExit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeHelperSignature {
    StateExternalSymbolToUnit,
    StateUnimplementedReasonToUnit,
    StateExitCodeToNever,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ExternalSymbolId(u32);

impl ExternalSymbolId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExternalCallRequest {
    symbol: ExternalSymbolId,
    call_site: X86Va,
    return_to: X86Va,
}

impl ExternalCallRequest {
    pub fn new(
        symbol: ExternalSymbolId,
        call_site: X86Va,
        return_to: X86Va,
    ) -> Result<Self, ExternalCallRequestError> {
        if call_site >= return_to {
            return Err(ExternalCallRequestError::EmptyOrReversedRange {
                call_site,
                return_to,
            });
        }

        Ok(Self {
            symbol,
            call_site,
            return_to,
        })
    }

    pub const fn symbol(self) -> ExternalSymbolId {
        self.symbol
    }

    pub const fn call_site(self) -> X86Va {
        self.call_site
    }

    pub const fn return_to(self) -> X86Va {
        self.return_to
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalCallRequestError {
    EmptyOrReversedRange { call_site: X86Va, return_to: X86Va },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallRequest {
    abi: SyscallAbi,
    at: X86Va,
    return_to: X86Va,
}

impl SyscallRequest {
    pub fn new(abi: SyscallAbi, at: X86Va, return_to: X86Va) -> Result<Self, SyscallRequestError> {
        if at >= return_to {
            return Err(SyscallRequestError::EmptyOrReversedRange { at, return_to });
        }

        Ok(Self { abi, at, return_to })
    }

    pub const fn abi(self) -> SyscallAbi {
        self.abi
    }

    pub const fn at(self) -> X86Va {
        self.at
    }

    pub const fn return_to(self) -> X86Va {
        self.return_to
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallAbi {
    X86_64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallRequestError {
    EmptyOrReversedRange { at: X86Va, return_to: X86Va },
}

#[cfg(test)]
mod tests {
    use crate::{
        ExternalCallRequest, ExternalCallRequestError, ExternalSymbolId, HostHelperAbi,
        HostHelperName, HostHelperRequest, HostHelperSignature, RuntimeHelper, RuntimeHelperAbi,
        RuntimeHelperName, RuntimeHelperSignature, SyscallAbi, SyscallRequest, SyscallRequestError,
        X86Va,
    };

    #[test]
    fn syscall_request_exposes_public_abi_and_range() {
        let request =
            SyscallRequest::new(SyscallAbi::X86_64, X86Va::new(0x1000), X86Va::new(0x1002))
                .expect("test syscall range is valid");

        assert_eq!(request.abi(), SyscallAbi::X86_64);
        assert_eq!(request.at(), X86Va::new(0x1000));
        assert_eq!(request.return_to(), X86Va::new(0x1002));
    }

    #[test]
    fn syscall_request_rejects_empty_range() {
        assert_eq!(
            SyscallRequest::new(SyscallAbi::X86_64, X86Va::new(4), X86Va::new(4)),
            Err(SyscallRequestError::EmptyOrReversedRange {
                at: X86Va::new(4),
                return_to: X86Va::new(4)
            })
        );
    }

    #[test]
    fn syscall_request_rejects_reversed_range() {
        assert_eq!(
            SyscallRequest::new(SyscallAbi::X86_64, X86Va::new(5), X86Va::new(4)),
            Err(SyscallRequestError::EmptyOrReversedRange {
                at: X86Va::new(5),
                return_to: X86Va::new(4)
            })
        );
    }

    #[test]
    fn external_call_request_exposes_symbol_and_range() {
        let request = ExternalCallRequest::new(
            ExternalSymbolId::new(7),
            X86Va::new(0x2000),
            X86Va::new(0x2005),
        )
        .expect("test external call range is valid");

        assert_eq!(request.symbol(), ExternalSymbolId::new(7));
        assert_eq!(request.call_site(), X86Va::new(0x2000));
        assert_eq!(request.return_to(), X86Va::new(0x2005));
    }

    #[test]
    fn external_call_request_rejects_empty_range() {
        assert_eq!(
            ExternalCallRequest::new(ExternalSymbolId::new(7), X86Va::new(4), X86Va::new(4)),
            Err(ExternalCallRequestError::EmptyOrReversedRange {
                call_site: X86Va::new(4),
                return_to: X86Va::new(4)
            })
        );
    }

    #[test]
    fn call_external_request_maps_to_call_external_runtime_helper() {
        let request =
            ExternalCallRequest::new(ExternalSymbolId::new(7), X86Va::new(0), X86Va::new(5))
                .expect("test external call range is valid");

        assert_eq!(
            crate::HelperRequest::CallExternal(request).runtime_helper(),
            RuntimeHelper::CallExternal
        );
    }

    #[test]
    fn runtime_helper_abi_defines_call_external() {
        assert_eq!(
            RuntimeHelper::CallExternal.abi(),
            RuntimeHelperAbi::new(
                RuntimeHelperName::HelperCallExternal,
                RuntimeHelperSignature::StateExternalSymbolToUnit,
            )
        );
    }

    #[test]
    fn runtime_helper_abi_defines_unimplemented() {
        assert_eq!(
            RuntimeHelper::Unimplemented.abi(),
            RuntimeHelperAbi::new(
                RuntimeHelperName::HelperUnimplemented,
                RuntimeHelperSignature::StateUnimplementedReasonToUnit,
            )
        );
    }

    #[test]
    fn runtime_helper_abi_defines_exit() {
        assert_eq!(
            RuntimeHelper::Exit.abi(),
            RuntimeHelperAbi::new(
                RuntimeHelperName::HelperExit,
                RuntimeHelperSignature::StateExitCodeToNever,
            )
        );
    }

    #[test]
    fn runtime_helper_minimal_b4_set_is_stable() {
        assert_eq!(
            RuntimeHelperAbi::minimal_b4_set(),
            [
                RuntimeHelper::CallExternal.abi(),
                RuntimeHelper::Unimplemented.abi(),
                RuntimeHelper::Exit.abi(),
            ]
        );
    }

    #[test]
    fn host_helper_abi_defines_write_stdout() {
        assert_eq!(
            HostHelperRequest::WriteStdout.abi(),
            HostHelperAbi::new(
                HostHelperName::WriteStdout,
                HostHelperSignature::PtrLenToUnit,
            )
        );
    }
}
