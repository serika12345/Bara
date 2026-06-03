#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostTrapPlan {
    stdout: RunStdout,
}

impl HostTrapPlan {
    pub const fn none() -> Self {
        Self {
            stdout: RunStdout::empty(),
        }
    }

    pub const fn stdout(stdout: RunStdout) -> Self {
        Self { stdout }
    }

    pub const fn stdout_output(&self) -> &RunStdout {
        &self.stdout
    }
}

impl Default for HostTrapPlan {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunStdout {
    text: String,
}

impl RunStdout {
    pub const fn empty() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn from_text(text: String) -> Result<Self, RunStdoutError> {
        if !text.is_ascii() {
            return Err(RunStdoutError::NonAscii);
        }

        Ok(Self { text })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunStdoutError {
    NonAscii,
}

#[cfg(test)]
mod tests {
    use crate::{HostTrapPlan, RunStdout, RunStdoutError};

    #[test]
    fn stdout_trap_accepts_ascii_text() {
        let stdout =
            RunStdout::from_text(String::from("hello trap\n")).expect("stdout trap text is ascii");

        assert_eq!(stdout.as_str(), "hello trap\n");
    }

    #[test]
    fn stdout_trap_rejects_non_ascii_text() {
        assert_eq!(
            RunStdout::from_text(String::from("こんにちは")),
            Err(RunStdoutError::NonAscii)
        );
    }

    #[test]
    fn host_trap_plan_defaults_to_empty_stdout() {
        assert_eq!(HostTrapPlan::none().stdout_output().as_str(), "");
    }
}
