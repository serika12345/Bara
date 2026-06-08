use bara_ir::{
    validate_program, BoundaryRequest, HelperRequest, HostTrapKind, IrOp, Operand, Program,
    Terminator, UnsupportedReason, X86Cond, X86Reg, X86Va,
};

use crate::{ArmPc, PcMapEntry};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arm64MachineCode {
    bytes: Vec<u8>,
}

impl Arm64MachineCode {
    pub fn new(bytes: Vec<u8>) -> Result<Self, EmitError> {
        if bytes.is_empty() {
            return Err(EmitError::EmptyCode);
        }

        Ok(Self { bytes })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmittedFunction {
    code: Arm64MachineCode,
    pc_map: Vec<PcMapEntry>,
    host_trap_requests: EmittedHostTrapRequests,
}

impl EmittedFunction {
    pub fn new(code: Arm64MachineCode, pc_map: Vec<PcMapEntry>) -> Self {
        Self::with_host_trap_requests(code, pc_map, EmittedHostTrapRequests::none())
    }

    pub const fn with_host_trap_requests(
        code: Arm64MachineCode,
        pc_map: Vec<PcMapEntry>,
        host_trap_requests: EmittedHostTrapRequests,
    ) -> Self {
        Self {
            code,
            pc_map,
            host_trap_requests,
        }
    }

    pub const fn code(&self) -> &Arm64MachineCode {
        &self.code
    }

    pub fn pc_map(&self) -> &[PcMapEntry] {
        &self.pc_map
    }

    pub const fn host_trap_requests(&self) -> &EmittedHostTrapRequests {
        &self.host_trap_requests
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmittedHostTrapRequests {
    stdout: bool,
}

impl EmittedHostTrapRequests {
    pub const fn none() -> Self {
        Self { stdout: false }
    }

    pub const fn stdout() -> Self {
        Self { stdout: true }
    }

    pub const fn stdout_requested(self) -> bool {
        self.stdout
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmitError {
    InvalidProgram,
    EmptyCode,
    UnsupportedIr { reason: UnsupportedReason },
    UnsupportedShape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BlockOffset {
    source: X86Va,
    target: ArmPc,
}

impl BlockOffset {
    const fn new(source: X86Va, target: ArmPc) -> Self {
        Self { source, target }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BranchFixup {
    offset: usize,
    source: ArmPc,
    target: X86Va,
    kind: BranchFixupKind,
}

impl BranchFixup {
    const fn new(offset: usize, source: ArmPc, target: X86Va, kind: BranchFixupKind) -> Self {
        Self {
            offset,
            source,
            target,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BranchFixupKind {
    Unconditional,
    Conditional { condition: X86Cond },
}

pub fn emit_program(program: &Program) -> Result<EmittedFunction, EmitError> {
    if !validate_program(program).is_valid() {
        return Err(EmitError::InvalidProgram);
    }

    let mut code = Vec::new();
    let mut host_trap_requests = EmittedHostTrapRequests::none();
    let mut pc_map = Vec::new();
    let mut block_offsets = Vec::new();
    let mut branch_fixups = Vec::new();

    for block in program.blocks() {
        let block_target = current_arm_pc(&code)?;
        pc_map.push(PcMapEntry::new(block.start(), block_target));
        block_offsets.push(BlockOffset::new(block.start(), block_target));

        let mut has_rax_value = false;

        for op in block.ops() {
            match op {
                IrOp::HostTrap {
                    kind: HostTrapKind::Stdout,
                } => {
                    host_trap_requests = EmittedHostTrapRequests::stdout();
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(value),
                } => {
                    emit_mov_x0_u64(&mut code, *value);
                    has_rax_value = true;
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::Reg(X86Reg::Rdi),
                } => {
                    has_rax_value = true;
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::Mem8 { base: X86Reg::Rdi },
                } => {
                    emit_ldrb_w0_x0(&mut code);
                    has_rax_value = true;
                }
                IrOp::Mov { .. } => {
                    return Err(unsupported_ir());
                }
                IrOp::Add {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(value),
                } => {
                    if !has_rax_value {
                        return Err(EmitError::UnsupportedShape);
                    }
                    emit_add_x0_imm12(&mut code, *value)?;
                }
                IrOp::Add { .. } => {
                    return Err(unsupported_ir());
                }
                IrOp::Sub {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(value),
                } => {
                    if !has_rax_value {
                        return Err(EmitError::UnsupportedShape);
                    }
                    emit_sub_x0_imm12(&mut code, *value)?;
                }
                IrOp::Sub { .. } => {
                    return Err(unsupported_ir());
                }
                IrOp::Cmp {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::ImmU64(value),
                } => {
                    if !has_rax_value {
                        return Err(EmitError::UnsupportedShape);
                    }
                    emit_cmp_x0_imm12(&mut code, *value)?;
                }
                IrOp::Cmp { .. } => {
                    return Err(unsupported_ir());
                }
                IrOp::Test {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::Reg(X86Reg::Rax),
                } => {
                    if !has_rax_value {
                        return Err(EmitError::UnsupportedShape);
                    }
                    emit_tst_x0_x0(&mut code);
                }
                IrOp::Test { .. } => {
                    return Err(unsupported_ir());
                }
                IrOp::Unsupported { reason } => {
                    return Err(EmitError::UnsupportedIr {
                        reason: reason.clone(),
                    });
                }
            }
        }

        match block.terminator() {
            Terminator::Return => {
                if !has_rax_value {
                    return Err(EmitError::UnsupportedShape);
                }
                emit_u32_le(&mut code, 0xd65f_03c0);
            }
            Terminator::BoundaryRequest {
                request: BoundaryRequest::Helper(HelperRequest::CallExternal(request)),
            } => {
                return Err(EmitError::UnsupportedIr {
                    reason: UnsupportedReason::ExternalCallUnsupported { request: *request },
                });
            }
            Terminator::BoundaryRequest {
                request: BoundaryRequest::Syscall(request),
            } => {
                return Err(EmitError::UnsupportedIr {
                    reason: UnsupportedReason::SyscallUnsupported { request: *request },
                });
            }
            Terminator::Fallthrough { target } | Terminator::DirectJump { target } => {
                emit_branch_placeholder(
                    &mut code,
                    &mut branch_fixups,
                    *target,
                    BranchFixupKind::Unconditional,
                )?;
            }
            Terminator::CondJump {
                condition,
                taken,
                fallthrough,
            } => {
                emit_branch_placeholder(
                    &mut code,
                    &mut branch_fixups,
                    *taken,
                    BranchFixupKind::Conditional {
                        condition: *condition,
                    },
                )?;
                emit_branch_placeholder(
                    &mut code,
                    &mut branch_fixups,
                    *fallthrough,
                    BranchFixupKind::Unconditional,
                )?;
            }
            Terminator::Unsupported { reason } => {
                return Err(EmitError::UnsupportedIr {
                    reason: reason.clone(),
                });
            }
        }
    }

    apply_branch_fixups(&mut code, &branch_fixups, &block_offsets)?;

    let machine_code = Arm64MachineCode::new(code)?;
    Ok(EmittedFunction::with_host_trap_requests(
        machine_code,
        pc_map,
        host_trap_requests,
    ))
}

fn unsupported_ir() -> EmitError {
    EmitError::UnsupportedIr {
        reason: UnsupportedReason::EmitUnsupportedIr,
    }
}

fn current_arm_pc(code: &[u8]) -> Result<ArmPc, EmitError> {
    u64::try_from(code.len())
        .map(ArmPc::new)
        .map_err(|_| unsupported_ir())
}

fn emit_branch_placeholder(
    code: &mut Vec<u8>,
    branch_fixups: &mut Vec<BranchFixup>,
    target: X86Va,
    kind: BranchFixupKind,
) -> Result<(), EmitError> {
    let source = current_arm_pc(code)?;
    let offset = code.len();
    emit_u32_le(code, 0);
    branch_fixups.push(BranchFixup::new(offset, source, target, kind));
    Ok(())
}

fn apply_branch_fixups(
    code: &mut [u8],
    branch_fixups: &[BranchFixup],
    block_offsets: &[BlockOffset],
) -> Result<(), EmitError> {
    for fixup in branch_fixups {
        let Some(target) = find_block_target(fixup.target, block_offsets) else {
            return Err(unsupported_ir());
        };
        let instruction = encode_branch(fixup.source, target, fixup.kind)?;
        let end = fixup.offset.checked_add(4).ok_or_else(unsupported_ir)?;
        let Some(slot) = code.get_mut(fixup.offset..end) else {
            return Err(unsupported_ir());
        };
        slot.copy_from_slice(&instruction.to_le_bytes());
    }

    Ok(())
}

fn find_block_target(source: X86Va, block_offsets: &[BlockOffset]) -> Option<ArmPc> {
    block_offsets
        .iter()
        .find(|offset| offset.source == source)
        .map(|offset| offset.target)
}

fn encode_branch(source: ArmPc, target: ArmPc, kind: BranchFixupKind) -> Result<u32, EmitError> {
    match kind {
        BranchFixupKind::Unconditional => {
            let imm26 = branch_immediate(source, target, 26)?;
            Ok(0x1400_0000 | imm26)
        }
        BranchFixupKind::Conditional { condition } => {
            let imm19 = branch_immediate(source, target, 19)?;
            Ok(0x5400_0000 | (imm19 << 5) | arm64_condition(condition)?)
        }
    }
}

fn branch_immediate(source: ArmPc, target: ArmPc, bit_width: u32) -> Result<u32, EmitError> {
    let delta_bytes = i128::from(target.value()) - i128::from(source.value());
    if delta_bytes % 4 != 0 {
        return Err(unsupported_ir());
    }

    let delta_words = delta_bytes / 4;
    let min = -(1i128 << (bit_width - 1));
    let max = (1i128 << (bit_width - 1)) - 1;
    if delta_words < min || delta_words > max {
        return Err(unsupported_ir());
    }

    if delta_words < 0 {
        Ok(((1i128 << bit_width) + delta_words) as u32)
    } else {
        Ok(delta_words as u32)
    }
}

fn arm64_condition(condition: X86Cond) -> Result<u32, EmitError> {
    match condition {
        X86Cond::Equal => Ok(0),
        X86Cond::NotEqual => Ok(1),
        _ => Err(unsupported_ir()),
    }
}

fn emit_mov_x0_u64(code: &mut Vec<u8>, value: u64) -> usize {
    let mut emitted = 0usize;
    let mut wrote_first = false;

    for hw in 0..4u32 {
        let imm16 = ((value >> (hw * 16)) & 0xffff) as u32;
        if imm16 != 0 || !wrote_first {
            let opcode = if wrote_first {
                0xf280_0000
            } else {
                0xd280_0000
            };
            emit_u32_le(code, opcode | (hw << 21) | (imm16 << 5));
            wrote_first = true;
            emitted += 1;
        }
    }

    emitted
}

fn emit_add_x0_imm12(code: &mut Vec<u8>, value: u64) -> Result<usize, EmitError> {
    let Ok(imm12) = u32::try_from(value) else {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    };

    if imm12 > 0xfff {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    }

    Ok(emit_u32_le(code, 0x9100_0000 | (imm12 << 10)))
}

fn emit_sub_x0_imm12(code: &mut Vec<u8>, value: u64) -> Result<usize, EmitError> {
    let Ok(imm12) = u32::try_from(value) else {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    };

    if imm12 > 0xfff {
        return Err(EmitError::UnsupportedIr {
            reason: UnsupportedReason::EmitUnsupportedIr,
        });
    }

    Ok(emit_u32_le(code, 0xd100_0000 | (imm12 << 10)))
}

fn emit_cmp_x0_imm12(code: &mut Vec<u8>, value: u64) -> Result<usize, EmitError> {
    let Ok(imm12) = u32::try_from(value) else {
        return Err(unsupported_ir());
    };

    if imm12 > 0xfff {
        return Err(unsupported_ir());
    }

    Ok(emit_u32_le(code, 0xf100_001f | (imm12 << 10)))
}

fn emit_tst_x0_x0(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xea00_001f)
}

fn emit_ldrb_w0_x0(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0x3940_0000)
}

fn emit_u32_le(code: &mut Vec<u8>, instruction: u32) -> usize {
    code.extend_from_slice(&instruction.to_le_bytes());
    code.len()
}

#[cfg(test)]
mod tests {
    use bara_ir::{
        BasicBlock, BlockId, BoundaryRequest, ExternalCallRequest, ExternalSymbolId, HelperRequest,
        HostTrapKind, IrOp, Operand, Program, SyscallAbi, SyscallRequest, Terminator,
        UnsupportedReason, X86Cond, X86Reg, X86Va,
    };

    use crate::{emit_program, Arm64MachineCode, ArmPc, EmitError};

    fn program_with_ops(ops: Vec<IrOp>, terminator: Terminator) -> Program {
        let block = BasicBlock::new(
            BlockId::new(0),
            X86Va::new(0),
            X86Va::new(1),
            ops,
            terminator,
        )
        .expect("test block range is valid");
        Program::new(X86Va::new(0), vec![block]).expect("program has entry block")
    }

    #[test]
    fn machine_code_rejects_empty_bytes() {
        assert_eq!(Arm64MachineCode::new(Vec::new()), Err(EmitError::EmptyCode));
    }

    #[test]
    fn emits_mov_x0_imm_and_ret_for_rax_immediate_return() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42),
            }],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("M1 IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
        assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
        assert_eq!(emitted.pc_map()[0].target(), ArmPc::new(0));
    }

    #[test]
    fn emits_ret_only_for_rax_from_rdi_identity() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::Reg(X86Reg::Rdi),
            }],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("identity argument IR emits");

        assert_eq!(emitted.code().bytes(), &[0xc0, 0x03, 0x5f, 0xd6]);
    }

    #[test]
    fn emits_ldrb_w0_x0_for_rax_from_rdi_mem8() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::Mem8 { base: X86Reg::Rdi },
            }],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("memory load IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x00, 0x00, 0x40, 0x39, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn records_stdout_host_trap_request_without_emitting_code_for_it() {
        let program = program_with_ops(
            vec![
                IrOp::HostTrap {
                    kind: HostTrapKind::Stdout,
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("host trap IR emits");

        assert!(emitted.host_trap_requests().stdout_requested());
        assert_eq!(
            emitted.code().bytes(),
            &[0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_add_x0_immediate_for_rax_add_immediate() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Add {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("add immediate IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_sub_x0_immediate_for_rax_sub_immediate() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Sub {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(3),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("sub immediate IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0x00, 0x0c, 0x00, 0xd1, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_mov_x0_with_multiple_u16_chunks() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(0x0001_0000_0000_0002),
            }],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("multi-chunk immediate emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x00, 0x80, 0xd2, 0x20, 0x00, 0xe0, 0xf2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn return_without_rax_immediate_is_unsupported_shape() {
        let program = program_with_ops(Vec::new(), Terminator::Return);

        assert_eq!(emit_program(&program), Err(EmitError::UnsupportedShape));
    }

    #[test]
    fn unsupported_terminator_is_invalid_program() {
        let program = program_with_ops(
            Vec::new(),
            Terminator::Unsupported {
                reason: UnsupportedReason::MissingReturnTerminator { at: X86Va::new(1) },
            },
        );

        assert_eq!(emit_program(&program), Err(EmitError::InvalidProgram));
    }

    #[test]
    fn syscall_request_terminator_is_not_emitted() {
        let request = SyscallRequest::new(SyscallAbi::X86_64, X86Va::new(0), X86Va::new(2))
            .expect("test syscall range is valid");
        let program = program_with_ops(
            Vec::new(),
            Terminator::BoundaryRequest {
                request: BoundaryRequest::Syscall(request),
            },
        );

        assert_eq!(
            emit_program(&program),
            Err(EmitError::UnsupportedIr {
                reason: UnsupportedReason::SyscallUnsupported { request }
            })
        );
    }

    #[test]
    fn external_call_helper_request_terminator_is_not_emitted() {
        let request =
            ExternalCallRequest::new(ExternalSymbolId::new(9), X86Va::new(0), X86Va::new(5))
                .expect("test external call range is valid");
        let program = program_with_ops(
            Vec::new(),
            Terminator::BoundaryRequest {
                request: BoundaryRequest::Helper(HelperRequest::CallExternal(request)),
            },
        );

        assert_eq!(
            emit_program(&program),
            Err(EmitError::UnsupportedIr {
                reason: UnsupportedReason::ExternalCallUnsupported { request }
            })
        );
    }

    #[test]
    fn emits_conditional_branch_fixups_for_equal() {
        let block0 = BasicBlock::new(
            BlockId::new(0),
            X86Va::new(0),
            X86Va::new(4),
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0),
                },
                IrOp::Test {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::Reg(X86Reg::Rax),
                },
            ],
            Terminator::CondJump {
                condition: X86Cond::Equal,
                taken: X86Va::new(8),
                fallthrough: X86Va::new(4),
            },
        )
        .expect("test block range is valid");
        let block1 = BasicBlock::new(
            BlockId::new(1),
            X86Va::new(4),
            X86Va::new(8),
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(7),
            }],
            Terminator::Return,
        )
        .expect("test block range is valid");
        let block2 = BasicBlock::new(
            BlockId::new(2),
            X86Va::new(8),
            X86Va::new(12),
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42),
            }],
            Terminator::Return,
        )
        .expect("test block range is valid");
        let program =
            Program::new(X86Va::new(0), vec![block0, block1, block2]).expect("program is valid");

        let emitted = emit_program(&program).expect("control flow emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x00, 0x00, 0x80, 0xd2, 0x1f, 0x00, 0x00, 0xea, 0x80, 0x00, 0x00, 0x54, 0x01, 0x00,
                0x00, 0x14, 0xe0, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6, 0x40, 0x05, 0x80, 0xd2,
                0xc0, 0x03, 0x5f, 0xd6,
            ]
        );
        assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
        assert_eq!(emitted.pc_map()[1].source(), X86Va::new(4));
        assert_eq!(emitted.pc_map()[1].target(), ArmPc::new(16));
        assert_eq!(emitted.pc_map()[2].source(), X86Va::new(8));
        assert_eq!(emitted.pc_map()[2].target(), ArmPc::new(24));
    }

    #[test]
    fn emits_cmp_x0_immediate_for_rax_compare_immediate() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Cmp {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::ImmU64(42),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("cmp flags emit");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0x1f, 0xa8, 0x00, 0xf1, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_tst_x0_x0_for_rax_test_rax() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Test {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::Reg(X86Reg::Rax),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("test flags emit");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0x1f, 0x00, 0x00, 0xea, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }
}
