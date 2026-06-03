use bara_ir::X86Va;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ArmPc(u64);

impl ArmPc {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcMapEntry {
    source: X86Va,
    target: ArmPc,
}

impl PcMapEntry {
    pub const fn new(source: X86Va, target: ArmPc) -> Self {
        Self { source, target }
    }

    pub const fn source(&self) -> X86Va {
        self.source
    }

    pub const fn target(&self) -> ArmPc {
        self.target
    }
}
