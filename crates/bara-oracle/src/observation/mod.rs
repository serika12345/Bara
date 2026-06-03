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

#[cfg(test)]
mod tests {
    use crate::{CaseId, CaseIdError, ObservedResult};

    #[test]
    fn case_id_rejects_empty_value() {
        assert_eq!(CaseId::new(""), Err(CaseIdError::Empty));
    }

    #[test]
    fn case_id_exposes_string_value() {
        let case_id = CaseId::new("return_42").expect("case id is non-empty");

        assert_eq!(case_id.as_str(), "return_42");
    }

    #[test]
    fn observed_result_exposes_fields() {
        let result = ObservedResult::new(
            CaseId::new("return_42").expect("case id is non-empty"),
            0,
            42,
            "out".to_owned(),
            "err".to_owned(),
        );

        assert_eq!(result.case_id().as_str(), "return_42");
        assert_eq!(result.exit_status(), 0);
        assert_eq!(result.return_value(), 42);
        assert_eq!(result.stdout(), "out");
        assert_eq!(result.stderr(), "err");
    }
}
