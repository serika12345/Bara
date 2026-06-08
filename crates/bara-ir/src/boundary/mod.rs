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
        ExternalCallRequest, ExternalCallRequestError, ExternalSymbolId, SyscallAbi,
        SyscallRequest, SyscallRequestError, X86Va,
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
}
