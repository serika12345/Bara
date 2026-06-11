use bara_ir::X86Va;
use bara_isa_x86::{DecodeError, X86Bytes};

use crate::{CaseId, CaseIdError, ExpectedResult, ObservedResult, TestCase, TestCaseAbi};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmallCaseSet {
    cases: Vec<SmallCase>,
}

impl SmallCaseSet {
    fn new(cases: Vec<SmallCase>) -> Self {
        Self { cases }
    }

    pub fn cases(&self) -> &[SmallCase] {
        &self.cases
    }

    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmallCase {
    test_case: TestCase,
    expected: ExpectedResult,
}

impl SmallCase {
    fn new(test_case: TestCase, expected: ExpectedResult) -> Self {
        Self {
            test_case,
            expected,
        }
    }

    pub const fn test_case(&self) -> &TestCase {
        &self.test_case
    }

    pub const fn expected(&self) -> &ExpectedResult {
        &self.expected
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmallCaseShrinkPlan {
    candidates: Vec<SmallCase>,
}

impl SmallCaseShrinkPlan {
    fn new(candidates: Vec<SmallCase>) -> Self {
        Self { candidates }
    }

    pub fn candidates(&self) -> &[SmallCase] {
        &self.candidates
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmallCaseError {
    CaseId(CaseIdError),
    DecodeInput(DecodeError),
}

pub fn small_no_args_u64_cases() -> Result<SmallCaseSet, SmallCaseError> {
    [
        SmallNoArgsReturnSpec::new("small_return_0", 0),
        SmallNoArgsReturnSpec::new("small_return_1", 1),
        SmallNoArgsReturnSpec::new("small_return_42", 42),
        SmallNoArgsReturnSpec::new("small_return_255", 255),
    ]
    .into_iter()
    .map(SmallNoArgsReturnSpec::into_small_case)
    .collect::<Result<Vec<_>, _>>()
    .map(SmallCaseSet::new)
}

pub fn shrink_no_args_u64_case(
    test_case: &TestCase,
) -> Result<SmallCaseShrinkPlan, SmallCaseError> {
    if !matches!(test_case.abi(), TestCaseAbi::NoArgsU64) {
        return Ok(SmallCaseShrinkPlan::new(Vec::new()));
    }

    let Some(return_value) = mov_eax_imm32_ret_value(test_case.x86_bytes().bytes()) else {
        return Ok(SmallCaseShrinkPlan::new(Vec::new()));
    };
    if return_value == 0 {
        return Ok(SmallCaseShrinkPlan::new(Vec::new()));
    }

    let candidate_id = format!("{}_shrink_return_0", test_case.case_id().as_str());
    no_args_return_immediate_case(candidate_id, 0)
        .map(|candidate| SmallCaseShrinkPlan::new(vec![candidate]))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SmallNoArgsReturnSpec {
    case_id: &'static str,
    return_value: u32,
}

impl SmallNoArgsReturnSpec {
    const fn new(case_id: &'static str, return_value: u32) -> Self {
        Self {
            case_id,
            return_value,
        }
    }

    fn into_small_case(self) -> Result<SmallCase, SmallCaseError> {
        no_args_return_immediate_case(self.case_id, self.return_value)
    }
}

fn no_args_return_immediate_case(
    case_id: impl Into<String>,
    return_value: u32,
) -> Result<SmallCase, SmallCaseError> {
    let case_id = CaseId::new(case_id).map_err(SmallCaseError::CaseId)?;
    let x86_bytes = X86Bytes::new(X86Va::new(0), mov_eax_imm32_ret(return_value))
        .map_err(SmallCaseError::DecodeInput)?;
    let test_case = TestCase::new(case_id.clone(), x86_bytes, TestCaseAbi::NoArgsU64);
    let expected = ObservedResult::new(
        case_id,
        0,
        u64::from(return_value),
        String::new(),
        String::new(),
    );

    Ok(SmallCase::new(test_case, expected))
}

fn mov_eax_imm32_ret(return_value: u32) -> Vec<u8> {
    let mut bytes = vec![0xb8];
    bytes.extend_from_slice(&return_value.to_le_bytes());
    bytes.push(0xc3);
    bytes
}

fn mov_eax_imm32_ret_value(bytes: &[u8]) -> Option<u32> {
    let [0xb8, b0, b1, b2, b3, 0xc3] = bytes else {
        return None;
    };

    Some(u32::from_le_bytes([*b0, *b1, *b2, *b3]))
}

#[cfg(test)]
mod tests {
    use super::{shrink_no_args_u64_case, small_no_args_u64_cases};

    #[test]
    fn small_no_args_u64_cases_include_expected_final_states() {
        let cases = small_no_args_u64_cases().expect("small cases are internally valid");

        assert!(!cases.is_empty());
        assert_eq!(cases.cases().len(), 4);
        assert_eq!(
            cases.cases()[2].test_case().case_id().as_str(),
            "small_return_42"
        );
        assert_eq!(cases.cases()[2].expected().return_value(), 42);
    }

    #[test]
    fn shrink_plan_replaces_nonzero_return_immediate_with_zero_return() {
        let cases = small_no_args_u64_cases().expect("small cases are internally valid");
        let plan =
            shrink_no_args_u64_case(cases.cases()[2].test_case()).expect("shrink plan is valid");

        assert_eq!(plan.candidates().len(), 1);
        assert_eq!(
            plan.candidates()[0].test_case().case_id().as_str(),
            "small_return_42_shrink_return_0"
        );
        assert_eq!(plan.candidates()[0].expected().return_value(), 0);
    }

    #[test]
    fn shrink_plan_leaves_zero_return_case_minimal() {
        let cases = small_no_args_u64_cases().expect("small cases are internally valid");
        let plan =
            shrink_no_args_u64_case(cases.cases()[0].test_case()).expect("shrink plan is valid");

        assert!(plan.is_empty());
    }
}
