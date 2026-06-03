use crate::ObservedResult;

#[derive(Debug)]
pub struct JsonError {
    source: serde_json::Error,
}

impl JsonError {
    pub const fn new(source: serde_json::Error) -> Self {
        Self { source }
    }

    pub const fn source(&self) -> &serde_json::Error {
        &self.source
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "json serialization error: {}", self.source)
    }
}

impl std::error::Error for JsonError {}

pub fn observed_result_to_json(result: &ObservedResult) -> Result<String, JsonError> {
    serde_json::to_string(result).map_err(JsonError::new)
}

#[cfg(test)]
mod tests {
    use crate::{observed_result_to_json, CaseId, ObservedResult};

    #[test]
    fn observed_result_serializes_as_m1_json() {
        let result = ObservedResult::new(
            CaseId::new("return_42").expect("test case id is non-empty"),
            0,
            42,
            String::new(),
            String::new(),
        );

        assert_eq!(
            observed_result_to_json(&result).expect("test result serializes"),
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}"
        );
    }
}
