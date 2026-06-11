use bara_ir::{
    validate_program, BoundaryRequest, HelperRequest, HostTrapKind, IrOp, MemoryReadWidth, Operand,
    Program, Terminator, UnsupportedReason, X86Cond, X86Reg, X86Va,
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
    branch_fixups: Vec<BranchFixup>,
    host_trap_requests: EmittedHostTrapRequests,
}

impl EmittedFunction {
    pub fn new(code: Arm64MachineCode, pc_map: Vec<PcMapEntry>) -> Self {
        Self::with_metadata(code, pc_map, Vec::new(), EmittedHostTrapRequests::none())
    }

    pub const fn with_metadata(
        code: Arm64MachineCode,
        pc_map: Vec<PcMapEntry>,
        branch_fixups: Vec<BranchFixup>,
        host_trap_requests: EmittedHostTrapRequests,
    ) -> Self {
        Self {
            code,
            pc_map,
            branch_fixups,
            host_trap_requests,
        }
    }

    pub const fn code(&self) -> &Arm64MachineCode {
        &self.code
    }

    pub fn pc_map(&self) -> &[PcMapEntry] {
        &self.pc_map
    }

    pub fn branch_fixups(&self) -> &[BranchFixup] {
        &self.branch_fixups
    }

    pub const fn host_trap_requests(&self) -> &EmittedHostTrapRequests {
        &self.host_trap_requests
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmittedHostTrapRequests {
    stdout: bool,
    appkit_gui_hello_world: bool,
}

impl EmittedHostTrapRequests {
    pub const fn none() -> Self {
        Self {
            stdout: false,
            appkit_gui_hello_world: false,
        }
    }

    pub const fn stdout() -> Self {
        Self {
            stdout: true,
            appkit_gui_hello_world: false,
        }
    }

    pub const fn appkit_gui_hello_world() -> Self {
        Self {
            stdout: false,
            appkit_gui_hello_world: true,
        }
    }

    const fn with_stdout_requested(self) -> Self {
        Self {
            stdout: true,
            appkit_gui_hello_world: self.appkit_gui_hello_world,
        }
    }

    const fn with_appkit_gui_hello_world_requested(self) -> Self {
        Self {
            stdout: self.stdout,
            appkit_gui_hello_world: true,
        }
    }

    pub const fn stdout_requested(self) -> bool {
        self.stdout
    }

    pub const fn appkit_gui_hello_world_requested(self) -> bool {
        self.appkit_gui_hello_world
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
pub struct BranchFixup {
    offset: ArmPc,
    source: ArmPc,
    target: X86Va,
    kind: BranchFixupKind,
}

impl BranchFixup {
    const fn new(offset: ArmPc, source: ArmPc, target: X86Va, kind: BranchFixupKind) -> Self {
        Self {
            offset,
            source,
            target,
            kind,
        }
    }

    #[cfg(test)]
    pub(crate) const fn for_test(
        offset: ArmPc,
        source: ArmPc,
        target: X86Va,
        kind: BranchFixupKind,
    ) -> Self {
        Self::new(offset, source, target, kind)
    }

    pub const fn offset(&self) -> ArmPc {
        self.offset
    }

    pub const fn source(&self) -> ArmPc {
        self.source
    }

    pub const fn target(&self) -> X86Va {
        self.target
    }

    pub const fn kind(&self) -> BranchFixupKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchFixupKind {
    Unconditional,
    Call,
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
    let rax_live_in_blocks = rax_live_in_blocks(program);

    for block in program.blocks() {
        let block_target = current_arm_pc(&code)?;
        pc_map.push(PcMapEntry::new(block.start(), block_target));
        block_offsets.push(BlockOffset::new(block.start(), block_target));

        let mut has_rax_value = rax_live_in_blocks.contains(&block.start());
        let mut rax_known_value = None;

        for op in block.ops() {
            match op {
                IrOp::HostTrap {
                    kind: HostTrapKind::Stdout,
                } => {
                    host_trap_requests = host_trap_requests.with_stdout_requested();
                }
                IrOp::HostTrap {
                    kind: HostTrapKind::AppKitGuiHelloWorld,
                } => {
                    host_trap_requests = host_trap_requests.with_appkit_gui_hello_world_requested();
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(value),
                } => {
                    emit_mov_x0_u64(&mut code, *value);
                    has_rax_value = true;
                    rax_known_value = Some(*value);
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::Reg(X86Reg::Rdi),
                } => {
                    has_rax_value = true;
                    rax_known_value = None;
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::Mem8 { base: X86Reg::Rdi },
                } => {
                    emit_ldrb_w0_x0(&mut code);
                    has_rax_value = true;
                    rax_known_value = None;
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src:
                        Operand::MemRipRelative {
                            address,
                            width: MemoryReadWidth::Bits64,
                        },
                } => {
                    let Some(value) = program
                        .image_metadata()
                        .mapped_bytes()
                        .read_u64_le(*address)
                    else {
                        return Err(EmitError::UnsupportedShape);
                    };
                    emit_mov_x0_u64(&mut code, value);
                    has_rax_value = true;
                    rax_known_value = Some(value);
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src:
                        Operand::MemRipRelative {
                            address,
                            width: MemoryReadWidth::Bits64,
                        },
                } => {
                    let Some(value) = program
                        .image_metadata()
                        .mapped_bytes()
                        .read_u64_le(*address)
                    else {
                        return Err(EmitError::UnsupportedShape);
                    };
                    emit_mov_x0_u64(&mut code, value);
                    has_rax_value = false;
                    rax_known_value = None;
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rsi),
                    src:
                        Operand::MemRipRelative {
                            address,
                            width: MemoryReadWidth::Bits64,
                        },
                } => {
                    let Some(value) = program
                        .image_metadata()
                        .mapped_bytes()
                        .read_u64_le(*address)
                    else {
                        return Err(EmitError::UnsupportedShape);
                    };
                    emit_mov_x1_u64(&mut code, value);
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::R14),
                    src:
                        Operand::MemRipRelative {
                            address,
                            width: MemoryReadWidth::Bits64,
                        },
                } => {
                    let Some(value) = program
                        .image_metadata()
                        .mapped_bytes()
                        .read_u64_le(*address)
                    else {
                        return Err(EmitError::UnsupportedShape);
                    };
                    emit_mov_x14_u64(&mut code, value);
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdx),
                    src:
                        Operand::MemRegIndirect {
                            base: X86Reg::Rax,
                            width: MemoryReadWidth::Bits64,
                        },
                } => {
                    let Some(address) = rax_known_value.map(X86Va::new) else {
                        return Err(EmitError::UnsupportedIr {
                            reason: UnsupportedReason::RegisterIndirectMemoryReadUnsupported {
                                base: X86Reg::Rax,
                                width: MemoryReadWidth::Bits64,
                            },
                        });
                    };
                    let Some(value) = program.image_metadata().mapped_bytes().read_u64_le(address)
                    else {
                        return Err(EmitError::UnsupportedIr {
                            reason: UnsupportedReason::MappedMemoryReadUnsupported {
                                address,
                                width: MemoryReadWidth::Bits64,
                            },
                        });
                    };
                    emit_mov_x2_u64(&mut code, value);
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src: Operand::AddressRipRelative { address },
                } => {
                    emit_mov_x0_u64(&mut code, address.value());
                    has_rax_value = false;
                    rax_known_value = None;
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rsi),
                    src: Operand::AddressRipRelative { address },
                } => {
                    emit_mov_x1_u64(&mut code, address.value());
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rbx),
                    src: Operand::Reg(X86Reg::Rax),
                } => {
                    if !has_rax_value {
                        return Err(EmitError::UnsupportedShape);
                    }
                    emit_mov_x19_x0(&mut code);
                }
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rbp),
                    src: Operand::Reg(X86Reg::Rsp),
                } => {
                    emit_mov_x29_sp(&mut code);
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
                    rax_known_value = None;
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
                    rax_known_value = None;
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
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rax),
                } => {
                    if !has_rax_value {
                        return Err(EmitError::UnsupportedShape);
                    }
                    emit_push_x0(&mut code);
                }
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rbx),
                } => {
                    emit_push_x19(&mut code);
                }
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rbp),
                } => {
                    emit_push_x29(&mut code);
                }
                IrOp::Push {
                    src: Operand::Reg(X86Reg::R14),
                } => {
                    emit_push_x14(&mut code);
                }
                IrOp::Push {
                    src: Operand::Reg(X86Reg::R15),
                } => {
                    emit_push_x15(&mut code);
                }
                IrOp::Push { .. } => {
                    return Err(unsupported_ir());
                }
                IrOp::Pop {
                    dst: Operand::Reg(X86Reg::Rax),
                } => {
                    emit_pop_x0(&mut code);
                    has_rax_value = true;
                    rax_known_value = None;
                }
                IrOp::Pop { .. } => {
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
            Terminator::DirectCall { target, return_to } => {
                emit_direct_call_placeholders(&mut code, &mut branch_fixups, *target, *return_to)?;
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
    Ok(EmittedFunction::with_metadata(
        machine_code,
        pc_map,
        branch_fixups,
        host_trap_requests,
    ))
}

fn rax_live_in_blocks(program: &Program) -> Vec<X86Va> {
    let blocks = program.blocks();
    let mut live_in = vec![false; blocks.len()];
    let mut changed = true;

    while changed {
        changed = false;

        for (index, block) in blocks.iter().enumerate() {
            let has_rax = block_output_has_rax(block, live_in[index]);
            match block.terminator() {
                Terminator::Fallthrough { target } | Terminator::DirectJump { target } => {
                    changed |= propagate_rax_to_block(blocks, &mut live_in, *target, has_rax);
                }
                Terminator::CondJump {
                    taken, fallthrough, ..
                } => {
                    changed |= propagate_rax_to_block(blocks, &mut live_in, *taken, has_rax);
                    changed |= propagate_rax_to_block(blocks, &mut live_in, *fallthrough, has_rax);
                }
                Terminator::DirectCall { target, return_to } => {
                    changed |= propagate_rax_to_block(blocks, &mut live_in, *target, has_rax);
                    changed |= propagate_rax_to_block(blocks, &mut live_in, *return_to, true);
                }
                Terminator::Return
                | Terminator::BoundaryRequest { .. }
                | Terminator::Unsupported { .. } => {}
            }
        }
    }

    program
        .blocks()
        .iter()
        .zip(live_in)
        .filter_map(
            |(block, is_live)| {
                if is_live {
                    Some(block.start())
                } else {
                    None
                }
            },
        )
        .collect()
}

fn block_output_has_rax(block: &bara_ir::BasicBlock, mut has_rax: bool) -> bool {
    for op in block.ops() {
        match op {
            IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                ..
            }
            | IrOp::Pop {
                dst: Operand::Reg(X86Reg::Rax),
            } => {
                has_rax = true;
            }
            IrOp::Mov { .. }
            | IrOp::Add { .. }
            | IrOp::Sub { .. }
            | IrOp::Cmp { .. }
            | IrOp::Test { .. }
            | IrOp::Push { .. }
            | IrOp::HostTrap { .. }
            | IrOp::Pop { .. }
            | IrOp::Unsupported { .. } => {}
        }
    }

    has_rax
}

