# Binary-to-Binary Compiler 研究 TODO

## 目的

x86_64 などの既存バイナリを ARM64 などへ変換する 汎用binary-to-binary compiler の研究を行う。

重点は、変換コアと OS/ABI/ローダー固有部分を分離し、Wine のような互換レイヤーへ接続しやすい構成を作ること。

並行研究として、Wasm build 可能なオープンソースソフトウェアを `wasm2c` で C へ戻し、`clang` のみが提供される NDA 系ターゲットへ最小労力で移植する platform adapter 研究を行う。この研究とは、host helper ABI、platform abstraction、artifact packaging、regression 基盤を共有できるようにする。

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
- NDA 系ターゲット固有の知識は closed platform adapter に閉じ、open source core、helper ABI、test fake platform へ漏らさない。
- Bara の binary translation と wasm2c 移植研究は、入力形式は違っても user-space runtime / platform bridge / artifact packaging で合流できるようにする。

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
- platform 固有の描画、入力、音声、window/event loop、file/system integration は抽象 interface 越しに扱う。
- open な fake backend で platform abstraction を検証し、NDA adapter は薄い実装層に限定する。

## 初期設計ドキュメント

- [初期スコープ](docs/scope.md)
- [クリーンルーム運用](docs/clean-room.md)
- [コーディングルール](docs/coding-rules.md)
- [初期 IR 設計](docs/ir.md)
- [Rosetta Oracle 検証ワークフロー](docs/test-oracle.md)
- [進行履歴](docs/progress.md)

## 大項目 TODO

現在の最小 `hello world` milestone は完了済み。

到達済み:

- x86 raw function fixture を decode / lift / ARM64 emit できる。
- ARM64 machine code artifact をファイルへ出力できる。
- macOS ARM64 executable artifact として package できる。
- 生成 executable を OS 上で起動し、実 OS stdout に `hello world\n` を出せる。

ここから先は、fixture 専用の成功経路を実バイナリ対応へ広げる。

### B1: Hello World 成果物の安定化

- [ ] 生成 executable の smoke test を blackbox report に含める。
- [ ] `link-fixture-arm64-stdout-main` の出力、stdout、exit status を stable JSON report にする。
- [ ] temporary assembly / toolchain 呼び出しの失敗分類を整理する。
- [ ] native artifact 関連の CLI テストを `main.rs` から責務別 module へ分割する。
- [ ] `docs/hello-world-roadmap.md` を完了済みロードマップとして整理し、次フェーズ文書へ接続する。

### B2: 実行可能成果物モデル

- [ ] raw ARM64 bytes、native assembly source、linked executable を区別する domain type を作る。
- [ ] native artifact の metadata を JSON として出力する。
- [ ] generated code、stdout data、toolchain command、output path の責務を分ける。
- [ ] 外部 `clang` packaging と将来の pure Mach-O writer を差し替え可能な境界にする。
- [ ] macOS ARM64 以外の host では classified unsupported として安定出力する。

### B3: Mach-O 出力境界

- [ ] 既存の入力 Mach-O parser と出力 Mach-O writer の責務を分ける。
- [ ] 最小 ARM64 Mach-O executable writer を pure function として設計する。
- [ ] `_main` entry、`__TEXT`、`__const`、最小 load commands の公開仕様ベース model を定義する。
- [ ] `clang` packaging 経路と pure writer 経路の出力差分を検証する。
- [ ] writer が育つ場合は `bara-oracle` から独立した crate へ切り出す。

### B4: x86 syscall / libc 境界

- [ ] x86 `syscall` を実行せず、まず public ABI 上の request として IR に表現する。
- [ ] stdout 相当を Bara host helper から native stdout emission へ変換する境界を明文化する。
- [ ] macOS / Linux / Windows の OS ABI 差分を helper boundary で分離する。
- [ ] libc / dyld / import 呼び出しを直接模倣せず、public symbol/import model として扱う。
- [ ] unsupported syscall / external call の分類と report schema を安定させる。

### B5: Control Flow / Stack / Call

- [ ] basic block 分割を導入する。
- [ ] direct branch / conditional branch / fallthrough を typed terminator として扱う。
- [ ] flags model を定義し、`cmp` / `test` / `jcc` を段階的に追加する。
- [ ] stack pointer / return address / direct call の最小 semantics を実装する。
- [ ] nested call を含む fixture を native executable artifact として実行する。

