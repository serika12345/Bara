use std::{error::Error, fmt};

use bara_arm64::{
    emit_program, verify_emitted_function, BranchFixup, BranchFixupKind,
    EmittedFunctionVerificationIssue, EmittedFunctionVerificationReport, EmittedHostTrapRequests,
    PcMapEntry,
};
use bara_ir::{
    ExternalImportTarget, Program, PublicDyldSymbol, PublicLibcSymbol, PublicSymbolImport,
    SyscallAbi, UnsupportedReason,
};
use bara_isa_x86::{decode_function, lift_decoded_function_with_image_metadata};
use bara_oracle::{
    FailureKind, MachOEntryFunctionInput, ObservedResult, TestCase, TestCaseAbi, TestCaseStackState,
};
use bara_runtime::{
    run_no_args_u64_with_host_traps, run_one_input_memory_ptr, run_one_u64, HostTrapPlan,
    InputMemory, InputMemoryError, RunArgumentU64, RunError, RunStdout, RunStdoutError,
};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionCompileResult {
    program: Program,
    emitted: bara_arm64::EmittedFunction,
}

impl FunctionCompileResult {
    fn new(program: Program, emitted: bara_arm64::EmittedFunction) -> Self {
        Self { program, emitted }
    }

    pub(crate) fn arm64_bytes(&self) -> FunctionArm64Bytes<'_> {
        FunctionArm64Bytes::new(self.emitted.code())
    }

    fn emitted(&self) -> &bara_arm64::EmittedFunction {
        &self.emitted
    }

    pub(crate) fn stdout_host_trap_request(&self) -> FunctionStdoutHostTrapRequest {
        FunctionStdoutHostTrapRequest::new(self.emitted.host_trap_requests().stdout_requested())
    }

    pub(crate) fn artifact_metadata(&self, source: &TestCase) -> FunctionArtifactMetadata {
        FunctionArtifactMetadata::from_compile_result(source, self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FunctionArm64Bytes<'a>(&'a bara_arm64::Arm64MachineCode);

impl<'a> FunctionArm64Bytes<'a> {
    const fn new(code: &'a bara_arm64::Arm64MachineCode) -> Self {
        Self(code)
    }

    pub(crate) fn as_slice(self) -> &'a [u8] {
        self.0.bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FunctionStdoutHostTrapRequest {
    requested: bool,
}

impl FunctionStdoutHostTrapRequest {
    const fn new(requested: bool) -> Self {
        Self { requested }
    }

    pub(crate) const fn is_requested(self) -> bool {
        self.requested
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionArtifactMetadata {
    compiled_ir: FunctionCompiledIrArtifact,
    pcmap: FunctionPcMapArtifact,
    fixups: FunctionFixupsArtifact,
    helpers: FunctionHelpersArtifact,
    artifact_report: FunctionArtifactReport,
    verifier_report: FunctionVerifierReportArtifact,
}

impl FunctionArtifactMetadata {
    fn from_compile_result(source: &TestCase, result: &FunctionCompileResult) -> Self {
        Self {
            compiled_ir: FunctionCompiledIrArtifact::from_program(&result.program),
            pcmap: FunctionPcMapArtifact::from_entries(result.emitted.pc_map()),
            fixups: FunctionFixupsArtifact::from_fixups(result.emitted.branch_fixups()),
            helpers: FunctionHelpersArtifact::from_requests(result.emitted.host_trap_requests()),
            artifact_report: FunctionArtifactReport::from_source_and_compile_result(source, result),
            verifier_report: FunctionVerifierReportArtifact::from_report(&verify_emitted_function(
                &result.program,
                &result.emitted,
            )),
        }
    }

    pub(crate) const fn compiled_ir(&self) -> &FunctionCompiledIrArtifact {
        &self.compiled_ir
    }

    pub(crate) const fn pcmap(&self) -> &FunctionPcMapArtifact {
        &self.pcmap
    }

    pub(crate) const fn fixups(&self) -> &FunctionFixupsArtifact {
        &self.fixups
    }

    pub(crate) const fn helpers(&self) -> &FunctionHelpersArtifact {
        &self.helpers
    }

    pub(crate) const fn artifact_report(&self) -> &FunctionArtifactReport {
        &self.artifact_report
    }

    pub(crate) const fn verifier_report(&self) -> &FunctionVerifierReportArtifact {
        &self.verifier_report
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionCompiledIrArtifact {
    entry: u64,
    blocks: Vec<FunctionIrBlockArtifact>,
}

impl FunctionCompiledIrArtifact {
    fn from_program(program: &Program) -> Self {
        Self {
            entry: program.entry().value(),
            blocks: program
                .blocks()
                .iter()
                .map(FunctionIrBlockArtifact::from_block)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionIrBlockArtifact {
    id: u32,
    start: u64,
    end: u64,
    ops: Vec<FunctionIrOpArtifact>,
    terminator: FunctionTerminatorArtifact,
}

impl FunctionIrBlockArtifact {
    fn from_block(block: &bara_ir::BasicBlock) -> Self {
        Self {
            id: block.id().value(),
            start: block.start().value(),
            end: block.end().value(),
            ops: block
                .ops()
                .iter()
                .map(FunctionIrOpArtifact::from_op)
                .collect(),
            terminator: FunctionTerminatorArtifact::from_terminator(block.terminator()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionIrOpArtifact {
    Mov {
        dst: FunctionOperandArtifact,
        src: FunctionOperandArtifact,
    },
    Add {
        dst: FunctionOperandArtifact,
        src: FunctionOperandArtifact,
    },
    Sub {
        dst: FunctionOperandArtifact,
        src: FunctionOperandArtifact,
    },
    Cmp {
        lhs: FunctionOperandArtifact,
        rhs: FunctionOperandArtifact,
    },
    Test {
        lhs: FunctionOperandArtifact,
        rhs: FunctionOperandArtifact,
    },
    Push {
        src: FunctionOperandArtifact,
    },
    Pop {
        dst: FunctionOperandArtifact,
    },
    HostTrap {
        trap: FunctionHostTrapArtifact,
    },
    Unsupported {
        reason: FunctionUnsupportedReasonArtifact,
    },
}

impl FunctionIrOpArtifact {
    fn from_op(op: &bara_ir::IrOp) -> Self {
        match op {
            bara_ir::IrOp::Mov { dst, src } => Self::Mov {
                dst: FunctionOperandArtifact::from_operand(dst),
                src: FunctionOperandArtifact::from_operand(src),
            },
            bara_ir::IrOp::Add { dst, src } => Self::Add {
                dst: FunctionOperandArtifact::from_operand(dst),
                src: FunctionOperandArtifact::from_operand(src),
            },
            bara_ir::IrOp::Sub { dst, src } => Self::Sub {
                dst: FunctionOperandArtifact::from_operand(dst),
                src: FunctionOperandArtifact::from_operand(src),
            },
            bara_ir::IrOp::Cmp { lhs, rhs } => Self::Cmp {
                lhs: FunctionOperandArtifact::from_operand(lhs),
                rhs: FunctionOperandArtifact::from_operand(rhs),
            },
            bara_ir::IrOp::Test { lhs, rhs } => Self::Test {
                lhs: FunctionOperandArtifact::from_operand(lhs),
                rhs: FunctionOperandArtifact::from_operand(rhs),
            },
            bara_ir::IrOp::Push { src } => Self::Push {
                src: FunctionOperandArtifact::from_operand(src),
            },
            bara_ir::IrOp::Pop { dst } => Self::Pop {
                dst: FunctionOperandArtifact::from_operand(dst),
            },
            bara_ir::IrOp::HostTrap { kind } => Self::HostTrap {
                trap: FunctionHostTrapArtifact::from_ir(*kind),
            },
            bara_ir::IrOp::Unsupported { reason } => Self::Unsupported {
                reason: FunctionUnsupportedReasonArtifact::from_ir(reason),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionOperandArtifact {
    Reg { reg: FunctionRegisterArtifact },
    ImmU64 { value: u64 },
    Mem8 { base: FunctionRegisterArtifact },
}

impl FunctionOperandArtifact {
    fn from_operand(operand: &bara_ir::Operand) -> Self {
        match operand {
            bara_ir::Operand::Reg(reg) => Self::Reg {
                reg: FunctionRegisterArtifact::from_ir(*reg),
            },
            bara_ir::Operand::ImmU64(value) => Self::ImmU64 { value: *value },
            bara_ir::Operand::Mem8 { base } => Self::Mem8 {
                base: FunctionRegisterArtifact::from_ir(*base),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionRegisterArtifact {
    Rax,
    Rdi,
}

impl FunctionRegisterArtifact {
    const fn from_ir(reg: bara_ir::X86Reg) -> Self {
        match reg {
            bara_ir::X86Reg::Rax => Self::Rax,
            bara_ir::X86Reg::Rdi => Self::Rdi,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionTerminatorArtifact {
    Return,
    Fallthrough {
        target: u64,
    },
    DirectJump {
        target: u64,
    },
    DirectCall {
        target: u64,
        return_to: u64,
    },
    CondJump {
        condition: FunctionConditionArtifact,
        taken: u64,
        fallthrough: u64,
    },
    BoundaryRequest {
        request: FunctionBoundaryRequestArtifact,
    },
    Unsupported {
        reason: FunctionUnsupportedReasonArtifact,
    },
}

impl FunctionTerminatorArtifact {
    fn from_terminator(terminator: &bara_ir::Terminator) -> Self {
        match terminator {
            bara_ir::Terminator::Return => Self::Return,
            bara_ir::Terminator::Fallthrough { target } => Self::Fallthrough {
                target: target.value(),
            },
            bara_ir::Terminator::DirectJump { target } => Self::DirectJump {
                target: target.value(),
            },
            bara_ir::Terminator::DirectCall { target, return_to } => Self::DirectCall {
                target: target.value(),
                return_to: return_to.value(),
            },
            bara_ir::Terminator::CondJump {
                condition,
                taken,
                fallthrough,
            } => Self::CondJump {
                condition: FunctionConditionArtifact::from_ir(*condition),
                taken: taken.value(),
                fallthrough: fallthrough.value(),
            },
            bara_ir::Terminator::BoundaryRequest { request } => Self::BoundaryRequest {
                request: FunctionBoundaryRequestArtifact::from_ir(request),
            },
            bara_ir::Terminator::Unsupported { reason } => Self::Unsupported {
                reason: FunctionUnsupportedReasonArtifact::from_ir(reason),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionBoundaryRequestArtifact {
    Syscall,
    Helper,
}

impl FunctionBoundaryRequestArtifact {
    const fn from_ir(request: &bara_ir::BoundaryRequest) -> Self {
        match request {
            bara_ir::BoundaryRequest::Syscall(_) => Self::Syscall,
            bara_ir::BoundaryRequest::Helper(_) => Self::Helper,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionUnsupportedReasonArtifact {
    Unsupported,
}

impl FunctionUnsupportedReasonArtifact {
    const fn from_ir(_reason: &UnsupportedReason) -> Self {
        Self::Unsupported
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionHostTrapArtifact {
    Stdout,
}

impl FunctionHostTrapArtifact {
    const fn from_ir(kind: bara_ir::HostTrapKind) -> Self {
        match kind {
            bara_ir::HostTrapKind::Stdout => Self::Stdout,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionConditionArtifact {
    Overflow,
    NotOverflow,
    Below,
    AboveOrEqual,
    Equal,
    NotEqual,
    BelowOrEqual,
    Above,
    Sign,
    NotSign,
    Parity,
    NotParity,
    Less,
    GreaterOrEqual,
    LessOrEqual,
    Greater,
}

impl FunctionConditionArtifact {
    const fn from_ir(condition: bara_ir::X86Cond) -> Self {
        match condition {
            bara_ir::X86Cond::Overflow => Self::Overflow,
            bara_ir::X86Cond::NotOverflow => Self::NotOverflow,
            bara_ir::X86Cond::Below => Self::Below,
            bara_ir::X86Cond::AboveOrEqual => Self::AboveOrEqual,
            bara_ir::X86Cond::Equal => Self::Equal,
            bara_ir::X86Cond::NotEqual => Self::NotEqual,
            bara_ir::X86Cond::BelowOrEqual => Self::BelowOrEqual,
            bara_ir::X86Cond::Above => Self::Above,
            bara_ir::X86Cond::Sign => Self::Sign,
            bara_ir::X86Cond::NotSign => Self::NotSign,
            bara_ir::X86Cond::Parity => Self::Parity,
            bara_ir::X86Cond::NotParity => Self::NotParity,
            bara_ir::X86Cond::Less => Self::Less,
            bara_ir::X86Cond::GreaterOrEqual => Self::GreaterOrEqual,
            bara_ir::X86Cond::LessOrEqual => Self::LessOrEqual,
            bara_ir::X86Cond::Greater => Self::Greater,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionPcMapArtifact {
    entries: Vec<FunctionPcMapEntryArtifact>,
}

impl FunctionPcMapArtifact {
    fn from_entries(entries: &[PcMapEntry]) -> Self {
        Self {
            entries: entries
                .iter()
                .map(FunctionPcMapEntryArtifact::from_entry)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionPcMapEntryArtifact {
    source: u64,
    target: u64,
}

impl FunctionPcMapEntryArtifact {
    const fn from_entry(entry: &PcMapEntry) -> Self {
        Self {
            source: entry.source().value(),
            target: entry.target().value(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionFixupsArtifact {
    fixups: Vec<FunctionFixupArtifact>,
}

impl FunctionFixupsArtifact {
    fn from_fixups(fixups: &[BranchFixup]) -> Self {
        Self {
            fixups: fixups
                .iter()
                .map(FunctionFixupArtifact::from_fixup)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionFixupArtifact {
    offset: u64,
    source: u64,
    target: u64,
    kind: FunctionFixupKindArtifact,
}

impl FunctionFixupArtifact {
    const fn from_fixup(fixup: &BranchFixup) -> Self {
        Self {
            offset: fixup.offset().value(),
            source: fixup.source().value(),
            target: fixup.target().value(),
            kind: FunctionFixupKindArtifact::from_kind(fixup.kind()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionFixupKindArtifact {
    Unconditional,
    Call,
    Conditional {
        condition: FunctionConditionArtifact,
    },
}

impl FunctionFixupKindArtifact {
    const fn from_kind(kind: BranchFixupKind) -> Self {
        match kind {
            BranchFixupKind::Unconditional => Self::Unconditional,
            BranchFixupKind::Call => Self::Call,
            BranchFixupKind::Conditional { condition } => Self::Conditional {
                condition: FunctionConditionArtifact::from_ir(condition),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionHelpersArtifact {
    helpers: Vec<FunctionHelperArtifact>,
}

impl FunctionHelpersArtifact {
    fn from_requests(requests: &EmittedHostTrapRequests) -> Self {
        let mut helpers = Vec::new();
        if requests.stdout_requested() {
            helpers.push(FunctionHelperArtifact::WriteStdout);
        }

        Self { helpers }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionHelperArtifact {
    WriteStdout,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionVerifierReportArtifact {
    issues: Vec<FunctionVerifierIssueArtifact>,
}

impl FunctionVerifierReportArtifact {
    fn from_report(report: &EmittedFunctionVerificationReport) -> Self {
        Self {
            issues: report
                .issues()
                .iter()
                .map(FunctionVerifierIssueArtifact::from_issue)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionVerifierIssueArtifact {
    MissingPcMapSource { source: u64 },
    FixupTargetMissingPcMapSource { target: u64 },
    FixupOffsetOutOfCode { offset: u64 },
    FixupSourceOutOfCode { source: u64 },
}

impl FunctionVerifierIssueArtifact {
    const fn from_issue(issue: &EmittedFunctionVerificationIssue) -> Self {
        match issue {
            EmittedFunctionVerificationIssue::MissingPcMapSource { source } => {
                Self::MissingPcMapSource {
                    source: source.value(),
                }
            }
            EmittedFunctionVerificationIssue::FixupTargetMissingPcMapSource { target } => {
                Self::FixupTargetMissingPcMapSource {
                    target: target.value(),
                }
            }
            EmittedFunctionVerificationIssue::FixupOffsetOutOfCode { offset } => {
                Self::FixupOffsetOutOfCode {
                    offset: offset.value(),
                }
            }
            EmittedFunctionVerificationIssue::FixupSourceOutOfCode { source } => {
                Self::FixupSourceOutOfCode {
                    source: source.value(),
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionArtifactReport {
    state_layout: FunctionStateLayoutArtifact,
    cache_validation_identity: FunctionCacheValidationIdentityArtifact,
    helper_requirements: Vec<FunctionHelperRequirementArtifact>,
}

impl FunctionArtifactReport {
    fn from_source_and_compile_result(source: &TestCase, result: &FunctionCompileResult) -> Self {
        Self {
            state_layout: FunctionStateLayoutArtifact::from_source(source),
            cache_validation_identity: FunctionCacheValidationIdentityArtifact::from_source(source),
            helper_requirements: FunctionHelperRequirementsArtifact::from_requests(
                result.emitted.host_trap_requests(),
            )
            .into_values(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionStateLayoutArtifact {
    kind: FunctionStateLayoutKindArtifact,
    source_isa: FunctionSourceIsaArtifact,
    target_isa: FunctionTargetIsaArtifact,
    abi: FunctionAbiArtifact,
    return_register: FunctionRegisterArtifact,
    stack: FunctionStackLayoutArtifact,
}

impl FunctionStateLayoutArtifact {
    fn from_source(source: &TestCase) -> Self {
        Self {
            kind: FunctionStateLayoutKindArtifact::FunctionLevelV0,
            source_isa: FunctionSourceIsaArtifact::X8664,
            target_isa: FunctionTargetIsaArtifact::Arm64,
            abi: FunctionAbiArtifact::from_test_case_abi(source.abi()),
            return_register: FunctionRegisterArtifact::Rax,
            stack: FunctionStackLayoutArtifact::from_stack_state(source.stack_state()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionStateLayoutKindArtifact {
    FunctionLevelV0,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum FunctionSourceIsaArtifact {
    #[serde(rename = "x86_64")]
    X8664,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionTargetIsaArtifact {
    Arm64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionAbiArtifact {
    args: Vec<FunctionAbiArgumentArtifact>,
    #[serde(rename = "return")]
    return_value: FunctionAbiReturnArtifact,
}

impl FunctionAbiArtifact {
    fn from_test_case_abi(abi: &TestCaseAbi) -> Self {
        let args = match abi {
            TestCaseAbi::NoArgsU64 => Vec::new(),
            TestCaseAbi::OneU64ArgReturnsU64 { .. } => {
                vec![FunctionAbiArgumentArtifact::U64]
            }
            TestCaseAbi::OneInputMemoryPtrReturnsU64 { .. } => {
                vec![FunctionAbiArgumentArtifact::Ptr]
            }
        };

        Self {
            args,
            return_value: FunctionAbiReturnArtifact::U64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionAbiArgumentArtifact {
    U64,
    Ptr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionAbiReturnArtifact {
    U64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionStackLayoutArtifact {
    None,
    Fixed { size: u64 },
}

impl FunctionStackLayoutArtifact {
    fn from_stack_state(stack_state: &TestCaseStackState) -> Self {
        match stack_state.size() {
            Some(size) => Self::Fixed {
                size: size.byte_count().get(),
            },
            None => Self::None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionCacheValidationIdentityArtifact {
    kind: FunctionCacheValidationIdentityKindArtifact,
    case_id: String,
    source_entry: u64,
    source_bytes: String,
    source_abi: FunctionAbiArtifact,
    target_backend: FunctionTargetBackendArtifact,
}

impl FunctionCacheValidationIdentityArtifact {
    fn from_source(source: &TestCase) -> Self {
        Self {
            kind: FunctionCacheValidationIdentityKindArtifact::FixtureFunctionV0,
            case_id: source.case_id().as_str().to_owned(),
            source_entry: source.x86_bytes().entry().value(),
            source_bytes: encode_lower_hex(source.x86_bytes().bytes()),
            source_abi: FunctionAbiArtifact::from_test_case_abi(source.abi()),
            target_backend: FunctionTargetBackendArtifact::BaraArm64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionCacheValidationIdentityKindArtifact {
    FixtureFunctionV0,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum FunctionTargetBackendArtifact {
    #[serde(rename = "bara-arm64")]
    BaraArm64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionHelperRequirementsArtifact {
    values: Vec<FunctionHelperRequirementArtifact>,
}

impl FunctionHelperRequirementsArtifact {
    fn from_requests(requests: &EmittedHostTrapRequests) -> Self {
        let mut values = Vec::new();
        if requests.stdout_requested() {
            values.push(FunctionHelperRequirementArtifact::write_stdout());
        }

        Self { values }
    }

    fn into_values(self) -> Vec<FunctionHelperRequirementArtifact> {
        self.values
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionHelperRequirementArtifact {
    name: FunctionHelperNameArtifact,
    signature: FunctionHelperSignatureArtifact,
}

impl FunctionHelperRequirementArtifact {
    const fn write_stdout() -> Self {
        Self {
            name: FunctionHelperNameArtifact::WriteStdout,
            signature: FunctionHelperSignatureArtifact::PtrLenToUnit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionHelperNameArtifact {
    WriteStdout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionHelperSignatureArtifact {
    PtrLenToUnit,
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionRunResult {
    return_value: FunctionReturnValue,
    stdout: FunctionStdout,
}

impl FunctionRunResult {
    fn from_runtime(result: &bara_runtime::RunResult) -> Self {
        Self {
            return_value: FunctionReturnValue::from_runtime(result),
            stdout: FunctionStdout::from_runtime(result),
        }
    }

    pub(crate) fn into_observed_result(self, case_id: bara_oracle::CaseId) -> ObservedResult {
        ObservedResult::new(
            case_id,
            0,
            self.return_value.into_raw(),
            self.stdout.into_text(),
            String::new(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionReturnValue(u64);

impl FunctionReturnValue {
    fn from_runtime(result: &bara_runtime::RunResult) -> Self {
        Self(result.return_value())
    }

    fn into_raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionStdout(String);

impl FunctionStdout {
    fn from_runtime(result: &bara_runtime::RunResult) -> Self {
        Self(result.stdout().to_owned())
    }

    fn into_text(self) -> String {
        self.0
    }
}

#[derive(Debug)]
pub(crate) enum FunctionRunError {
    Decode(bara_isa_x86::DecodeError),
    Lift(bara_isa_x86::LiftError),
    Emit(bara_arm64::EmitError),
    StandaloneArtifact(FunctionStandaloneArtifactError),
    InputMemory(InputMemoryError),
    StdoutTrap(RunStdoutError),
    Run(RunError),
}

impl FunctionRunError {
    pub(crate) const fn failure_kind(&self) -> FailureKind {
        match self {
            Self::Decode(_) => FailureKind::DecodeError,
            Self::Lift(_) => FailureKind::LiftError,
            Self::Emit(_) => FailureKind::EmitError,
            Self::StandaloneArtifact(_) => FailureKind::EmitError,
            Self::InputMemory(_) | Self::StdoutTrap(_) | Self::Run(_) => FailureKind::RunError,
        }
    }
}

impl fmt::Display for FunctionRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "decode error: {error:?}"),
            Self::Lift(error) => write!(formatter, "lift error: {error:?}"),
            Self::Emit(error) => write_function_emit_error(formatter, error),
            Self::StandaloneArtifact(error) => {
                write!(formatter, "standalone artifact error: {error}")
            }
            Self::InputMemory(error) => write!(formatter, "input memory error: {error:?}"),
            Self::StdoutTrap(error) => write!(formatter, "stdout trap error: {error:?}"),
            Self::Run(error) => write!(formatter, "run error: {error:?}"),
        }
    }
}

impl Error for FunctionRunError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FunctionUnsupportedBoundaryReport {
    status: FunctionUnsupportedBoundaryStatus,
    failure_kind: FailureKind,
    boundary: FunctionUnsupportedBoundary,
}

impl FunctionUnsupportedBoundaryReport {
    fn from_emit_error(error: &bara_arm64::EmitError) -> Option<Self> {
        match error {
            bara_arm64::EmitError::UnsupportedIr { reason } => {
                Self::from_unsupported_reason(reason)
            }
            bara_arm64::EmitError::InvalidProgram
            | bara_arm64::EmitError::EmptyCode
            | bara_arm64::EmitError::UnsupportedShape => None,
        }
    }

    fn from_unsupported_reason(reason: &UnsupportedReason) -> Option<Self> {
        let boundary = match reason {
            UnsupportedReason::SyscallUnsupported { request } => {
                FunctionUnsupportedBoundary::Syscall {
                    abi: FunctionSyscallAbi::from_ir(request.abi()),
                    at: request.at().value(),
                    return_to: request.return_to().value(),
                }
            }
            UnsupportedReason::ExternalCallUnsupported { request } => {
                FunctionUnsupportedBoundary::ExternalCall {
                    symbol_id: request.symbol().value(),
                    import_target: FunctionExternalImportTarget::from_ir(request.import().target()),
                    call_site: request.call_site().value(),
                    return_to: request.return_to().value(),
                }
            }
            UnsupportedReason::DecodeUnsupportedOpcode { .. }
            | UnsupportedReason::MissingReturnTerminator { .. }
            | UnsupportedReason::DirectCallUnsupported { .. }
            | UnsupportedReason::EmitUnsupportedIr => return None,
        };

        Some(Self {
            status: FunctionUnsupportedBoundaryStatus::UnsupportedBoundary,
            failure_kind: FailureKind::EmitError,
            boundary,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionUnsupportedBoundaryStatus {
    UnsupportedBoundary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionUnsupportedBoundary {
    Syscall {
        abi: FunctionSyscallAbi,
        at: u64,
        return_to: u64,
    },
    ExternalCall {
        symbol_id: u32,
        import_target: FunctionExternalImportTarget,
        call_site: u64,
        return_to: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum FunctionSyscallAbi {
    #[serde(rename = "x86_64")]
    X86_64,
}

impl FunctionSyscallAbi {
    const fn from_ir(abi: SyscallAbi) -> Self {
        match abi {
            SyscallAbi::X86_64 => Self::X86_64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FunctionExternalImportTarget {
    Unresolved,
    PublicSymbol {
        namespace: FunctionPublicSymbolNamespace,
        symbol: FunctionPublicSymbolName,
    },
}

impl FunctionExternalImportTarget {
    const fn from_ir(target: ExternalImportTarget) -> Self {
        match target {
            ExternalImportTarget::Unresolved => Self::Unresolved,
            ExternalImportTarget::PublicSymbol(import) => match import {
                PublicSymbolImport::Libc(symbol) => Self::PublicSymbol {
                    namespace: FunctionPublicSymbolNamespace::Libc,
                    symbol: FunctionPublicSymbolName::from_libc(symbol),
                },
                PublicSymbolImport::Dyld(symbol) => Self::PublicSymbol {
                    namespace: FunctionPublicSymbolNamespace::Dyld,
                    symbol: FunctionPublicSymbolName::from_dyld(symbol),
                },
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionPublicSymbolNamespace {
    Libc,
    Dyld,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FunctionPublicSymbolName {
    Puts,
    Write,
    DyldStubBinder,
}

impl FunctionPublicSymbolName {
    const fn from_libc(symbol: PublicLibcSymbol) -> Self {
        match symbol {
            PublicLibcSymbol::Puts => Self::Puts,
            PublicLibcSymbol::Write => Self::Write,
        }
    }

    const fn from_dyld(symbol: PublicDyldSymbol) -> Self {
        match symbol {
            PublicDyldSymbol::DyldStubBinder => Self::DyldStubBinder,
        }
    }
}

fn write_function_emit_error(
    formatter: &mut fmt::Formatter<'_>,
    error: &bara_arm64::EmitError,
) -> fmt::Result {
    if let Some(report) = FunctionUnsupportedBoundaryReport::from_emit_error(error) {
        return write_function_unsupported_boundary_report(formatter, &report);
    }

    write!(formatter, "emit error: {error:?}")
}

fn write_function_unsupported_boundary_report(
    formatter: &mut fmt::Formatter<'_>,
    report: &FunctionUnsupportedBoundaryReport,
) -> fmt::Result {
    match serde_json::to_string(report) {
        Ok(json) => formatter.write_str(&json),
        Err(_) => Err(fmt::Error),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FunctionStandaloneArtifactError {
    HostTrapRequested,
}

impl fmt::Display for FunctionStandaloneArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostTrapRequested => write!(
                formatter,
                "host trap requested by testcase; standalone ARM64 artifact is unsupported"
            ),
        }
    }
}

impl Error for FunctionStandaloneArtifactError {}

pub(crate) fn compile_test_case_function(
    test_case: &TestCase,
) -> Result<FunctionCompileResult, FunctionRunError> {
    let input = test_case.x86_bytes().clone();
    let decoded = decode_function(&input).map_err(FunctionRunError::Decode)?;
    let program =
        lift_decoded_function_with_image_metadata(&decoded, bara_ir::ProgramImageMetadata::empty())
            .map_err(FunctionRunError::Lift)?;
    let emitted = emit_program(&program).map_err(FunctionRunError::Emit)?;

    Ok(FunctionCompileResult::new(program, emitted))
}

pub(crate) fn compile_mach_o_entry_function(
    entry_function: &MachOEntryFunctionInput,
) -> Result<FunctionCompileResult, FunctionRunError> {
    let input = entry_function.test_case().x86_bytes().clone();
    let decoded = decode_function(&input).map_err(FunctionRunError::Decode)?;
    let program = lift_decoded_function_with_image_metadata(
        &decoded,
        entry_function.program_image_metadata().clone(),
    )
    .map_err(FunctionRunError::Lift)?;
    let emitted = emit_program(&program).map_err(FunctionRunError::Emit)?;

    Ok(FunctionCompileResult::new(program, emitted))
}

pub(crate) fn compile_test_case_function_standalone_artifact(
    test_case: &TestCase,
) -> Result<FunctionCompileResult, FunctionRunError> {
    let compiled = compile_test_case_function(test_case)?;
    if !test_case.host_trap_plan().is_empty()
        || compiled.emitted().host_trap_requests().stdout_requested()
    {
        return Err(FunctionRunError::StandaloneArtifact(
            FunctionStandaloneArtifactError::HostTrapRequested,
        ));
    }

    Ok(compiled)
}

pub(crate) fn compile_mach_o_entry_function_standalone_artifact(
    entry_function: &MachOEntryFunctionInput,
) -> Result<FunctionCompileResult, FunctionRunError> {
    let compiled = compile_mach_o_entry_function(entry_function)?;
    let test_case = entry_function.test_case();
    if !test_case.host_trap_plan().is_empty()
        || compiled.emitted().host_trap_requests().stdout_requested()
    {
        return Err(FunctionRunError::StandaloneArtifact(
            FunctionStandaloneArtifactError::HostTrapRequested,
        ));
    }

    Ok(compiled)
}

pub(crate) fn run_test_case_function(
    test_case: &TestCase,
) -> Result<FunctionRunResult, FunctionRunError> {
    let compiled = compile_test_case_function(test_case)?;
    let emitted = compiled.emitted();
    let result = match test_case.abi() {
        TestCaseAbi::NoArgsU64 => run_no_args_u64_with_host_traps(
            emitted.code().bytes(),
            runtime_host_trap_plan(test_case.host_trap_plan(), emitted.host_trap_requests())?,
        ),
        TestCaseAbi::OneU64ArgReturnsU64 { argument } => run_one_u64(
            emitted.code().bytes(),
            RunArgumentU64::new(argument.value()),
        ),
        TestCaseAbi::OneInputMemoryPtrReturnsU64 { memory } => {
            let memory = InputMemory::from_bytes(memory.bytes().to_vec())
                .map_err(FunctionRunError::InputMemory)?;
            run_one_input_memory_ptr(emitted.code().bytes(), memory)
        }
    }
    .map_err(FunctionRunError::Run)?;

    Ok(FunctionRunResult::from_runtime(&result))
}

fn runtime_host_trap_plan(
    plan: &bara_oracle::TestCaseHostTrapPlan,
    requests: &EmittedHostTrapRequests,
) -> Result<HostTrapPlan, FunctionRunError> {
    if !requests.stdout_requested() {
        return Ok(HostTrapPlan::none());
    }

    let Some(stdout) = plan.stdout_trap() else {
        return Ok(HostTrapPlan::none());
    };
    let stdout =
        RunStdout::from_text(stdout.text().to_owned()).map_err(FunctionRunError::StdoutTrap)?;
    Ok(HostTrapPlan::stdout(stdout))
}

#[cfg(test)]
mod tests {
    use bara_ir::{
        ExternalCallRequest, ExternalSymbolId, ExternalSymbolImport, PublicLibcSymbol,
        PublicSymbolImport, SyscallAbi, SyscallRequest, UnsupportedReason, X86Va,
    };
    use bara_oracle::{test_case_from_json, FailureKind};

    use super::{
        compile_test_case_function, compile_test_case_function_standalone_artifact,
        FunctionRunError, FunctionStandaloneArtifactError,
    };

    #[test]
    fn compile_only_returns_return_42_arm64_bytes() {
        let test_case = test_case_from_json(include_str!("../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");

        let compiled =
            compile_test_case_function(&test_case).expect("return_42 compile-only succeeds");

        assert_eq!(
            compiled.arm64_bytes().as_slice(),
            &[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn standalone_artifact_rejects_stdout_host_trap_fixture() {
        let test_case = test_case_from_json(include_str!(
            "../../../tests/cases/stdout_trap_return_0.json"
        ))
        .expect("stdout trap testcase parses");

        let error = compile_test_case_function_standalone_artifact(&test_case)
            .expect_err("stdout host trap fixture is not exportable as standalone artifact");

        assert!(matches!(
            error,
            FunctionRunError::StandaloneArtifact(
                FunctionStandaloneArtifactError::HostTrapRequested
            )
        ));
    }

    #[test]
    fn unsupported_syscall_emit_error_uses_stable_boundary_report() {
        let request =
            SyscallRequest::new(SyscallAbi::X86_64, X86Va::new(0x1000), X86Va::new(0x1002))
                .expect("test syscall range is valid");
        let error = FunctionRunError::Emit(bara_arm64::EmitError::UnsupportedIr {
            reason: UnsupportedReason::SyscallUnsupported { request },
        });

        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert_eq!(
            error.to_string(),
            "{\"status\":\"unsupported_boundary\",\"failure_kind\":\"emit_error\",\"boundary\":{\"kind\":\"syscall\",\"abi\":\"x86_64\",\"at\":4096,\"return_to\":4098}}"
        );
    }

    #[test]
    fn unsupported_external_call_emit_error_uses_stable_boundary_report() {
        let import = ExternalSymbolImport::public_symbol(
            ExternalSymbolId::new(9),
            PublicSymbolImport::Libc(PublicLibcSymbol::Puts),
        );
        let request =
            ExternalCallRequest::new_import(import, X86Va::new(0x2000), X86Va::new(0x2005))
                .expect("test external call range is valid");
        let error = FunctionRunError::Emit(bara_arm64::EmitError::UnsupportedIr {
            reason: UnsupportedReason::ExternalCallUnsupported { request },
        });

        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert_eq!(
            error.to_string(),
            "{\"status\":\"unsupported_boundary\",\"failure_kind\":\"emit_error\",\"boundary\":{\"kind\":\"external_call\",\"symbol_id\":9,\"import_target\":{\"kind\":\"public_symbol\",\"namespace\":\"libc\",\"symbol\":\"puts\"},\"call_site\":8192,\"return_to\":8197}}"
        );
    }

    #[test]
    fn unsupported_unresolved_external_call_emit_error_uses_stable_boundary_report() {
        let request = ExternalCallRequest::new(
            ExternalSymbolId::new(11),
            X86Va::new(0x3000),
            X86Va::new(0x3005),
        )
        .expect("test external call range is valid");
        let error = FunctionRunError::Emit(bara_arm64::EmitError::UnsupportedIr {
            reason: UnsupportedReason::ExternalCallUnsupported { request },
        });

        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert_eq!(
            error.to_string(),
            "{\"status\":\"unsupported_boundary\",\"failure_kind\":\"emit_error\",\"boundary\":{\"kind\":\"external_call\",\"symbol_id\":11,\"import_target\":{\"kind\":\"unresolved\"},\"call_site\":12288,\"return_to\":12293}}"
        );
    }
}
