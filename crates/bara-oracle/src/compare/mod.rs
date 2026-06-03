use crate::{CaseId, ObservedResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparisonReport {
    issues: Vec<ComparisonIssue>,
}

impl ComparisonReport {
    pub fn new(issues: Vec<ComparisonIssue>) -> Self {
        Self { issues }
    }

    pub fn is_match(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn issues(&self) -> &[ComparisonIssue] {
        &self.issues
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComparisonIssue {
    CaseIdMismatch { expected: CaseId, actual: CaseId },
    ExitStatusMismatch { expected: i32, actual: i32 },
    ReturnValueMismatch { expected: u64, actual: u64 },
    StdoutMismatch { expected: String, actual: String },
    StderrMismatch { expected: String, actual: String },
}

pub fn compare_observed_results(
    expected: &ObservedResult,
    actual: &ObservedResult,
) -> ComparisonReport {
    let mut issues = Vec::new();

    if expected.case_id() != actual.case_id() {
        issues.push(ComparisonIssue::CaseIdMismatch {
            expected: expected.case_id().clone(),
            actual: actual.case_id().clone(),
        });
    }

    if expected.exit_status() != actual.exit_status() {
        issues.push(ComparisonIssue::ExitStatusMismatch {
            expected: expected.exit_status(),
            actual: actual.exit_status(),
        });
    }

    if expected.return_value() != actual.return_value() {
        issues.push(ComparisonIssue::ReturnValueMismatch {
            expected: expected.return_value(),
            actual: actual.return_value(),
        });
    }

    if expected.stdout() != actual.stdout() {
        issues.push(ComparisonIssue::StdoutMismatch {
            expected: expected.stdout().to_owned(),
            actual: actual.stdout().to_owned(),
        });
    }

    if expected.stderr() != actual.stderr() {
        issues.push(ComparisonIssue::StderrMismatch {
            expected: expected.stderr().to_owned(),
            actual: actual.stderr().to_owned(),
        });
    }

    ComparisonReport::new(issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(return_value: u64) -> ObservedResult {
        ObservedResult::new(
            CaseId::new("return_42").expect("test case id is non-empty"),
            0,
            return_value,
            String::new(),
            String::new(),
        )
    }

    #[test]
    fn matching_observations_have_no_issues() {
        let expected = result(42);
        let actual = result(42);

        assert!(compare_observed_results(&expected, &actual).is_match());
    }

    #[test]
    fn mismatched_return_value_is_reported() {
        let expected = result(42);
        let actual = result(41);

        let report = compare_observed_results(&expected, &actual);

        assert_eq!(
            report.issues(),
            &[ComparisonIssue::ReturnValueMismatch {
                expected: 42,
                actual: 41
            }]
        );
    }

    #[test]
    fn all_m1_observation_fields_are_compared() {
        let expected = ObservedResult::new(
            CaseId::new("expected").expect("test case id is non-empty"),
            0,
            42,
            "stdout".to_owned(),
            "stderr".to_owned(),
        );
        let actual = ObservedResult::new(
            CaseId::new("actual").expect("test case id is non-empty"),
            1,
            41,
            "different stdout".to_owned(),
            "different stderr".to_owned(),
        );

        let report = compare_observed_results(&expected, &actual);

        assert_eq!(
            report.issues(),
            &[
                ComparisonIssue::CaseIdMismatch {
                    expected: CaseId::new("expected").expect("test case id is non-empty"),
                    actual: CaseId::new("actual").expect("test case id is non-empty")
                },
                ComparisonIssue::ExitStatusMismatch {
                    expected: 0,
                    actual: 1
                },
                ComparisonIssue::ReturnValueMismatch {
                    expected: 42,
                    actual: 41
                },
                ComparisonIssue::StdoutMismatch {
                    expected: "stdout".to_owned(),
                    actual: "different stdout".to_owned()
                },
                ComparisonIssue::StderrMismatch {
                    expected: "stderr".to_owned(),
                    actual: "different stderr".to_owned()
                }
            ]
        );
    }
}