### B6: 実 Mach-O 入力からの standalone 実行

- [ ] Mach-O backed `return_42` 入力を native executable artifact へ変換する。
- [ ] Mach-O backed `hello world` 入力を native executable artifact へ変換する。
- [ ] input Mach-O の entry / segment / stack metadata を output packaging に渡す。
- [ ] fixture 専用 host trap JSON への依存を減らし、binary metadata から必要情報を得る。
- [ ] malformed / unsupported Mach-O の blocker classification を artifact 生成でも維持する。

### B7: Oracle / Regression 基盤

- [ ] generated executable を実プロセスとして走らせる regression gate を追加する。
- [ ] expected / actual に stdout、stderr、exit status、return value、artifact metadata を含める。
- [ ] Rosetta black-box oracle 経路を clean-room ルール内で再検討する。
- [ ] fixture shrink / failure classification / corpus update の運用を作る。
- [ ] CI で quick tests と host-specific native artifact tests を分ける。

### B8: PE / Wine 接続前段

- [ ] PE parser の最小 scope を決める。
- [ ] `.text` / `.rdata` / import table の domain model を設計する。
- [ ] Windows x64 ABI と helper boundary の対応を整理する。
- [ ] Wine へ渡すべき責務と Bara が持つべき責務を文書化する。
- [ ] hello world 相当を PE fixture から native artifact へ変換する長期計画を立てる。

### B9: x86 32-bit 対応を見越した設計

- [ ] `x86_64` 専用の前提を public API 名、型名、metadata schema に固定しすぎない。
- [ ] source ISA mode を `x86_64` / `x86_32` として明示する domain type を設計する。
- [ ] decoder / lifter は 64-bit 固有部分と 32-bit 共有可能部分を分ける。
- [ ] register model は `rax` だけでなく `eax` / `ax` / `al` などの部分レジスタへ拡張可能にする。
- [ ] address size / operand size / stack width を source mode から決められる設計にする。
- [ ] 32-bit calling convention と 64-bit calling convention を ABI type で分離する。
- [ ] 32-bit PE / ELF / Mach-O 入力を将来追加できる binary format metadata にする。
- [ ] segmentation、x87、MMX、古いSSEコードなど、32-bit x86固有の難所を unsupported 分類できるようにする。
- [ ] `x86 -> arm64` と `x86_64 -> arm64` を同じ IR / ARM64 backend に載せる境界を設計する。
- [ ] 必要なら `bara-isa-x86` 内を `x86_32` / `x86_64` / `shared` に分割するか、crate分割を検討する。

### B10: ユーザー空間完結 runtime architecture

- [ ] kernel extension、private kernel hook、private dyld behavior を前提にしない。
- [ ] loader、translation cache、runtime helper、artifact cache を user-space process 内に閉じる。
- [ ] executable memory は public OS API (`mmap` / `mprotect` など) 経由に限定する。
- [ ] JIT / AOT / fallback interpreter を同じ user-space runtime 境界から選べる設計にする。
- [ ] syscall / OS API bridge は helper boundary として明示し、core IR / emit へ混ぜない。
- [ ] signal / exception / thread / TLS / memory protection は user-space loader model として段階的に扱う。
- [ ] Wine 接続時に Bara が持つ責務と Wine へ委譲する責務を文書化する。
- [ ] process-wide 互換性が必要な箇所は、kernel 統合ではなく user-space loader / runtime metadata として扱う。
- [ ] macOS code signing / W^X / hardened runtime 制約は public API と documented behavior の範囲で整理する。
- [ ] Rosetta 2 の OS 統合型構成とは異なり、Bara は user-space binary translation runtime として設計する。

### B11: Platform abstraction / wasm2c 研究との合流

