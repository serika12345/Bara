# Binary-to-Binary Compiler 研究 TODO

## 目的

x86_64 などの既存バイナリを ARM64 などへ変換する 汎用binary-to-binary compiler の研究を行う。

重点は、変換コアと OS/ABI/ローダー固有部分を分離し、Wine のような互換レイヤーへ接続しやすい構成を作ること。

Rosetta は実装の参考にせず、arm64 macOS 上で x86_64 バイナリを実行するブラックボックス oracle としてだけ使う。

## 基本方針

- 最初から Wine 全体や GUI アプリを狙わない。
- まずは極小の x86_64 関数片を ARM64 に変換して実行する。
- 最初の成功条件は `mov eax, 42; ret` 相当が 42 を返すこと。
- コードだけでなく metadata も一次成果物として扱う。
- AOT 専用にしすぎず、将来の fallback JIT/interpreter/QEMU 接続を想定する。
- 実装は Rust を主軸にする。
- Haskell は仕様モデル、property test、検証器として使う。
- 関数型プログラミングの考え方に寄せ、シグネチャで仕様と境界が読める API にする。

## 初期決定事項

- 入力は raw x86_64 function bytes から始める。
- 実行単位はプロセスではなく関数単位にする。
- 初期 ABI は引数なし、`rax` 戻り値のみとする。
- 初期実装は `CpuState` 中心で、検証しやすさを優先する。
- flags は最初から lazy にせず、必要になった時点で明示的に materialize する。
- Rosetta oracle の比較対象は、まず `return_value` / `exit_status` / `stdout` / `stderr` に限定する。
- 未対応命令は panic ではなく `UnsupportedInstruction` として分類できるようにする。
- Rust の `unsafe` は executable memory と function pointer 呼び出し境界に閉じ込める。
- `unsafe` を含む実装は runtime 境界へ局所化し、core の decode / lift / IR / metadata / verifier へ広げない。

## 初期設計ドキュメント

- [初期スコープ](docs/scope.md)
- [クリーンルーム運用](docs/clean-room.md)
- [コーディングルール](docs/coding-rules.md)
- [初期 IR 設計](docs/ir.md)
- [Rosetta Oracle 検証ワークフロー](docs/test-oracle.md)

## ライセンス/境界ルール

Rosetta は期待出力生成器としてのみ使う。

既存の Linux user-space 変換レイヤーは、比較対象・調査対象として扱う。

研究対象:

- FEX-Emu
- Box64
- QEMU user-mode

これらは、arm64 Linux 上で x86 / x86_64 Linux バイナリを動かす既存実装、または汎用 user-mode emulation の比較対象として調べる。調査の目的は、問題領域、互換レイヤーの境界、syscall / dynamic linker / signal / threading / memory model の論点を整理することに限定する。

OK:

- Intel/AMD ISA manual
- ARM Architecture Reference Manual
- System V ABI / Windows x64 ABI
- Mach-O / PE / ELF の公開仕様
- 自前テスト
- Rosetta をブラックボックス実行して得た入出力結果
- FEX-Emu / Box64 / QEMU user-mode の公開ドキュメント、公開 README、外部から観測できる挙動

NG:

- Rosetta バイナリの逆アセンブル結果を実装根拠にする
- Rosetta 内部構造を模倣する
- Apple 固有 metadata/format をコピーする
- 非公開 symbol や内部アルゴリズムを実装へ持ち込む
- 既存変換レイヤーの内部実装、非公開知識、またはコード構造を Bara の実装根拠にする

## 目標アーキテクチャ

```text
input binary / x86_64 snippet
        |
        v
Frontend
  - file format parser: raw / PE / Mach-O / ELF
  - x86_64 decoder
  - relocation/import/symbol extraction
        |
        v
Normalized Program IR
  - images / sections / symbols
  - code ranges
  - relocations / imports / exports
  - unwind metadata
        |
        v
ISA Translation Core
  - x86_64 semantics
  - flags model
  - memory operands
  - control-flow lowering
        |
        v
ARM64 Codegen
  - machine code emission
  - branch fixups
  - helper calls
        |
        v
Runtime Metadata
  - x86 PC <-> ARM64 PC map
  - fixup records
  - helper references
  - indirect branch metadata
  - unwind/exception map
        |
        v
Output / Runner
```

