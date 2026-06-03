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

#[cfg(test)]
mod tests {
    use crate::{
        validate_program, BasicBlock, BlockId, Program, Terminator, UnsupportedReason,
        ValidationIssue, ValidationReport, X86Va,
    };

    fn block(id: u32, start: u64, end: u64, terminator: Terminator) -> BasicBlock {
        BasicBlock::new(
            BlockId::new(id),
            X86Va::new(start),
            X86Va::new(end),
            Vec::new(),
            terminator,
        )
        .expect("test block range is valid")
    }

    #[test]
    fn validation_report_is_valid_when_it_has_no_issues() {
        let report = ValidationReport::new(Vec::new());

        assert!(report.is_valid());
        assert!(report.issues().is_empty());
    }

    #[test]
    fn valid_program_has_no_issues() {
        let program = Program::new(X86Va::new(0), vec![block(0, 0, 1, Terminator::Return)])
            .expect("program has entry block");

        assert!(validate_program(&program).is_valid());
    }

    #[test]
    fn overlapping_block_ranges_are_reported() {
        let program = Program::new(
            X86Va::new(0),
            vec![
                block(0, 0, 4, Terminator::Return),
                block(1, 3, 6, Terminator::Return),
            ],
        )
        .expect("program has entry block and unique block ids");

        assert_eq!(
            validate_program(&program).issues(),
            &[ValidationIssue::BlockRangeOverlap {
                first_start: X86Va::new(0),
                first_end: X86Va::new(4),
                second_start: X86Va::new(3),
                second_end: X86Va::new(6)
            }]
        );
    }

    #[test]
    fn unsupported_terminator_is_reported() {
        let reason = UnsupportedReason::MissingReturnTerminator { at: X86Va::new(1) };
        let program = Program::new(
            X86Va::new(0),
            vec![block(
                0,
                0,
                1,
                Terminator::Unsupported {
                    reason: reason.clone(),
                },
            )],
        )
        .expect("program has entry block");

        assert_eq!(
            validate_program(&program).issues(),
            &[ValidationIssue::UnsupportedTerminator { at: X86Va::new(1) }]
        );
    }
}
