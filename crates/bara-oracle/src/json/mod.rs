use crate::{CorpusReport, ObservedResult};

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
        write!(formatter, "json error: {}", self.source)
    }
}

impl std::error::Error for JsonError {}

pub fn observed_result_to_json(result: &ObservedResult) -> Result<String, JsonError> {
    serde_json::to_string(result).map_err(JsonError::new)
}

pub fn observed_result_from_json(input: &str) -> Result<ObservedResult, JsonError> {
    serde_json::from_str(input).map_err(JsonError::new)
}

pub fn corpus_report_to_json(report: &CorpusReport) -> Result<String, JsonError> {
    serde_json::to_string(report).map_err(JsonError::new)
}

#[cfg(test)]
mod tests {
    use crate::{
        corpus_report_to_json, observed_result_from_json, observed_result_to_json, CaseId,
        CorpusReport, FailureKind, FailureMessage, FixtureOutcome, FixtureReport, ObservedResult,
    };

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

    #[test]
    fn observed_result_parses_from_m1_json() {
        let result = observed_result_from_json(
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}",
        )
        .expect("expected result json parses");

        assert_eq!(
            result,
            ObservedResult::new(
                CaseId::new("return_42").expect("test case id is non-empty"),
                0,
                42,
                String::new(),
                String::new()
            )
        );
    }

    #[test]
    fn corpus_report_serializes_as_stable_json() {
        let report = vec![
            FixtureReport::new(
                CaseId::new("return_42").expect("case id is non-empty"),
                FixtureOutcome::Passed,
            ),
            FixtureReport::new(
                CaseId::new("bad_case").expect("case id is non-empty"),
                FixtureOutcome::failed(
                    FailureKind::DecodeError,
                    FailureMessage::from("decode failed"),
                ),
            ),
        ]
        .into_iter()
        .collect::<CorpusReport>();

        assert_eq!(
            corpus_report_to_json(&report).expect("report serializes"),
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"},{\"case_id\":\"bad_case\",\"outcome\":{\"failed\":{\"kind\":\"decode_error\",\"message\":\"decode failed\"}}}]}"
        );
    }
}