## Rust 側の構成 TODO

- [ ] Rust workspace を作る。
- [ ] package / crate は技術駆動ではなく、関心ごとのドメイン駆動で切る。
- [ ] I/O は専用ディレクトリに集約し、decode / IR / emit などのロジックへ散らさない。
- [ ] `bara-isa-x86` crate を作る。
- [ ] `bara-ir` crate を作る。
- [ ] `bara-arm64` crate を作る。
- [ ] `bara-oracle` crate を作る。
- [ ] `btbc-runtime` crate を作る。
- [ ] `btbc-cli` crate を作る。
- [ ] `btbc-tests` または integration test 用 crate を作る。
- [ ] 後で Haskell verifier を追加できるディレクトリ構成にする。

候補構成:

```text
btbc/
  crates/
    bara-isa-x86/
      src/
        decode/
        lift/
    bara-ir/
      src/
        program/
        block/
        validate/
    bara-arm64/
      src/
        emit/
        fixup/
    bara-oracle/
      src/
        io/
        rosetta/
        compare/
    btbc-runtime/
      src/
        io/
        executable_memory/
        runner/
    btbc-cli/
      src/
        io/
  spec/
    haskell-verifier/
  tests/
    cases/
    corpus/
    expected/
    actual/
```

## 型モデル TODO

- [ ] `X86Va` を newtype として定義する。
- [ ] `X86Rva` を newtype として定義する。
- [ ] `ArmPc` を newtype として定義する。
- [ ] `ImageBase` を newtype として定義する。
- [ ] `BlockId` を定義する。
- [ ] `SymbolId` を定義する。
- [ ] `HelperId` を定義する。
- [ ] `CpuState` を定義する。
- [ ] `PcMap` / `PcMapEntry` を定義する。
- [ ] `Fixup` enum を定義する。
- [ ] `BlockTerminator` enum を定義する。
- [ ] source PC と target PC が混ざらない API にする。

例:

```rust
struct X86Va(u64);
struct X86Rva(u32);
struct ArmPc(u64);

enum BlockTerminator {
    DirectJump { target: X86Va },
    CondJump { taken: X86Va, fallthrough: X86Va, cc: X86Cond },
    IndirectJump { operand: X86Operand },
    Call { target: CallTarget },
    Return,
    Unsupported { reason: UnsupportedReason },
}
```

## 最小 x86_64 対応 TODO

最初は関数単位の小さな命令列だけを対象にする。

- [ ] `iced-x86` の採用を検討する。
- [ ] raw x86_64 bytes を decode する。
- [ ] basic block に分割する。
- [ ] typed IR に lift する。
- [ ] unsupported instruction を明示的に表現する。

初期対応命令:

- [ ] `mov reg, imm`
- [ ] `mov reg, reg`
- [ ] `add`
- [ ] `sub`
- [ ] `cmp`
- [ ] `test`
- [ ] `jmp`
- [ ] `jcc`
- [ ] `call direct`
- [ ] `ret`
- [ ] `push`
- [ ] `pop`

後回し:

- [ ] SIMD/SSE
- [ ] x87
- [ ] AVX
- [ ] segment register
- [ ] syscall
- [ ] self-modifying code

## IR TODO

- [ ] basic block は必ず terminator を持つようにする。
- [ ] fallthrough を暗黙にしない。
- [ ] flags model を定義する。
- [ ] memory operand を明示する。
- [ ] direct branch と indirect branch を分ける。
- [ ] external helper call を IR 上に表現する。
- [ ] IR invariant checker を Rust 側にも用意する。

最初は最適化しない。

- [ ] flags は毎回 materialize してよい。
- [ ] register allocation は固定対応でよい。
- [ ] x86 state はメモリ構造体中心でよい。

## ARM64 emitter TODO

- [ ] ARM64 machine code emitter を作る。
- [ ] executable buffer へ書き込めるようにする。
- [ ] `mmap` / `mprotect` など executable memory 管理を runtime 側へ分離する。
- [ ] `mov/add/sub/cmp/ret` の最小 emission を実装する。
- [ ] conditional branch の fixup を実装する。
- [ ] direct call / return の最小実装を追加する。
- [ ] helper call ABI を定義して emitter から呼べるようにする。

