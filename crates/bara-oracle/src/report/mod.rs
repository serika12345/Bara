use serde::Serialize;

use crate::CaseId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CorpusReport {
    fixtures: Vec<FixtureReport>,
}

impl CorpusReport {
    fn new(fixtures: Vec<FixtureReport>) -> Self {
        Self { fixtures }
    }

    pub fn is_success(&self) -> bool {
        self.fixtures
            .iter()
            .all(|fixture| matches!(fixture.outcome(), FixtureOutcome::Passed))
    }

    pub fn fixtures(&self) -> &[FixtureReport] {
        &self.fixtures
    }
}

impl FromIterator<FixtureReport> for CorpusReport {
    fn from_iter<T: IntoIterator<Item = FixtureReport>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FixtureReport {
    case_id: CaseId,
    outcome: FixtureOutcome,
}

impl FixtureReport {
    pub const fn new(case_id: CaseId, outcome: FixtureOutcome) -> Self {
        Self { case_id, outcome }
    }

    pub const fn case_id(&self) -> &CaseId {
        &self.case_id
    }

    pub const fn outcome(&self) -> &FixtureOutcome {
        &self.outcome
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureOutcome {
    Passed,
    Failed {
        kind: FailureKind,
        message: FailureMessage,
    },
}

impl FixtureOutcome {
    pub const fn failed(kind: FailureKind, message: FailureMessage) -> Self {
        Self::Failed { kind, message }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FailureMessage(String);

impl FailureMessage {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for FailureMessage {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for FailureMessage {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    InvalidTestCase,
    MissingExpected,
    InvalidExpected,
    DecodeError,
    LiftError,
    EmitError,
    RunError,
    ComparisonMismatch,
}

#[cfg(test)]
mod tests {
    use crate::{CaseId, CorpusReport, FailureKind, FailureMessage, FixtureOutcome, FixtureReport};

    #[test]
    fn corpus_report_is_success_only_when_all_fixtures_pass() {
        let passed = FixtureReport::new(
            CaseId::new("pass").expect("case id is non-empty"),
            FixtureOutcome::Passed,
        );
        let failed = FixtureReport::new(
            CaseId::new("fail").expect("case id is non-empty"),
            FixtureOutcome::failed(
                FailureKind::DecodeError,
                FailureMessage::from("decode failed"),
            ),
        );

        assert!(vec![passed.clone()]
            .into_iter()
            .collect::<CorpusReport>()
            .is_success());
        assert!(!vec![passed, failed]
            .into_iter()
            .collect::<CorpusReport>()
            .is_success());
    }

    #[test]
    fn fixture_report_exposes_fields() {
        let case_id = CaseId::new("return_42").expect("case id is non-empty");
        let report = FixtureReport::new(case_id.clone(), FixtureOutcome::Passed);

        assert_eq!(report.case_id(), &case_id);
        assert_eq!(report.outcome(), &FixtureOutcome::Passed);
    }

    #[test]
    fn failure_message_exposes_string_value() {
        let message = FailureMessage::from("decode failed");

        assert_eq!(message.as_str(), "decode failed");
    }
}
