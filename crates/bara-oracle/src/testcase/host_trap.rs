use serde::Deserialize;

use super::TestCaseJsonError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestCaseHostTrapPlan {
    stdout: Option<TestCaseStdoutTrap>,
}

impl TestCaseHostTrapPlan {
    pub const fn none() -> Self {
        Self { stdout: None }
    }

    pub const fn stdout(stdout: TestCaseStdoutTrap) -> Self {
        Self {
            stdout: Some(stdout),
        }
    }

    pub const fn stdout_trap(&self) -> Option<&TestCaseStdoutTrap> {
        self.stdout.as_ref()
    }

    pub const fn is_empty(&self) -> bool {
        self.stdout.is_none()
    }
}

impl Default for TestCaseHostTrapPlan {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestCaseStdoutTrap {
    text: String,
}

impl TestCaseStdoutTrap {
    pub fn from_text(text: String) -> Result<Self, TestCaseStdoutTrapError> {
        if text.is_empty() {
            return Err(TestCaseStdoutTrapError::Empty);
        }
        if !text.is_ascii() {
            return Err(TestCaseStdoutTrapError::NonAscii);
        }

        Ok(Self { text })
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestCaseStdoutTrapError {
    Empty,
    NonAscii,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "kind")]
pub(crate) enum TestCaseHostTrapDto {
    Stdout { text: String },
}

pub(crate) fn host_trap_plan_from_dtos(
    traps: Vec<TestCaseHostTrapDto>,
) -> Result<TestCaseHostTrapPlan, TestCaseJsonError> {
    let mut stdout = None;
    for trap in traps {
        match trap {
            TestCaseHostTrapDto::Stdout { text } => {
                if stdout.is_some() {
                    return Err(TestCaseJsonError::DuplicateStdoutTrap);
                }
                stdout = Some(
                    TestCaseStdoutTrap::from_text(text).map_err(TestCaseJsonError::StdoutTrap)?,
                );
            }
        }
    }

    Ok(match stdout {
        Some(stdout) => TestCaseHostTrapPlan::stdout(stdout),
        None => TestCaseHostTrapPlan::none(),
    })
}
