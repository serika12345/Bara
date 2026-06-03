use crate::{Program, Terminator, X86Va};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationReport {
    issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn new(issues: Vec<ValidationIssue>) -> Self {
        Self { issues }
    }

    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationIssue {
    EmptyProgram,
    BlockRangeOverlap {
        first_start: X86Va,
        first_end: X86Va,
        second_start: X86Va,
        second_end: X86Va,
    },
    UnsupportedTerminator {
        at: X86Va,
    },
}

pub fn validate_program(program: &Program) -> ValidationReport {
    let mut issues = Vec::new();
    let blocks = program.blocks();

    if blocks.is_empty() {
        issues.push(ValidationIssue::EmptyProgram);
    }

    for (left_index, left) in blocks.iter().enumerate() {
        if matches!(left.terminator(), Terminator::Unsupported { .. }) {
            issues.push(ValidationIssue::UnsupportedTerminator { at: left.end() });
        }

        for right in &blocks[(left_index + 1)..] {
            if left.start() < right.end() && right.start() < left.end() {
                issues.push(ValidationIssue::BlockRangeOverlap {
                    first_start: left.start(),
                    first_end: left.end(),
                    second_start: right.start(),
                    second_end: right.end(),
                });
            }
        }
    }

    ValidationReport::new(issues)
}