- [ ] Bara helper boundary と wasm2c platform imports が共有できる host helper ABI を設計する。
- [ ] stdout、file I/O、time、memory allocation、input、audio、rendering、window/event loop を platform capability として分ける。
- [ ] open source core には platform interface、fake backend、tests だけを置く。
- [ ] NDA 系ターゲット固有の API、描画 surface binding、event pump、audio device binding、build glue は closed adapter に閉じる。
- [ ] `wasm -> wasm2c -> C -> clang target` 経路と `x86/x86_64 -> Bara IR -> ARM64/native artifact` 経路を同じ platform abstraction へ接続する。
- [ ] platform adapter は C ABI / Rust FFI / helper table のどれで接続するかを比較する。
- [ ] rendering backend は immediate mode / retained mode / GPU surface handoff の責務を分ける。
- [ ] NDA adapter がなくても public fake backend で CI と regression を回せるようにする。
- [ ] platform capability metadata を artifact report に含める。
- [ ] 将来の Wine 接続では、Wine API bridge と wasm2c platform bridge が同じ helper abstraction を共有できるか検証する。

### B12: LLVM IR / Wasm を副出力ターゲットとして扱う

- [ ] Bara IR を LLVM IR や Wasm に置き換えず、semantic IR として維持する。
- [ ] LLVM IR は backend 実験、object generation 比較、最適化比較の副出力として扱う。
- [ ] Wasm は sandboxed test runner、可視化、portable verifier target の副出力として扱う。
- [ ] `HostTrap` / helper request は LLVM external declaration または Wasm import へ落とす。
- [ ] LLVM / Wasm に落とせない semantics は metadata と helper boundary で保持する。
- [ ] x86_32 / x86_64 の source mode、PC map、flags、部分レジスタ情報を副出力で失わない方針を決める。

## 詳細設計 TODO / 設計メモ

リファクタリング、分割方針、設計判断は
[docs/design-todo.md](docs/design-todo.md) に分離して管理する。

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

- [x] Rust workspace を作る。
- [x] package / crate は技術駆動ではなく、関心ごとのドメイン駆動で切る。
- [x] I/O は専用ディレクトリに集約し、decode / IR / emit などのロジックへ散らさない。
- [x] `bara-isa-x86` crate を作る。
- [x] `bara-ir` crate を作る。
- [x] `bara-arm64` crate を作る。
- [x] `bara-oracle` crate を作る。
- [x] `bara-runtime` crate を作る。
- [x] `btbc-cli` crate を作る。
- [ ] `btbc-tests` または integration test 用 crate を作る。
- [ ] 後で Haskell verifier を追加できるディレクトリ構成にする。
- [x] Rust supply-chain 検証を追加する。
- [x] 不可視文字 / Unicode 制御文字検査を追加する。
- [x] ローカル一括検証 script を追加する。
- [x] pre-commit hook installer を追加する。
- [x] CVE baseline 運用を追加する。
- [x] Nix build / package 検証を追加する。
- [x] VS Code Rust エディタ設定を追加する。
- [ ] 機能が揃ってきたら CI workflow を追加する。

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
    bara-runtime/
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

- [x] `X86Va` を newtype として定義する。
- [ ] `X86Rva` を newtype として定義する。
- [x] `ArmPc` を newtype として定義する。
- [ ] `ImageBase` を newtype として定義する。
- [x] `BlockId` を定義する。
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
- [x] raw x86_64 bytes を decode する。
- [ ] basic block に分割する。
- [x] typed IR に lift する。
- [x] unsupported instruction を明示的に表現する。

初期対応命令:

- [x] `mov reg, imm`
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

- [x] ARM64 machine code emitter を作る。
- [x] executable buffer へ書き込めるようにする。
- [x] `mmap` / `mprotect` など executable memory 管理を runtime 側へ分離する。
- [x] `mov` / `ret` の最小 emission を実装する。
- [ ] `add/sub/cmp` の最小 emission を実装する。
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

- [x] Rust workspace を作る。
- [x] `bara-ir` に型モデルを作る。
- [x] M1 実行チェック用 CLI を作る。
- [ ] raw x86_64 bytes 入力を受ける汎用 CLI を作る。
- [x] `mov eax, imm; ret` だけ decode/lift する。
- [x] ARM64 emitter で即値 return を出す。
- [x] executable buffer で実行する。
- [x] `return_value` を JSON で出す。
- [ ] 同じケースを x86_64 Mach-O としてビルドし、Rosetta oracle で JSON を出す。
- [ ] expected/actual を比較する。
