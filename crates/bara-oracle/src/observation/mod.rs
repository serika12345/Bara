use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct CaseId(String);

impl CaseId {
    pub fn new(value: impl Into<String>) -> Result<Self, CaseIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CaseIdError::Empty);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaseIdError {
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ObservedResult {
    case_id: CaseId,
    exit_status: i32,
    return_value: u64,
    stdout: String,
    stderr: String,
}

pub type ExpectedResult = ObservedResult;

impl ObservedResult {
    pub fn new(
        case_id: CaseId,
        exit_status: i32,
        return_value: u64,
        stdout: String,
        stderr: String,
    ) -> Self {
        Self {
            case_id,
            exit_status,
            return_value,
            stdout,
            stderr,
        }
    }

    pub const fn case_id(&self) -> &CaseId {
        &self.case_id
    }

    pub const fn exit_status(&self) -> i32 {
        self.exit_status
    }

    pub const fn return_value(&self) -> u64 {
        self.return_value
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }
}
