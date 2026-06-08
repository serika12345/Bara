use crate::program::X86Va;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryRequest {
    Syscall(SyscallRequest),
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
    use crate::{SyscallAbi, SyscallRequest, SyscallRequestError, X86Va};

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
}
