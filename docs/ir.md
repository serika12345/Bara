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

`Cmp` は x86 の `cmp` と同じく destination operand を書き戻さず、
後続の flags / conditional branch lowering が読む status flags を更新する
op として扱う。現時点で decode / lift するのは `cmp eax, imm8/imm32` に
限り、ARM64 emit は flag lowering 実装前の explicit unsupported として止める。

`Test` は x86 の `test` と同じく operand 同士の bitwise AND 結果を
書き戻さず、flags 更新だけを表す op として扱う。現時点で decode / lift
するのは `test eax,eax` に限り、ARM64 emit は flag lowering 実装前の
explicit unsupported として止める。

```rust
pub enum HostTrapKind {
    Stdout,
}
```

`HostTrapKind::Stdout` は Bara host helper request へ写像される。
これは guest OS syscall や runtime 内部 helper ではなく、manifest や
runtime 境界で解決される Bara 定義の host effect capability である。

```rust
pub enum HostHelperRequest {
    WriteStdout,
}

pub struct HostHelperAbi {
    name: HostHelperName,
    signature: HostHelperSignature,
}

pub enum HostHelperName {
    WriteStdout,
}

pub enum HostHelperSignature {
    PtrLenToUnit,
}
```

## Terminator

```rust
pub enum Terminator {
    Return,
    BoundaryRequest {
        request: BoundaryRequest,
    },
    Fallthrough {
        target: X86Va,
    },
    DirectJump { target: X86Va },
    CondJump {
        condition: X86Cond,
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

現時点では short `je/jz rel8` を `X86Cond::Equal`、short `jne/jnz rel8` を
`X86Cond::NotEqual` の `CondJump` へ decode / lift し、ARM64 emit は
`cmp` / `test` が更新した flags を `b.eq` / `b.ne` へ lower する。その他の
`jcc` 条件と rel32 form は B5 の後続小ステップで扱う。

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

pub enum RuntimeHelper {
    CallExternal,
    Unimplemented,
    Exit,
}

pub struct RuntimeHelperAbi {
    name: RuntimeHelperName,
    signature: RuntimeHelperSignature,
}

pub enum RuntimeHelperName {
    HelperCallExternal,
    HelperUnimplemented,
    HelperExit,
}

pub enum RuntimeHelperSignature {
    StateExternalSymbolToUnit,
    StateUnimplementedReasonToUnit,
    StateExitCodeToNever,
}

pub struct ExternalSymbolId(u32);

pub struct ExternalSymbolImport {
    symbol: ExternalSymbolId,
    target: ExternalImportTarget,
}

pub enum ExternalImportTarget {
    Unresolved,
    PublicSymbol(PublicSymbolImport),
}

pub enum PublicSymbolImport {
    Libc(PublicLibcSymbol),
    Dyld(PublicDyldSymbol),
}

pub enum PublicLibcSymbol {
    Puts,
    Write,
}

pub enum PublicDyldSymbol {
    DyldStubBinder,
}

pub struct ExternalCallRequest {
    import: ExternalSymbolImport,
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

`RuntimeHelper` は変換済みコードが runtime 内部へ戻るための helper ABI を
表し、`HostHelperRequest` は stdout など host-observable effect の
capability を表す。この 2 つを分けることで、`write_stdout` のような
Bara host helper を syscall / libc / OS API の直接実装と混ぜない。

`ExternalSymbolImport` は libc / dyld / import call を直接模倣するための
実行指示ではない。public symbol identity を IR に残すための model であり、
実行、動的 loader、libc ABI 再現、dyld 挙動の模倣は別の helper boundary で
解決または unsupported 分類する。

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
    cf: FlagValue,
    pf: FlagValue,
    af: FlagValue,
    zf: FlagValue,
    sf: FlagValue,
    of: FlagValue,
}

pub enum FlagValue {
    Known(bool),
    Unknown,
}
```

flags は `Flags::new(...)` または `Flags::unknown()` で作り、`cf()` /
`pf()` / `af()` / `zf()` / `sf()` / `of()` accessor から読む。

現時点では `cmp eax, imm8/imm32` を `IrOp::Cmp` へ、`test eax,eax` を
`IrOp::Test` へ decode / lift し、short `je/jz rel8` と `jne/jnz rel8` を
`CondJump` へ decode / lift する。ARM64 emit は `cmp x0,#imm12`、
`tst x0,x0`、`b.eq` / `b.ne` の最小 lowering を持つ。その他の `jcc` 条件と
rel32 form は B5 の後続小ステップで扱う。

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
