use bara_ir::{Program, X86Va};

use crate::EmittedFunction;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmittedFunctionVerificationReport {
    issues: Vec<EmittedFunctionVerificationIssue>,
}

impl EmittedFunctionVerificationReport {
    fn new(issues: Vec<EmittedFunctionVerificationIssue>) -> Self {
        Self { issues }
    }

    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn issues(&self) -> &[EmittedFunctionVerificationIssue] {
        &self.issues
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmittedFunctionVerificationIssue {
    MissingPcMapSource { source: X86Va },
}

pub fn verify_emitted_function(
    program: &Program,
    emitted: &EmittedFunction,
) -> EmittedFunctionVerificationReport {
    let mut issues = Vec::new();
    let pc_map_sources = emitted
        .pc_map()
        .iter()
        .map(|entry| entry.source())
        .collect::<Vec<_>>();

    for block in program.blocks() {
        if !pc_map_sources.contains(&block.start()) {
            issues.push(EmittedFunctionVerificationIssue::MissingPcMapSource {
                source: block.start(),
            });
        }
    }

    EmittedFunctionVerificationReport::new(issues)
}

#[cfg(test)]
mod tests {
    use bara_ir::{BasicBlock, BlockId, Program, Terminator, X86Va};

    use crate::{Arm64MachineCode, ArmPc, EmittedFunction, PcMapEntry};

    use super::{verify_emitted_function, EmittedFunctionVerificationIssue};

    fn block(id: u32, start: u64, end: u64) -> BasicBlock {
        BasicBlock::new(
            BlockId::new(id),
            X86Va::new(start),
            X86Va::new(end),
            Vec::new(),
            Terminator::Return,
        )
        .expect("test block range is valid")
    }

    #[test]
    fn verifier_accepts_pc_map_covering_every_ir_block_start() {
        let program = Program::new(X86Va::new(0), vec![block(0, 0, 4), block(1, 4, 8)])
            .expect("program has entry block");
        let emitted = EmittedFunction::new(
            Arm64MachineCode::new(vec![0xc0, 0x03, 0x5f, 0xd6]).expect("test code is non-empty"),
            vec![
                PcMapEntry::new(X86Va::new(0), ArmPc::new(0)),
                PcMapEntry::new(X86Va::new(4), ArmPc::new(4)),
            ],
        );

        assert!(verify_emitted_function(&program, &emitted).is_valid());
    }

    #[test]
    fn verifier_reports_ir_block_start_missing_from_pc_map() {
        let program = Program::new(X86Va::new(0), vec![block(0, 0, 4), block(1, 4, 8)])
            .expect("program has entry block");
        let emitted = EmittedFunction::new(
            Arm64MachineCode::new(vec![0xc0, 0x03, 0x5f, 0xd6]).expect("test code is non-empty"),
            vec![PcMapEntry::new(X86Va::new(0), ArmPc::new(0))],
        );

        assert_eq!(
            verify_emitted_function(&program, &emitted).issues(),
            &[EmittedFunctionVerificationIssue::MissingPcMapSource {
                source: X86Va::new(4)
            }]
        );
    }
}