## Runtime ABI TODO

変換済みコードが困った時に呼ぶ helper ABI を先に決める。

- [ ] `CpuState` の layout を `repr(C)` で定義する。
- [ ] caller/callee saved の扱いを決める。
- [ ] helper 呼び出し規約を決める。
- [ ] `helper_unimplemented` を実装する。
- [ ] `helper_indirect_branch` を実装する。
- [ ] `helper_call_external` を実装する。
- [ ] `helper_exit` を実装する。
- [ ] fallback engine 接続用 hook を用意する。

初期 helper:

```text
helper_unimplemented(state, opcode)
helper_indirect_branch(state, target)
helper_call_external(state, symbol_id)
helper_exit(state, code)
```

## Metadata TODO

コンパイラ出力は code だけにしない。

- [ ] `compiled.ir.json` を出せるようにする。
- [ ] `pcmap.json` を出せるようにする。
- [ ] `fixups.json` を出せるようにする。
- [ ] `helpers.json` を出せるようにする。
- [ ] `final_state.json` を runner から出せるようにする。
- [ ] schema を安定させる。
- [ ] 最初は JSON、速度が問題になったら CBOR/MessagePack などを検討する。

必要な metadata:

```text
target machine code
source <-> target PC map
fixup records
runtime helper references
indirect branch metadata
unwind/exception map
state layout description
cache validation identity
```

## Rosetta ブラックボックス oracle TODO

arm64 macOS 上で x86_64 Mach-O を Rosetta 実行し、期待結果を得る。

- [ ] `clang -target x86_64-apple-macos...` で x86_64 テストバイナリを生成する。
- [ ] x86_64 oracle runner を作る。
- [ ] oracle runner は結果を JSON で出す。
- [ ] Rosetta 実行結果を `expected.json` として保存する。
- [ ] BTB 出力を ARM64 native runner で実行する。
- [ ] ARM64 実行結果を `actual.json` として保存する。
- [ ] `expected.json` と `actual.json` を比較する。

基本フロー:

```text
testcase
  |
  +-- build x86_64 Mach-O
  |       |
  |       v
  |   run under Rosetta
  |       |
  |       v
  |   expected.json
  |
  +-- btbc compile
          |
          v
      run native ARM64
          |
          v
      actual.json
          |
          v
      compare
```

初期 JSON:

```json
{
  "case_id": "return_42",
  "exit_status": 0,
  "return_value": 42,
  "stdout": "",
  "stderr": ""
}
```

## Haskell verifier TODO

Haskell は実装本体ではなく、仕様・検証に使う。

- [ ] Haskell package を作る。
- [ ] testcase schema を読む。
- [ ] Rust が吐いた IR schema を読む。
- [ ] PC map schema を読む。
- [ ] fixup schema を読む。
- [ ] final state schema を読む。
- [ ] 小さな x86 semantics interpreter を作る。
- [ ] IR invariant checker を作る。
- [ ] PC map invariant checker を作る。
- [ ] fixup consistency checker を作る。
- [ ] expected/actual final state comparator を作る。
- [ ] QuickCheck または Hedgehog で testcase generator を作る。
- [ ] failing case を shrink できるようにする。

最初の Haskell モデル:

```haskell
data Instr
  = Mov Reg Operand
  | Add Reg Operand
  | Sub Reg Operand
  | Cmp Reg Operand
  | Test Reg Operand
  | Jmp Addr
  | Jcc Cond Addr
  | Call Addr
  | Ret
```

Haskell が検査すること:

- [ ] basic block が terminator を持つ。
- [ ] branch target が存在する。
- [ ] source PC range が重ならない。
- [ ] x86 PC と ARM64 PC の map が矛盾しない。
- [ ] fixup がすべて解決可能。
- [ ] 最終 state が oracle と一致する。

## 自動化サイクル TODO

コーディングエージェントで失敗ケース駆動の改善サイクルを回す。

```text
1. testcase generator が x86_64 小プログラムを生成
2. x86_64 Mach-O を clang でビルド
3. Rosetta で実行して expected.json を取得
4. Rust BTB で同じ入力を ARM64 へ変換
5. native ARM64 runner で actual.json を取得
6. verifier が expected と actual を比較
7. 失敗したら最小化/shrink
8. エージェントが失敗ケース、IR、trace を読んで修正
9. regression corpus に追加
```