fn propagate_rax_to_block(
    blocks: &[bara_ir::BasicBlock],
    live_in: &mut [bool],
    target: X86Va,
    has_rax: bool,
) -> bool {
    if !has_rax {
        return false;
    }

    let Some(index) = blocks.iter().position(|block| block.start() == target) else {
        return false;
    };

    if live_in[index] {
        return false;
    }

    live_in[index] = true;
    true
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

fn emit_direct_call_placeholders(
    code: &mut Vec<u8>,
    branch_fixups: &mut Vec<BranchFixup>,
    target: X86Va,
    return_to: X86Va,
) -> Result<(), EmitError> {
    emit_save_link_register(code);
    emit_branch_placeholder(code, branch_fixups, target, BranchFixupKind::Call)?;
    emit_restore_link_register(code);
    emit_branch_placeholder(
        code,
        branch_fixups,
        return_to,
        BranchFixupKind::Unconditional,
    )
}

fn emit_branch_placeholder(
    code: &mut Vec<u8>,
    branch_fixups: &mut Vec<BranchFixup>,
    target: X86Va,
    kind: BranchFixupKind,
) -> Result<(), EmitError> {
    let source = current_arm_pc(code)?;
    let offset = current_arm_pc(code)?;
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
        let offset = usize::try_from(fixup.offset.value()).map_err(|_| unsupported_ir())?;
        let end = offset.checked_add(4).ok_or_else(unsupported_ir)?;
        let Some(slot) = code.get_mut(offset..end) else {
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
        BranchFixupKind::Call => {
            let imm26 = branch_immediate(source, target, 26)?;
            Ok(0x9400_0000 | imm26)
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
        X86Cond::Overflow => Ok(6),
        X86Cond::NotOverflow => Ok(7),
        X86Cond::Below => Ok(3),
        X86Cond::AboveOrEqual => Ok(2),
        X86Cond::Equal => Ok(0),
        X86Cond::NotEqual => Ok(1),
        X86Cond::BelowOrEqual => Ok(9),
        X86Cond::Above => Ok(8),
        X86Cond::Sign => Ok(4),
        X86Cond::NotSign => Ok(5),
        X86Cond::Less => Ok(11),
        X86Cond::GreaterOrEqual => Ok(10),
        X86Cond::LessOrEqual => Ok(13),
        X86Cond::Greater => Ok(12),
        X86Cond::Parity | X86Cond::NotParity => Err(unsupported_ir()),
    }
}

fn emit_mov_x0_u64(code: &mut Vec<u8>, value: u64) -> usize {
    emit_mov_reg_u64(code, value, 0)
}

fn emit_mov_x1_u64(code: &mut Vec<u8>, value: u64) -> usize {
    emit_mov_reg_u64(code, value, 1)
}

fn emit_mov_x2_u64(code: &mut Vec<u8>, value: u64) -> usize {
    emit_mov_reg_u64(code, value, 2)
}

fn emit_mov_x14_u64(code: &mut Vec<u8>, value: u64) -> usize {
    emit_mov_reg_u64(code, value, 14)
}

fn emit_mov_reg_u64(code: &mut Vec<u8>, value: u64, reg: u32) -> usize {
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
            emit_u32_le(code, opcode | (hw << 21) | (imm16 << 5) | reg);
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

fn emit_push_x0(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xf81f_0fe0)
}

fn emit_push_x14(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xf81f_0fee)
}

fn emit_push_x15(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xf81f_0fef)
}

