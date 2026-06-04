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

pub fn host_trap_plan_from_json(input: &str) -> Result<TestCaseHostTrapPlan, TestCaseJsonError> {
    let dto: TestCaseHostTrapPlanDto =
        serde_json::from_str(input).map_err(|error| TestCaseJsonError::Json(error.to_string()))?;

    host_trap_plan_from_dtos(dto.host_traps)
}

#[derive(Deserialize)]
struct TestCaseHostTrapPlanDto {
    #[serde(default)]
    host_traps: Vec<TestCaseHostTrapDto>,
}

#[cfg(test)]
mod tests {
    use super::{
        host_trap_plan_from_json, TestCaseHostTrapPlan, TestCaseJsonError, TestCaseStdoutTrap,
        TestCaseStdoutTrapError,
    };

    #[test]
    fn parses_stdout_host_trap_plan_json() {
        let plan = host_trap_plan_from_json(
            r#"{"host_traps":[{"kind":"stdout","text":"hello world\n"}]}"#,
        )
        .expect("host trap plan json parses");

        assert_eq!(
            plan,
            TestCaseHostTrapPlan::stdout(
                TestCaseStdoutTrap::from_text(String::from("hello world\n"))
                    .expect("stdout trap text is valid")
            )
        );
    }

    #[test]
    fn parses_missing_host_traps_as_empty_plan() {
        let plan = host_trap_plan_from_json("{}").expect("empty plan parses");

        assert_eq!(plan, TestCaseHostTrapPlan::none());
    }

    #[test]
    fn rejects_duplicate_stdout_host_trap_plan_json() {
        let result = host_trap_plan_from_json(
            r#"{"host_traps":[{"kind":"stdout","text":"hello\n"},{"kind":"stdout","text":"again\n"}]}"#,
        );

        assert_eq!(result, Err(TestCaseJsonError::DuplicateStdoutTrap));
    }

    #[test]
    fn rejects_empty_stdout_host_trap_plan_json() {
        let result = host_trap_plan_from_json(r#"{"host_traps":[{"kind":"stdout","text":""}]}"#);

        assert_eq!(
            result,
            Err(TestCaseJsonError::StdoutTrap(
                TestCaseStdoutTrapError::Empty
            ))
        );
    }
}
