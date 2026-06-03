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

#[cfg(test)]
mod tests {
    use bara_ir::X86Va;

    use crate::{ArmPc, PcMapEntry};

    #[test]
    fn arm_pc_exposes_value() {
        assert_eq!(ArmPc::new(12).value(), 12);
    }

    #[test]
    fn pc_map_entry_exposes_source_and_target() {
        let entry = PcMapEntry::new(X86Va::new(0x1000), ArmPc::new(8));

        assert_eq!(entry.source(), X86Va::new(0x1000));
        assert_eq!(entry.target(), ArmPc::new(8));
    }
}
