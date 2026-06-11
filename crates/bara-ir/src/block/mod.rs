use crate::boundary::{BoundaryRequest, ExternalCallRequest, HostHelperRequest, SyscallRequest};
use crate::program::X86Va;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct BlockId(u32);

impl BlockId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasicBlock {
    id: BlockId,
    start: X86Va,
    end: X86Va,
    ops: Vec<IrOp>,
    terminator: Terminator,
}

impl BasicBlock {
    pub fn new(
        id: BlockId,
        start: X86Va,
        end: X86Va,
        ops: Vec<IrOp>,
        terminator: Terminator,
    ) -> Result<Self, BasicBlockError> {
        if start >= end {
            return Err(BasicBlockError::EmptyOrReversedRange { start, end });
        }

        Ok(Self {
            id,
            start,
            end,
            ops,
            terminator,
        })
    }

    pub const fn id(&self) -> BlockId {
        self.id
    }

    pub const fn start(&self) -> X86Va {
        self.start
    }

    pub const fn end(&self) -> X86Va {
        self.end
    }

    pub fn ops(&self) -> &[IrOp] {
        &self.ops
    }

    pub const fn terminator(&self) -> &Terminator {
        &self.terminator
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BasicBlockError {
    EmptyOrReversedRange { start: X86Va, end: X86Va },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrOp {
    Mov { dst: Operand, src: Operand },
    Add { dst: Operand, src: Operand },
    Sub { dst: Operand, src: Operand },
    Cmp { lhs: Operand, rhs: Operand },
    Test { lhs: Operand, rhs: Operand },
    Push { src: Operand },
    Pop { dst: Operand },
    HostTrap { kind: HostTrapKind },
    Unsupported { reason: UnsupportedReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Terminator {
    Return,
    BoundaryRequest {
        request: BoundaryRequest,
    },
    Fallthrough {
        target: X86Va,
    },
    DirectJump {
        target: X86Va,
    },
    DirectCall {
        target: X86Va,
        return_to: X86Va,
    },
    CondJump {
        condition: X86Cond,
        taken: X86Va,
        fallthrough: X86Va,
    },
    Unsupported {
        reason: UnsupportedReason,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Cond {
    Overflow,
    NotOverflow,
    Below,
    AboveOrEqual,
    Equal,
    NotEqual,
    BelowOrEqual,
    Above,
    Sign,
    NotSign,
    Parity,
    NotParity,
    Less,
    GreaterOrEqual,
    LessOrEqual,
    Greater,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operand {
    Reg(X86Reg),
    ImmU64(u64),
    Mem8 {
        base: X86Reg,
    },
    MemRegIndirect {
        base: X86Reg,
        width: MemoryReadWidth,
    },
    MemRipRelative {
        address: X86Va,
        width: MemoryReadWidth,
    },
    AddressRipRelative {
        address: X86Va,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryReadWidth {
    Bits64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Reg {
    Rax,
    Eax,
    Ax,
    Al,
    Rdx,
    Edx,
    Dx,
    Dl,
    Rbx,
    Ebx,
    Bx,
    Bl,
    Rbp,
    Ebp,
    Bp,
    Bpl,
    Rsp,
    Esp,
    Sp,
    Spl,
    R14,
    R14d,
    R14w,
    R14b,
    R15,
    R15d,
    R15w,
    R15b,
    Rdi,
    Edi,
    Di,
    Dil,
    Rsi,
    Esi,
    Si,
    Sil,
}

impl X86Reg {
    pub const fn family(self) -> X86RegFamily {
        match self {
            Self::Rax | Self::Eax | Self::Ax | Self::Al => X86RegFamily::Accumulator,
            Self::Rdx | Self::Edx | Self::Dx | Self::Dl => X86RegFamily::Data,
            Self::Rbx | Self::Ebx | Self::Bx | Self::Bl => X86RegFamily::Base,
            Self::Rbp | Self::Ebp | Self::Bp | Self::Bpl => X86RegFamily::BasePointer,
            Self::Rsp | Self::Esp | Self::Sp | Self::Spl => X86RegFamily::StackPointer,
            Self::R14 | Self::R14d | Self::R14w | Self::R14b => X86RegFamily::Extended14,
            Self::R15 | Self::R15d | Self::R15w | Self::R15b => X86RegFamily::Extended15,
            Self::Rdi | Self::Edi | Self::Di | Self::Dil => X86RegFamily::DestinationIndex,
            Self::Rsi | Self::Esi | Self::Si | Self::Sil => X86RegFamily::SourceIndex,
        }
    }

    pub const fn width(self) -> X86RegWidth {
        match self {
            Self::Al
            | Self::Dl
            | Self::Bl
            | Self::Bpl
            | Self::Spl
            | Self::R14b
            | Self::R15b
            | Self::Dil
            | Self::Sil => X86RegWidth::Bits8,
            Self::Ax
            | Self::Dx
            | Self::Bx
            | Self::Bp
            | Self::Sp
            | Self::R14w
            | Self::R15w
            | Self::Di
            | Self::Si => X86RegWidth::Bits16,
            Self::Eax
            | Self::Edx
            | Self::Ebx
            | Self::Ebp
            | Self::Esp
            | Self::R14d
            | Self::R15d
            | Self::Edi
            | Self::Esi => X86RegWidth::Bits32,
            Self::Rax
            | Self::Rdx
            | Self::Rbx
            | Self::Rbp
            | Self::Rsp
            | Self::R14
            | Self::R15
            | Self::Rdi
            | Self::Rsi => X86RegWidth::Bits64,
        }
    }

    pub const fn full_width(self) -> Self {
        match self.family() {
            X86RegFamily::Accumulator => Self::Rax,
            X86RegFamily::Data => Self::Rdx,
            X86RegFamily::Base => Self::Rbx,
            X86RegFamily::BasePointer => Self::Rbp,
            X86RegFamily::StackPointer => Self::Rsp,
            X86RegFamily::Extended14 => Self::R14,
            X86RegFamily::Extended15 => Self::R15,
            X86RegFamily::DestinationIndex => Self::Rdi,
            X86RegFamily::SourceIndex => Self::Rsi,
        }
    }

    pub const fn is_partial_view(self) -> bool {
        !matches!(self.width(), X86RegWidth::Bits64)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86RegFamily {
    Accumulator,
    Data,
    Base,
    BasePointer,
    StackPointer,
    Extended14,
    Extended15,
    DestinationIndex,
    SourceIndex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86RegWidth {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostTrapKind {
    Stdout,
    AppKitGuiHelloWorld,
}

impl HostTrapKind {
    pub const fn host_helper_request(self) -> HostHelperRequest {
        match self {
            Self::Stdout => HostHelperRequest::WriteStdout,
            Self::AppKitGuiHelloWorld => HostHelperRequest::AppKitGuiHelloWorld,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    DecodeUnsupportedOpcode {
        opcode: u8,
        at: X86Va,
    },
    MissingReturnTerminator {
        at: X86Va,
    },
    DirectCallUnsupported {
        target: X86Va,
        return_to: X86Va,
    },
    RegisterIndirectMemoryReadUnsupported {
        base: X86Reg,
        width: MemoryReadWidth,
    },
    MappedMemoryReadUnsupported {
        address: X86Va,
        width: MemoryReadWidth,
    },
    ExternalCallUnsupported {
        request: ExternalCallRequest,
    },
    SyscallUnsupported {
        request: SyscallRequest,
    },
    EmitUnsupportedIr,
}

#[cfg(test)]
mod tests {
    use crate::{
        BasicBlock, BasicBlockError, BlockId, HostHelperName, HostHelperRequest,
        HostHelperSignature, HostTrapKind, IrOp, Operand, Terminator, X86Cond, X86Reg,
        X86RegFamily, X86RegWidth, X86Va,
    };

    #[test]
    fn block_id_exposes_value() {
        assert_eq!(BlockId::new(9).value(), 9);
    }

    #[test]
    fn basic_block_rejects_empty_range() {
        assert_eq!(
            BasicBlock::new(
                BlockId::new(0),
                X86Va::new(4),
                X86Va::new(4),
                Vec::new(),
                Terminator::Return
            ),
            Err(BasicBlockError::EmptyOrReversedRange {
                start: X86Va::new(4),
                end: X86Va::new(4)
            })
        );
    }

    #[test]
    fn basic_block_rejects_reversed_range() {
        assert_eq!(
            BasicBlock::new(
                BlockId::new(0),
                X86Va::new(5),
                X86Va::new(4),
                Vec::new(),
                Terminator::Return
            ),
            Err(BasicBlockError::EmptyOrReversedRange {
                start: X86Va::new(5),
                end: X86Va::new(4)
            })
        );
    }

    #[test]
    fn basic_block_exposes_fields() {
        let op = IrOp::Mov {
            dst: Operand::Reg(X86Reg::Rax),
            src: Operand::ImmU64(42),
        };
        let block = BasicBlock::new(
            BlockId::new(1),
            X86Va::new(0),
            X86Va::new(6),
            vec![op.clone()],
            Terminator::Return,
        )
        .expect("test block range is valid");

        assert_eq!(block.id(), BlockId::new(1));
        assert_eq!(block.start(), X86Va::new(0));
        assert_eq!(block.end(), X86Va::new(6));
        assert_eq!(block.ops(), &[op]);
        assert_eq!(block.terminator(), &Terminator::Return);
    }

    #[test]
    fn control_flow_terminators_expose_typed_targets() {
        assert_eq!(
            Terminator::DirectJump {
                target: X86Va::new(0x1020)
            },
            Terminator::DirectJump {
                target: X86Va::new(0x1020)
            }
        );
        assert_eq!(
            Terminator::CondJump {
                condition: X86Cond::Equal,
                taken: X86Va::new(0x1020),
                fallthrough: X86Va::new(0x1005),
            },
            Terminator::CondJump {
                condition: X86Cond::Equal,
                taken: X86Va::new(0x1020),
                fallthrough: X86Va::new(0x1005),
            }
        );
        assert_eq!(
            Terminator::DirectCall {
                target: X86Va::new(0x1020),
                return_to: X86Va::new(0x1005),
            },
            Terminator::DirectCall {
                target: X86Va::new(0x1020),
                return_to: X86Va::new(0x1005),
            }
        );
        assert_eq!(
            Terminator::Fallthrough {
                target: X86Va::new(0x1005)
            },
            Terminator::Fallthrough {
                target: X86Va::new(0x1005)
            }
        );
    }

    #[test]
    fn cmp_op_exposes_typed_operands() {
        assert_eq!(
            IrOp::Cmp {
                lhs: Operand::Reg(X86Reg::Rax),
                rhs: Operand::ImmU64(42),
            },
            IrOp::Cmp {
                lhs: Operand::Reg(X86Reg::Rax),
                rhs: Operand::ImmU64(42),
            }
        );
    }

    #[test]
    fn test_op_exposes_typed_operands() {
        assert_eq!(
            IrOp::Test {
                lhs: Operand::Reg(X86Reg::Rax),
                rhs: Operand::Reg(X86Reg::Rax),
            },
            IrOp::Test {
                lhs: Operand::Reg(X86Reg::Rax),
                rhs: Operand::Reg(X86Reg::Rax),
            }
        );
    }

    #[test]
    fn push_pop_ops_expose_typed_operands() {
        assert_eq!(
            IrOp::Push {
                src: Operand::Reg(X86Reg::Rax),
            },
            IrOp::Push {
                src: Operand::Reg(X86Reg::Rax),
            }
        );
        assert_eq!(
            IrOp::Pop {
                dst: Operand::Reg(X86Reg::Rax),
            },
            IrOp::Pop {
                dst: Operand::Reg(X86Reg::Rax),
            }
        );
    }

    #[test]
    fn x86_register_model_exposes_partial_register_views_by_family_and_width() {
        assert_eq!(X86Reg::Rax.family(), X86RegFamily::Accumulator);
        assert_eq!(X86Reg::Eax.family(), X86RegFamily::Accumulator);
        assert_eq!(X86Reg::Ax.family(), X86RegFamily::Accumulator);
        assert_eq!(X86Reg::Al.family(), X86RegFamily::Accumulator);
        assert_eq!(X86Reg::Rax.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::Eax.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::Ax.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::Al.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::Eax.full_width(), X86Reg::Rax);
        assert_eq!(X86Reg::Ax.full_width(), X86Reg::Rax);
        assert_eq!(X86Reg::Al.full_width(), X86Reg::Rax);
        assert!(X86Reg::Eax.is_partial_view());

        assert_eq!(X86Reg::Rdx.family(), X86RegFamily::Data);
        assert_eq!(X86Reg::Edx.family(), X86RegFamily::Data);
        assert_eq!(X86Reg::Dx.family(), X86RegFamily::Data);
        assert_eq!(X86Reg::Dl.family(), X86RegFamily::Data);
        assert_eq!(X86Reg::Rdx.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::Edx.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::Dx.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::Dl.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::Edx.full_width(), X86Reg::Rdx);
        assert_eq!(X86Reg::Dx.full_width(), X86Reg::Rdx);
        assert_eq!(X86Reg::Dl.full_width(), X86Reg::Rdx);
        assert!(X86Reg::Edx.is_partial_view());

        assert_eq!(X86Reg::Rbx.family(), X86RegFamily::Base);
        assert_eq!(X86Reg::Ebx.family(), X86RegFamily::Base);
        assert_eq!(X86Reg::Bx.family(), X86RegFamily::Base);
        assert_eq!(X86Reg::Bl.family(), X86RegFamily::Base);
        assert_eq!(X86Reg::Rbx.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::Ebx.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::Bx.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::Bl.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::Ebx.full_width(), X86Reg::Rbx);
        assert_eq!(X86Reg::Bx.full_width(), X86Reg::Rbx);
        assert_eq!(X86Reg::Bl.full_width(), X86Reg::Rbx);
        assert!(X86Reg::Ebx.is_partial_view());

        assert_eq!(X86Reg::Rbp.family(), X86RegFamily::BasePointer);
        assert_eq!(X86Reg::Ebp.family(), X86RegFamily::BasePointer);
        assert_eq!(X86Reg::Bp.family(), X86RegFamily::BasePointer);
        assert_eq!(X86Reg::Bpl.family(), X86RegFamily::BasePointer);
        assert_eq!(X86Reg::Rbp.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::Ebp.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::Bp.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::Bpl.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::Ebp.full_width(), X86Reg::Rbp);
        assert_eq!(X86Reg::Bp.full_width(), X86Reg::Rbp);
        assert_eq!(X86Reg::Bpl.full_width(), X86Reg::Rbp);
        assert!(X86Reg::Ebp.is_partial_view());

        assert_eq!(X86Reg::Rsp.family(), X86RegFamily::StackPointer);
        assert_eq!(X86Reg::Esp.family(), X86RegFamily::StackPointer);
        assert_eq!(X86Reg::Sp.family(), X86RegFamily::StackPointer);
        assert_eq!(X86Reg::Spl.family(), X86RegFamily::StackPointer);
        assert_eq!(X86Reg::Rsp.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::Esp.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::Sp.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::Spl.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::Esp.full_width(), X86Reg::Rsp);
        assert_eq!(X86Reg::Sp.full_width(), X86Reg::Rsp);
        assert_eq!(X86Reg::Spl.full_width(), X86Reg::Rsp);
        assert!(X86Reg::Esp.is_partial_view());

        assert_eq!(X86Reg::R14.family(), X86RegFamily::Extended14);
        assert_eq!(X86Reg::R14d.family(), X86RegFamily::Extended14);
        assert_eq!(X86Reg::R14w.family(), X86RegFamily::Extended14);
        assert_eq!(X86Reg::R14b.family(), X86RegFamily::Extended14);
        assert_eq!(X86Reg::R14.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::R14d.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::R14w.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::R14b.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::R14d.full_width(), X86Reg::R14);
        assert_eq!(X86Reg::R14w.full_width(), X86Reg::R14);
        assert_eq!(X86Reg::R14b.full_width(), X86Reg::R14);
        assert!(X86Reg::R14d.is_partial_view());

        assert_eq!(X86Reg::R15.family(), X86RegFamily::Extended15);
        assert_eq!(X86Reg::R15d.family(), X86RegFamily::Extended15);
        assert_eq!(X86Reg::R15w.family(), X86RegFamily::Extended15);
        assert_eq!(X86Reg::R15b.family(), X86RegFamily::Extended15);
        assert_eq!(X86Reg::R15.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::R15d.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::R15w.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::R15b.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::R15d.full_width(), X86Reg::R15);
        assert_eq!(X86Reg::R15w.full_width(), X86Reg::R15);
        assert_eq!(X86Reg::R15b.full_width(), X86Reg::R15);
        assert!(X86Reg::R15d.is_partial_view());

        assert_eq!(X86Reg::Rdi.family(), X86RegFamily::DestinationIndex);
        assert_eq!(X86Reg::Edi.family(), X86RegFamily::DestinationIndex);
        assert_eq!(X86Reg::Di.family(), X86RegFamily::DestinationIndex);
        assert_eq!(X86Reg::Dil.family(), X86RegFamily::DestinationIndex);
        assert_eq!(X86Reg::Rdi.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::Edi.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::Di.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::Dil.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::Edi.full_width(), X86Reg::Rdi);
        assert_eq!(X86Reg::Di.full_width(), X86Reg::Rdi);
        assert_eq!(X86Reg::Dil.full_width(), X86Reg::Rdi);
        assert!(!X86Reg::Rdi.is_partial_view());

        assert_eq!(X86Reg::Rsi.family(), X86RegFamily::SourceIndex);
        assert_eq!(X86Reg::Esi.family(), X86RegFamily::SourceIndex);
        assert_eq!(X86Reg::Si.family(), X86RegFamily::SourceIndex);
        assert_eq!(X86Reg::Sil.family(), X86RegFamily::SourceIndex);
        assert_eq!(X86Reg::Rsi.width(), X86RegWidth::Bits64);
        assert_eq!(X86Reg::Esi.width(), X86RegWidth::Bits32);
        assert_eq!(X86Reg::Si.width(), X86RegWidth::Bits16);
        assert_eq!(X86Reg::Sil.width(), X86RegWidth::Bits8);
        assert_eq!(X86Reg::Esi.full_width(), X86Reg::Rsi);
        assert_eq!(X86Reg::Si.full_width(), X86Reg::Rsi);
        assert_eq!(X86Reg::Sil.full_width(), X86Reg::Rsi);
        assert!(!X86Reg::Rsi.is_partial_view());
    }

    #[test]
    fn stdout_host_trap_maps_to_write_stdout_host_helper_request() {
        let abi = HostTrapKind::Stdout.host_helper_request().abi();

        assert_eq!(
            HostTrapKind::Stdout.host_helper_request(),
            HostHelperRequest::WriteStdout
        );
        assert_eq!(abi.name(), HostHelperName::WriteStdout);
        assert_eq!(abi.signature(), HostHelperSignature::PtrLenToUnit);
    }

    #[test]
    fn appkit_gui_host_trap_maps_to_gui_lifecycle_host_helper_request() {
        let abi = HostTrapKind::AppKitGuiHelloWorld
            .host_helper_request()
            .abi();

        assert_eq!(
            HostTrapKind::AppKitGuiHelloWorld.host_helper_request(),
            HostHelperRequest::AppKitGuiHelloWorld
        );
        assert_eq!(abi.name(), HostHelperName::AppKitGuiHelloWorld);
        assert_eq!(
            abi.signature(),
            HostHelperSignature::NoArgsToGuiLifecycleEvent
        );
    }
}
