use std::num::NonZeroU64;

use serde::Deserialize;

use super::TestCaseJsonError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestCaseStackState {
    size: Option<TestCaseStackSize>,
}

impl TestCaseStackState {
    pub const fn none() -> Self {
        Self { size: None }
    }

    pub const fn with_size(size: TestCaseStackSize) -> Self {
        Self { size: Some(size) }
    }

    pub const fn size(&self) -> Option<TestCaseStackSize> {
        self.size
    }

    pub const fn is_empty(&self) -> bool {
        self.size.is_none()
    }
}

impl Default for TestCaseStackState {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestCaseStackSize {
    byte_count: NonZeroU64,
}

impl TestCaseStackSize {
    pub const fn from_nonzero_byte_count(byte_count: NonZeroU64) -> Self {
        Self { byte_count }
    }

    pub const fn byte_count(self) -> NonZeroU64 {
        self.byte_count
    }

    pub(crate) fn from_trusted_nonzero_byte_count(byte_count: u64) -> Self {
        Self {
            byte_count: NonZeroU64::new(byte_count)
                .expect("trusted testcase stack size is non-zero"),
        }
    }

    pub(crate) fn from_optional_nonzero_byte_count(byte_count: u64) -> Option<Self> {
        NonZeroU64::new(byte_count).map(Self::from_nonzero_byte_count)
    }

    pub(crate) fn try_from_json_byte_count(
        byte_count: u64,
    ) -> Result<Self, TestCaseStackSizeError> {
        if byte_count == 0 {
            Err(TestCaseStackSizeError::Zero)
        } else {
            Ok(Self::from_trusted_nonzero_byte_count(byte_count))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestCaseStackSizeError {
    Zero,
}

#[derive(Deserialize)]
pub(crate) struct TestCaseStackDto {
    size: Option<u64>,
}

pub(crate) fn stack_state_from_optional_dto(
    stack: Option<TestCaseStackDto>,
) -> Result<TestCaseStackState, TestCaseJsonError> {
    let Some(stack) = stack else {
        return Ok(TestCaseStackState::none());
    };

    let Some(size) = stack.size else {
        return Ok(TestCaseStackState::none());
    };

    TestCaseStackSize::try_from_json_byte_count(size)
        .map(TestCaseStackState::with_size)
        .map_err(TestCaseJsonError::StackSize)
}
