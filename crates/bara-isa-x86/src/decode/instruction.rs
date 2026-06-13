use bara_ir::{UnsupportedReason, X86Cond, X86Va};

use super::{DecodeError, X86Imm32, X86Imm8};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedFunction {
    entry: X86Va,
    instructions: Vec<DecodedInstruction>,
}

impl DecodedFunction {
    pub fn new(entry: X86Va, instructions: Vec<DecodedInstruction>) -> Result<Self, DecodeError> {
        if instructions.is_empty() {
            return Err(DecodeError::EmptyFunction { entry });
        }

        Ok(Self {
            entry,
            instructions,
        })
    }

    pub const fn entry(&self) -> X86Va {
        self.entry
    }

    pub fn instructions(&self) -> &[DecodedInstruction] {
        &self.instructions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedInstruction {
    start: X86Va,
    end: X86Va,
    kind: DecodedInstructionKind,
}

impl DecodedInstruction {
    pub const fn new(start: X86Va, end: X86Va, kind: DecodedInstructionKind) -> Self {
        Self { start, end, kind }
    }

    pub const fn start(&self) -> X86Va {
        self.start
    }

    pub const fn end(&self) -> X86Va {
        self.end
    }

    pub const fn kind(&self) -> &DecodedInstructionKind {
        &self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedInstructionKind {
    MovEaxImm32 {
        imm: u32,
    },
    MovRaxRdi,
    MovRbxRax,
    MovRaxQwordPtrRipRelative {
        displacement: X86Imm32,
        address: X86Va,
    },
    MovRdiQwordPtrRipRelative {
        displacement: X86Imm32,
        address: X86Va,
    },
    MovRsiQwordPtrRipRelative {
        displacement: X86Imm32,
        address: X86Va,
    },
    MovR14QwordPtrRipRelative {
        displacement: X86Imm32,
        address: X86Va,
    },
    MovR15QwordPtrRipRelative {
        displacement: X86Imm32,
        address: X86Va,
    },
    MovRdiQwordPtrR15,
    MovRdxQwordPtrRax,
    LeaRdiRipRelative {
        displacement: X86Imm32,
        address: X86Va,
    },
    LeaRsiRipRelative {
        displacement: X86Imm32,
        address: X86Va,
    },
    MovzxEaxBytePtrRdi,
    AddEaxImm32 {
        imm: X86Imm32,
    },
    AddEaxImm8 {
        imm: X86Imm8,
    },
    SubEaxImm32 {
        imm: X86Imm32,
    },
    SubEaxImm8 {
        imm: X86Imm8,
    },
    CmpEaxImm32 {
        imm: X86Imm32,
    },
    CmpEaxImm8 {
        imm: X86Imm8,
    },
    TestEaxEax,
    MovRbpRsp,
    PushRax,
    PushRbx,
    PushRbp,
    PushR14,
    PushR15,
    PopRax,
    XorEaxEax,
    JccRel8 {
        condition: X86Cond,
        taken: X86Va,
        fallthrough: X86Va,
    },
    JccRel32 {
        condition: X86Cond,
        taken: X86Va,
        fallthrough: X86Va,
    },
    JmpRel8 {
        target: X86Va,
    },
    CallRel32 {
        target: X86Va,
        return_to: X86Va,
    },
    CallR14 {
        return_to: X86Va,
    },
    Syscall,
    BaraHostTrapSentinel,
    BaraAppKitGuiHelloWorldTrapSentinel,
    Ret,
    Unsupported {
        reason: UnsupportedReason,
    },
}