fn emit_push_x19(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xf81f_0ff3)
}

fn emit_push_x29(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xf81f_0ffd)
}

fn emit_mov_x29_sp(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0x9100_03fd)
}

fn emit_mov_x19_x0(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xaa00_03f3)
}

fn emit_pop_x0(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xf841_07e0)
}

fn emit_save_link_register(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xa9bf_7bfd)
}

fn emit_restore_link_register(code: &mut Vec<u8>) -> usize {
    emit_u32_le(code, 0xa8c1_7bfd)
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
        HostTrapKind, IrOp, MemoryReadWidth, Operand, Program, ProgramImageMappedByteSegment,
        ProgramImageMappedBytes, ProgramImageMetadata, ProgramImageRange, SyscallAbi,
        SyscallRequest, Terminator, UnsupportedReason, X86Cond, X86Reg, X86Va,
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

    fn program_with_ops_and_metadata(
        ops: Vec<IrOp>,
        terminator: Terminator,
        metadata: ProgramImageMetadata,
    ) -> Program {
        let block = BasicBlock::new(
            BlockId::new(0),
            X86Va::new(0),
            X86Va::new(1),
            ops,
            terminator,
        )
        .expect("test block range is valid");
        Program::with_image_metadata(X86Va::new(0), vec![block], metadata)
            .expect("program has entry block")
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
    fn emits_static_mapped_qword_for_rax_from_rip_relative_memory() {
        let range = ProgramImageRange::new(X86Va::new(0x3000), X86Va::new(0x3008))
            .expect("mapped range is non-empty");
        let segment = ProgramImageMappedByteSegment::new(
            range,
            vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11],
        )
        .expect("mapped bytes match range");
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            bara_ir::ProgramImageSections::empty(),
            ProgramImageMappedBytes::from_segments([segment]),
            bara_ir::ProgramImageSymbols::empty(),
            bara_ir::ProgramImageRelocations::empty(),
            bara_ir::ProgramImageImports::empty(),
            bara_ir::ProgramUnwindMetadata::empty(),
        );
        let program = program_with_ops_and_metadata(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::MemRipRelative {
                    address: X86Va::new(0x3000),
                    width: MemoryReadWidth::Bits64,
                },
            }],
            Terminator::Return,
            metadata,
        );

        let emitted = emit_program(&program).expect("RIP-relative mapped qword IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x00, 0xf1, 0x8e, 0xd2, 0xc0, 0xac, 0xaa, 0xf2, 0x80, 0x68, 0xc6, 0xf2, 0x40, 0x24,
                0xe2, 0xf2, 0xc0, 0x03, 0x5f, 0xd6
            ]
        );
    }

    #[test]
    fn emits_static_mapped_qword_for_rdi_from_rip_relative_memory() {
        let range = ProgramImageRange::new(X86Va::new(0x3000), X86Va::new(0x3008))
            .expect("mapped range is non-empty");
        let segment = ProgramImageMappedByteSegment::new(
            range,
            vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11],
        )
        .expect("mapped bytes match range");
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            bara_ir::ProgramImageSections::empty(),
            ProgramImageMappedBytes::from_segments([segment]),
            bara_ir::ProgramImageSymbols::empty(),
            bara_ir::ProgramImageRelocations::empty(),
            bara_ir::ProgramImageImports::empty(),
            bara_ir::ProgramUnwindMetadata::empty(),
        );
        let program = program_with_ops_and_metadata(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x3000),
                        width: MemoryReadWidth::Bits64,
                    },
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(1),
                },
            ],
            Terminator::Return,
            metadata,
        );

        let emitted = emit_program(&program).expect("RIP-relative mapped RDI qword IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x00, 0xf1, 0x8e, 0xd2, 0xc0, 0xac, 0xaa, 0xf2, 0x80, 0x68, 0xc6, 0xf2, 0x40, 0x24,
                0xe2, 0xf2, 0x20, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6
            ]
        );
    }

    #[test]
    fn rdi_rip_relative_memory_does_not_leave_rax_available() {
        let range = ProgramImageRange::new(X86Va::new(0x3000), X86Va::new(0x3008))
            .expect("mapped range is non-empty");
        let segment = ProgramImageMappedByteSegment::new(
            range,
            vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11],
        )
        .expect("mapped bytes match range");
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            bara_ir::ProgramImageSections::empty(),
            ProgramImageMappedBytes::from_segments([segment]),
            bara_ir::ProgramImageSymbols::empty(),
            bara_ir::ProgramImageRelocations::empty(),
            bara_ir::ProgramImageImports::empty(),
            bara_ir::ProgramUnwindMetadata::empty(),
        );
        let program = program_with_ops_and_metadata(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x3000),
                        width: MemoryReadWidth::Bits64,
                    },
                },
            ],
            Terminator::Return,
            metadata,
        );

        assert_eq!(emit_program(&program), Err(EmitError::UnsupportedShape));
    }

    #[test]
    fn emits_static_mapped_qword_for_rsi_from_rip_relative_memory_without_clobbering_rax() {
        let range = ProgramImageRange::new(X86Va::new(0x3000), X86Va::new(0x3008))
            .expect("mapped range is non-empty");
        let segment = ProgramImageMappedByteSegment::new(
            range,
            vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11],
        )
        .expect("mapped bytes match range");
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            bara_ir::ProgramImageSections::empty(),
            ProgramImageMappedBytes::from_segments([segment]),
            bara_ir::ProgramImageSymbols::empty(),
            bara_ir::ProgramImageRelocations::empty(),
            bara_ir::ProgramImageImports::empty(),
            bara_ir::ProgramUnwindMetadata::empty(),
        );
        let program = program_with_ops_and_metadata(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rsi),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x3000),
                        width: MemoryReadWidth::Bits64,
                    },
                },
            ],
            Terminator::Return,
            metadata,
        );

        let emitted = emit_program(&program).expect("RIP-relative mapped RSI qword IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x40, 0x05, 0x80, 0xd2, 0x01, 0xf1, 0x8e, 0xd2, 0xc1, 0xac, 0xaa, 0xf2, 0x81, 0x68,
                0xc6, 0xf2, 0x41, 0x24, 0xe2, 0xf2, 0xc0, 0x03, 0x5f, 0xd6
            ]
        );
    }

    #[test]
    fn emits_static_mapped_qword_for_r14_from_rip_relative_memory_without_clobbering_rax() {
        let range = ProgramImageRange::new(X86Va::new(0x3000), X86Va::new(0x3008))
            .expect("mapped range is non-empty");
        let segment = ProgramImageMappedByteSegment::new(
            range,
            vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11],
        )
        .expect("mapped bytes match range");
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            bara_ir::ProgramImageSections::empty(),
            ProgramImageMappedBytes::from_segments([segment]),
            bara_ir::ProgramImageSymbols::empty(),
            bara_ir::ProgramImageRelocations::empty(),
            bara_ir::ProgramImageImports::empty(),
            bara_ir::ProgramUnwindMetadata::empty(),
        );
        let program = program_with_ops_and_metadata(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::R14),
                    src: Operand::MemRipRelative {
                        address: X86Va::new(0x3000),
                        width: MemoryReadWidth::Bits64,
                    },
                },
            ],
            Terminator::Return,
            metadata,
        );

        let emitted = emit_program(&program).expect("RIP-relative mapped R14 qword IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x40, 0x05, 0x80, 0xd2, 0x0e, 0xf1, 0x8e, 0xd2, 0xce, 0xac, 0xaa, 0xf2, 0x8e, 0x68,
                0xc6, 0xf2, 0x4e, 0x24, 0xe2, 0xf2, 0xc0, 0x03, 0x5f, 0xd6
            ]
        );
    }

    #[test]
    fn emits_static_mapped_qword_for_rdx_from_known_rax_indirect_memory() {
        let range = ProgramImageRange::new(X86Va::new(0x4000), X86Va::new(0x4008))
            .expect("mapped range is non-empty");
        let segment = ProgramImageMappedByteSegment::new(
            range,
            vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11],
        )
        .expect("mapped bytes match range");
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            bara_ir::ProgramImageSections::empty(),
            ProgramImageMappedBytes::from_segments([segment]),
            bara_ir::ProgramImageSymbols::empty(),
            bara_ir::ProgramImageRelocations::empty(),
            bara_ir::ProgramImageImports::empty(),
            bara_ir::ProgramUnwindMetadata::empty(),
        );
        let program = program_with_ops_and_metadata(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0x4000),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdx),
                    src: Operand::MemRegIndirect {
                        base: X86Reg::Rax,
                        width: MemoryReadWidth::Bits64,
                    },
                },
            ],
            Terminator::Return,
            metadata,
        );

        let emitted = emit_program(&program).expect("RAX-indirect mapped qword IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x00, 0x00, 0x88, 0xd2, 0x02, 0xf1, 0x8e, 0xd2, 0xc2, 0xac, 0xaa, 0xf2, 0x82, 0x68,
                0xc6, 0xf2, 0x42, 0x24, 0xe2, 0xf2, 0xc0, 0x03, 0x5f, 0xd6
            ]
        );
    }

    #[test]
    fn emits_rdi_rip_relative_address_as_x0_immediate() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src: Operand::AddressRipRelative {
                        address: X86Va::new(0x26d0),
                    },
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(1),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("RIP-relative LEA IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x00, 0xda, 0x84, 0xd2, 0x20, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn rdi_address_materialization_does_not_leave_rax_available() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdi),
                    src: Operand::AddressRipRelative {
                        address: X86Va::new(0x26d0),
                    },
                },
            ],
            Terminator::Return,
        );

        assert_eq!(emit_program(&program), Err(EmitError::UnsupportedShape));
    }

    #[test]
    fn emits_rsi_rip_relative_address_as_x1_immediate_without_clobbering_rax() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rsi),
                    src: Operand::AddressRipRelative {
                        address: X86Va::new(0x26da),
                    },
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("RIP-relative RSI LEA IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0x41, 0xdb, 0x84, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn rejects_rip_relative_memory_when_mapped_bytes_are_absent() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::MemRipRelative {
                    address: X86Va::new(0x3000),
                    width: MemoryReadWidth::Bits64,
                },
            }],
            Terminator::Return,
        );

        assert_eq!(emit_program(&program), Err(EmitError::UnsupportedShape));
    }

    #[test]
    fn rejects_rax_indirect_memory_when_rax_value_is_not_static() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::Reg(X86Reg::Rdi),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdx),
                    src: Operand::MemRegIndirect {
                        base: X86Reg::Rax,
                        width: MemoryReadWidth::Bits64,
                    },
                },
            ],
            Terminator::Return,
        );

        assert_eq!(
            emit_program(&program),
            Err(EmitError::UnsupportedIr {
                reason: UnsupportedReason::RegisterIndirectMemoryReadUnsupported {
                    base: X86Reg::Rax,
                    width: MemoryReadWidth::Bits64,
                }
            })
        );
    }

    #[test]
    fn rejects_rax_indirect_memory_when_mapped_bytes_are_absent() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0x4000),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rdx),
                    src: Operand::MemRegIndirect {
                        base: X86Reg::Rax,
                        width: MemoryReadWidth::Bits64,
                    },
                },
            ],
            Terminator::Return,
        );

        assert_eq!(
            emit_program(&program),
            Err(EmitError::UnsupportedIr {
                reason: UnsupportedReason::MappedMemoryReadUnsupported {
                    address: X86Va::new(0x4000),
                    width: MemoryReadWidth::Bits64,
                }
            })
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
    fn records_appkit_gui_host_trap_request_without_emitting_code_for_it() {
        let program = program_with_ops(
            vec![
                IrOp::HostTrap {
                    kind: HostTrapKind::AppKitGuiHelloWorld,
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("appkit GUI host trap IR emits");

        assert!(emitted
            .host_trap_requests()
            .appkit_gui_hello_world_requested());
        assert!(!emitted.host_trap_requests().stdout_requested());
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
    fn emits_push_pop_rax_with_aligned_stack_slot() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rax),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(7),
                },
                IrOp::Pop {
                    dst: Operand::Reg(X86Reg::Rax),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("push/pop IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x40, 0x05, 0x80, 0xd2, 0xe0, 0x0f, 0x1f, 0xf8, 0xe0, 0x00, 0x80, 0xd2, 0xe0, 0x07,
                0x41, 0xf8, 0xc0, 0x03, 0x5f, 0xd6,
            ]
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
    fn emits_push_rbp_with_aligned_stack_slot() {
        let program = program_with_ops(
            vec![
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rbp),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("push rbp IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0xfd, 0x0f, 0x1f, 0xf8, 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_push_r15_with_aligned_stack_slot() {
        let program = program_with_ops(
            vec![
                IrOp::Push {
                    src: Operand::Reg(X86Reg::R15),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("push r15 IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0xef, 0x0f, 0x1f, 0xf8, 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_push_r14_with_aligned_stack_slot() {
        let program = program_with_ops(
            vec![
                IrOp::Push {
                    src: Operand::Reg(X86Reg::R14),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("push r14 IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0xee, 0x0f, 0x1f, 0xf8, 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_push_rbx_with_aligned_stack_slot() {
        let program = program_with_ops(
            vec![
                IrOp::Push {
                    src: Operand::Reg(X86Reg::Rbx),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("push rbx IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0xf3, 0x0f, 0x1f, 0xf8, 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_mov_rbx_rax_as_saved_register_assignment() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rbx),
                    src: Operand::Reg(X86Reg::Rax),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("mov rbx,rax IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0x40, 0x05, 0x80, 0xd2, 0xf3, 0x03, 0x00, 0xaa, 0xc0, 0x03, 0x5f, 0xd6]
        );
    }

    #[test]
    fn emits_mov_rbp_rsp_as_frame_pointer_assignment() {
        let program = program_with_ops(
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rbp),
                    src: Operand::Reg(X86Reg::Rsp),
                },
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(42),
                },
            ],
            Terminator::Return,
        );

        let emitted = emit_program(&program).expect("mov rbp,rsp IR emits");

        assert_eq!(
            emitted.code().bytes(),
            &[0xfd, 0x03, 0x00, 0x91, 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
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
    fn missing_branch_target_is_invalid_program() {
        let program = program_with_ops(
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42),
            }],
            Terminator::DirectJump {
                target: X86Va::new(8),
            },
        );

        assert_eq!(emit_program(&program), Err(EmitError::InvalidProgram));
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
    fn emits_conditional_branch_fixups_for_less() {
        let block0 = BasicBlock::new(
            BlockId::new(0),
            X86Va::new(0),
            X86Va::new(4),
            vec![
                IrOp::Mov {
                    dst: Operand::Reg(X86Reg::Rax),
                    src: Operand::ImmU64(0),
                },
                IrOp::Cmp {
                    lhs: Operand::Reg(X86Reg::Rax),
                    rhs: Operand::ImmU64(1),
                },
            ],
            Terminator::CondJump {
                condition: X86Cond::Less,
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

        let emitted = emit_program(&program).expect("less conditional branch emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0x00, 0x00, 0x80, 0xd2, 0x1f, 0x04, 0x00, 0xf1, 0x8b, 0x00, 0x00, 0x54, 0x01, 0x00,
                0x00, 0x14, 0xe0, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6, 0x40, 0x05, 0x80, 0xd2,
                0xc0, 0x03, 0x5f, 0xd6,
            ]
        );
    }

    #[test]
    fn parity_conditional_branch_is_not_emitted() {
        let block0 = BasicBlock::new(
            BlockId::new(0),
            X86Va::new(0),
            X86Va::new(4),
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(0),
            }],
            Terminator::CondJump {
                condition: X86Cond::Parity,
                taken: X86Va::new(4),
                fallthrough: X86Va::new(8),
            },
        )
        .expect("test block range is valid");
        let block1 = BasicBlock::new(
            BlockId::new(1),
            X86Va::new(4),
            X86Va::new(8),
            vec![IrOp::Mov {
                dst: Operand::Reg(X86Reg::Rax),
                src: Operand::ImmU64(42),
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
                src: Operand::ImmU64(7),
            }],
            Terminator::Return,
        )
        .expect("test block range is valid");
        let program =
            Program::new(X86Va::new(0), vec![block0, block1, block2]).expect("program is valid");

        assert_eq!(
            emit_program(&program),
            Err(EmitError::UnsupportedIr {
                reason: UnsupportedReason::EmitUnsupportedIr
            })
        );
    }

    #[test]
    fn emits_direct_call_fixups_and_return_to_block() {
        let block0 = BasicBlock::new(
            BlockId::new(0),
            X86Va::new(0),
            X86Va::new(5),
            Vec::new(),
            Terminator::DirectCall {
                target: X86Va::new(6),
                return_to: X86Va::new(5),
            },
        )
        .expect("test block range is valid");
        let block1 = BasicBlock::new(
            BlockId::new(1),
            X86Va::new(5),
            X86Va::new(6),
            Vec::new(),
            Terminator::Return,
        )
        .expect("test block range is valid");
        let block2 = BasicBlock::new(
            BlockId::new(2),
            X86Va::new(6),
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

        let emitted = emit_program(&program).expect("direct call emits");

        assert_eq!(
            emitted.code().bytes(),
            &[
                0xfd, 0x7b, 0xbf, 0xa9, 0x04, 0x00, 0x00, 0x94, 0xfd, 0x7b, 0xc1, 0xa8, 0x01, 0x00,
                0x00, 0x14, 0xc0, 0x03, 0x5f, 0xd6, 0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6,
            ]
        );
        assert_eq!(emitted.pc_map()[0].target(), ArmPc::new(0));
        assert_eq!(emitted.pc_map()[1].target(), ArmPc::new(16));
        assert_eq!(emitted.pc_map()[2].target(), ArmPc::new(20));
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
