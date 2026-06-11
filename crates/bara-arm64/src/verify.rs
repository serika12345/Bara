use bara_ir::{validate_program, Program, ValidationIssue, X86Va};

use crate::{ArmPc, EmittedFunction};

const ARM64_INSTRUCTION_BYTES: usize = 4;

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
    IrInvariant { issue: ValidationIssue },
    MissingPcMapSource { source: X86Va },
    FixupTargetMissingPcMapSource { target: X86Va },
    FixupOffsetOutOfCode { offset: ArmPc },
    FixupSourceOutOfCode { source: ArmPc },
}

pub fn verify_emitted_function(
    program: &Program,
    emitted: &EmittedFunction,
) -> EmittedFunctionVerificationReport {
    let mut issues = Vec::new();

    for issue in validate_program(program).issues() {
        issues.push(EmittedFunctionVerificationIssue::IrInvariant {
            issue: issue.clone(),
        });
    }

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

    let code_len = emitted.code().bytes().len();
    for fixup in emitted.branch_fixups() {
        if !pc_map_sources.contains(&fixup.target()) {
            issues.push(
                EmittedFunctionVerificationIssue::FixupTargetMissingPcMapSource {
                    target: fixup.target(),
                },
            );
        }

        if !arm64_instruction_slot_exists(fixup.offset(), code_len) {
            issues.push(EmittedFunctionVerificationIssue::FixupOffsetOutOfCode {
                offset: fixup.offset(),
            });
        }

        if !arm64_instruction_slot_exists(fixup.source(), code_len) {
            issues.push(EmittedFunctionVerificationIssue::FixupSourceOutOfCode {
                source: fixup.source(),
            });
        }
    }

    EmittedFunctionVerificationReport::new(issues)
}

fn arm64_instruction_slot_exists(pc: ArmPc, code_len: usize) -> bool {
    let Ok(offset) = usize::try_from(pc.value()) else {
        return false;
    };

    match offset.checked_add(ARM64_INSTRUCTION_BYTES) {
        Some(end) => end <= code_len,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use bara_ir::{
        BasicBlock, BlockId, Program, Terminator, UnsupportedReason, ValidationIssue, X86Va,
    };

    use crate::{
        Arm64MachineCode, ArmPc, BranchFixup, BranchFixupKind, EmittedFunction,
        EmittedHostTrapRequests, PcMapEntry,
    };

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

    #[test]
    fn verifier_reports_ir_invariant_issues() {
        let program = Program::new(
            X86Va::new(0),
            vec![BasicBlock::new(
                BlockId::new(0),
                X86Va::new(0),
                X86Va::new(1),
                Vec::new(),
                Terminator::Unsupported {
                    reason: UnsupportedReason::MissingReturnTerminator { at: X86Va::new(1) },
                },
            )
            .expect("test block range is valid")],
        )
        .expect("program has entry block");
        let emitted = EmittedFunction::new(
            Arm64MachineCode::new(vec![0xc0, 0x03, 0x5f, 0xd6]).expect("test code is non-empty"),
            vec![PcMapEntry::new(X86Va::new(0), ArmPc::new(0))],
        );

        assert_eq!(
            verify_emitted_function(&program, &emitted).issues(),
            &[EmittedFunctionVerificationIssue::IrInvariant {
                issue: ValidationIssue::UnsupportedTerminator { at: X86Va::new(1) }
            }]
        );
    }

    #[test]
    fn verifier_accepts_branch_fixup_targeting_pc_map_source() {
        let program = Program::new(X86Va::new(0), vec![block(0, 0, 4), block(1, 4, 8)])
            .expect("program has entry block");
        let emitted = EmittedFunction::with_metadata(
            Arm64MachineCode::new(vec![0, 0, 0, 0, 0xc0, 0x03, 0x5f, 0xd6])
                .expect("test code is non-empty"),
            vec![
                PcMapEntry::new(X86Va::new(0), ArmPc::new(0)),
                PcMapEntry::new(X86Va::new(4), ArmPc::new(4)),
            ],
            vec![BranchFixup::for_test(
                ArmPc::new(0),
                ArmPc::new(0),
                X86Va::new(4),
                BranchFixupKind::Unconditional,
            )],
            EmittedHostTrapRequests::none(),
        );

        assert!(verify_emitted_function(&program, &emitted).is_valid());
    }

    #[test]
    fn verifier_reports_fixup_target_missing_from_pc_map() {
        let program =
            Program::new(X86Va::new(0), vec![block(0, 0, 4)]).expect("program has entry block");
        let emitted = EmittedFunction::with_metadata(
            Arm64MachineCode::new(vec![0, 0, 0, 0]).expect("test code is non-empty"),
            vec![PcMapEntry::new(X86Va::new(0), ArmPc::new(0))],
            vec![BranchFixup::for_test(
                ArmPc::new(0),
                ArmPc::new(0),
                X86Va::new(8),
                BranchFixupKind::Unconditional,
            )],
            EmittedHostTrapRequests::none(),
        );

        assert_eq!(
            verify_emitted_function(&program, &emitted).issues(),
            &[
                EmittedFunctionVerificationIssue::FixupTargetMissingPcMapSource {
                    target: X86Va::new(8)
                }
            ]
        );
    }

    #[test]
    fn verifier_reports_fixup_arm_pcs_outside_code() {
        let program =
            Program::new(X86Va::new(0), vec![block(0, 0, 4)]).expect("program has entry block");
        let emitted = EmittedFunction::with_metadata(
            Arm64MachineCode::new(vec![0, 0, 0, 0]).expect("test code is non-empty"),
            vec![PcMapEntry::new(X86Va::new(0), ArmPc::new(0))],
            vec![BranchFixup::for_test(
                ArmPc::new(4),
                ArmPc::new(8),
                X86Va::new(0),
                BranchFixupKind::Unconditional,
            )],
            EmittedHostTrapRequests::none(),
        );

        assert_eq!(
            verify_emitted_function(&program, &emitted).issues(),
            &[
                EmittedFunctionVerificationIssue::FixupOffsetOutOfCode {
                    offset: ArmPc::new(4)
                },
                EmittedFunctionVerificationIssue::FixupSourceOutOfCode {
                    source: ArmPc::new(8)
                }
            ]
        );
    }
}
