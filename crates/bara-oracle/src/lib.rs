pub mod compare;
pub mod json;
pub mod observation;
pub mod report;
pub mod testcase;

pub use compare::{compare_observed_results, ComparisonIssue, ComparisonReport};
pub use json::{
    corpus_report_to_json, observed_result_from_json, observed_result_to_json, JsonError,
};
pub use observation::{CaseId, CaseIdError, ExpectedResult, ObservedResult};
pub use report::{CorpusReport, FailureKind, FailureMessage, FixtureOutcome, FixtureReport};
pub use testcase::{
    test_case_from_json, TestCase, TestCaseAbi, TestCaseHostTrapPlan, TestCaseInputMemory,
    TestCaseInputMemoryError, TestCaseJsonError, TestCaseStdoutTrap, TestCaseStdoutTrapError,
    TestCaseU64,
};