エージェントに渡してよい情報:

- [ ] 入力 x86_64 snippet
- [ ] Rust が生成した IR
- [ ] Rust が生成した ARM64 disassembly
- [ ] `expected.json`
- [ ] `actual.json`
- [ ] 差分
- [ ] 公開 ISA 仕様に基づく対応命令仕様
- [ ] 既存テスト

エージェントに渡さない情報:

- [ ] Rosetta の逆アセンブル結果
- [ ] Rosetta の内部 symbol に基づく設計
- [ ] Apple 固有内部 metadata

失敗分類:

- [ ] `DecodeError`
- [ ] `UnsupportedInstruction`
- [ ] `WrongRegisterValue`
- [ ] `WrongFlags`
- [ ] `WrongMemory`
- [ ] `WrongBranchTarget`
- [ ] `WrongCallReturn`
- [ ] `WrongExternalCall`

## CI TODO

- [ ] 通常 Rust test を作る。
- [ ] oracle が不要な unit test を作る。
- [ ] arm64 macOS 上だけで Rosetta oracle test を走らせる。
- [ ] quick test と oracle test を分離する。
- [ ] nightly で randomized test を走らせる。
- [ ] 失敗ケースを corpus に保存する。

CI レーン:

```text
quick:
  cargo test
  decoder/lifter tests
  fixed corpus

oracle:
  build x86_64 Mach-O
  run under Rosetta
  run BTB output
  compare

nightly:
  randomized tests
  shrink failures
  corpus update candidate
```

## マイルストーン

### M1: 値を返す最小関数

- [ ] `mov eax, 42; ret` を decode する。
- [ ] IR に lift する。
- [ ] ARM64 に emit する。
- [ ] native ARM64 runner で実行する。
- [ ] 42 が返る。
- [ ] `pcmap.json` を出す。

### M2: 算術

- [ ] `add` を実装する。
- [ ] `sub` を実装する。
- [ ] 複数の戻り値テストを通す。

### M3: flags と分岐

- [ ] `cmp` を実装する。
- [ ] `test` を実装する。
- [ ] `jcc` を実装する。
- [ ] if/else 相当を通す。
- [ ] 簡単な loop を通す。

### M4: stack / call / ret

- [ ] `push` を実装する。
- [ ] `pop` を実装する。
- [ ] direct `call` を実装する。
- [ ] nested call を通す。

### M5: helper 経由の hello world

- [ ] external symbol を helper に逃がす。
- [ ] `helper_call_external` を実装する。
- [ ] `puts` 相当を host 側で呼ぶ。
- [ ] hello world 相当を通す。

### M6: Rosetta oracle 比較

- [ ] x86_64 oracle runner を作る。
- [ ] Rosetta 実行で `expected.json` を出す。
- [ ] BTB 実行で `actual.json` を出す。
- [ ] 比較器を実装する。

### M7: Haskell verifier

- [ ] tiny x86 semantics を実装する。
- [ ] IR invariant を検証する。
- [ ] final state を比較する。
- [ ] QuickCheck/Hedgehog で小ケース生成を始める。

### M8: PE/Wine 前段

- [ ] 極小 PE parser を追加する。
- [ ] `.text` を取り出す。
- [ ] `.rdata` を取り出す。
- [ ] import call を helper に逃がす。
- [ ] Wine loader 接続の境界を設計する。

### M9: fallback

- [ ] unimplemented instruction fallback を決める。
- [ ] unknown indirect target fallback を決める。
- [ ] 小さな interpreter fallback を検討する。
- [ ] QEMU/TCG fallback の接続可能性を調査する。

## 当面の最短 TODO

すぐ着手するなら以下。

- [ ] Rust workspace を作る。
- [ ] `bara-ir` に型モデルを作る。
- [ ] raw x86_64 bytes 入力を受ける CLI を作る。
- [ ] `mov eax, imm; ret` だけ decode/lift する。
- [ ] ARM64 emitter で即値 return を出す。
- [ ] executable buffer で実行する。
- [ ] `return_value` を JSON で出す。
- [ ] 同じケースを x86_64 Mach-O としてビルドし、Rosetta oracle で JSON を出す。
- [ ] expected/actual を比較する。
