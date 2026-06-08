# 初期 IR 設計

## 方針

IR は小さく始める。最初の目的は最適化ではなく、命令の意味と制御フローを検証しやすく表現すること。

暗黙の fallthrough や曖昧な `u64` アドレスを避け、型で仕様を表す。

## 型の境界

source address と target address を混ぜない。

```rust
pub struct X86Va(pub u64);
pub struct X86Offset(pub u32);
pub struct ArmPc(pub u64);
pub struct BlockId(pub u32);
pub struct SymbolId(pub u32);
pub struct HelperId(pub u32);
```

## Program

```rust
pub struct Program {
    pub entry: X86Va,
    pub blocks: Vec<BasicBlock>,
}
```

## BasicBlock

```rust
pub struct BasicBlock {
    pub id: BlockId,
    pub start: X86Va,
    pub end: X86Va,
    pub ops: Vec<IrOp>,
    pub terminator: Terminator,
}
```

invariant:

- `start < end`
- `ops` は block 内の命令だけを表す
- `terminator` は必須
- fallthrough は明示する
- block の x86 address range は重ならない

## IrOp

初期対応:

```rust
pub enum IrOp {
    Mov { dst: Operand, src: Operand },
    Add { dst: Operand, src: Operand },
    Sub { dst: Operand, src: Operand },
    HostTrap { kind: HostTrapKind },
    Cmp { lhs: Operand, rhs: Operand },
    Test { lhs: Operand, rhs: Operand },
    Push { src: Operand },
    Pop { dst: Operand },
    Unsupported { reason: UnsupportedReason },
}
```

`HostTrap` は OS syscall ではなく、Bara 専用 sentinel/helper sequence を
runtime 境界へ伝えるための明示的な外部効果要求として扱う。

```rust
pub enum HostTrapKind {
    Stdout,
}
```

## Terminator

```rust
pub enum Terminator {
    Return,
    BoundaryRequest {
        request: BoundaryRequest,
    },
    DirectJump { target: X86Va },
    CondJump {
        cc: X86Cond,
        taken: X86Va,
        fallthrough: X86Va,
    },
    DirectCall {
        target: X86Va,
        return_to: X86Va,
    },
    IndirectJump {
        target: Operand,
    },
    IndirectCall {
        target: Operand,
        return_to: X86Va,
    },
    ExternalHelper {
        helper: HelperId,
    },
    Unsupported {
        reason: UnsupportedReason,
    },
}
```

`BoundaryRequest` は guest 側から public ABI / external boundary へ出ようとする
意図を IR に残す。runtime や host OS syscall を直接実行する指示ではない。
現在は x86_64 `syscall` と external symbol/import call を typed request として
保持するだけで、ARM64 emit では unsupported boundary として止める。

```rust
pub enum BoundaryRequest {
    Helper(HelperRequest),
    Syscall(SyscallRequest),
}

pub enum HelperRequest {
    CallExternal(ExternalCallRequest),
}

pub struct ExternalSymbolId(u32);

pub struct ExternalCallRequest {
    symbol: ExternalSymbolId,
    call_site: X86Va,
    return_to: X86Va,
}

pub struct SyscallRequest {
    abi: SyscallAbi,
    at: X86Va,
    return_to: X86Va,
}

pub enum SyscallAbi {
    X86_64,
}
```

## Operand

```rust
pub enum Operand {
    Reg(X86Reg),
    ImmU64(u64),
    Mem8 { base: X86Reg },
    Mem(MemRef),
}
```

現在の初期 corpus では `Reg`、`ImmU64`、`Mem8 { base: Rdi }` を使う。

## Flags

初期は lazy flags を使わない。

```rust
pub struct Flags {
    pub cf: FlagValue,
    pub pf: FlagValue,
    pub af: FlagValue,
    pub zf: FlagValue,
    pub sf: FlagValue,
    pub of: FlagValue,
}

pub enum FlagValue {
    Known(bool),
    Unknown,
}
```

M1 では flags を観測しない。M3 で `cmp` / `test` / `jcc` と一緒に導入する。

## Metadata

IR から codegen した結果は、必ず PC map と fixup 情報を持つ。

```rust
pub struct PcMapEntry {
    pub source: X86Va,
    pub target: ArmPc,
    pub kind: PcMapKind,
}

pub enum Fixup {
    Arm64Branch26 {
        at: ArmPc,
        target: FixupTarget,
    },
    RuntimeHelper {
        at: ArmPc,
        helper: HelperId,
    },
}
```

## 初期 invariant checker

- [ ] block に terminator がある。
- [ ] branch target が存在する。
- [ ] fallthrough target が存在する。
- [ ] block range が重ならない。
- [ ] unsupported op は明示的に残る。
- [ ] PC map が source PC を失っていない。
