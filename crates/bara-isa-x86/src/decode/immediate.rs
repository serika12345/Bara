#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86Imm32 {
    value: i32,
}

impl X86Imm32 {
    pub(crate) const fn new(value: i32) -> Self {
        Self { value }
    }

    pub(crate) fn as_i64(self) -> i64 {
        i64::from(self.value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86Imm8(i8);

impl X86Imm8 {
    pub const fn new(value: i8) -> Self {
        Self(value)
    }

    pub(crate) fn as_i64(self) -> i64 {
        i64::from(self.0)
    }
}
