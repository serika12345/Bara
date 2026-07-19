# Binary-to-Binary Compiler 研究 TODO

## 目的

x86_64 などの既存バイナリを ARM64 などへ変換する 汎用binary-to-binary compiler の研究を行う。

重点は、変換コアと OS/ABI/ローダー固有部分を分離し、Wine のような互換レイヤーへ接続しやすい構成を作ること。

Bara の本流 TODO は、まず実 x86_64 macOS アプリを user-space runtime
で起動できる状態を目指す。wasm2c 移植、NDA 系ターゲット adapter、
LLVM/Wasm 副出力などの未確立な派生研究は
[将来構想メモ](docs/future-research-concepts.md) に分離し、本流の
TODO としては扱わない。

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
- PE / Wine 接続へ進む前に、実 x86_64 macOS アプリの起動を本流の
  中間到達点にする。続いて x86 32-bit アプリ対応を入れる。32-bit 対応は
  blocker なら飛ばしてよいが、Wine 接続前に先に処理するのが望ましい。

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
- [進行履歴](docs/progress.md)
- [将来構想メモ](docs/future-research-concepts.md)

## 線形実装ロードマップ

この節が実装順の唯一の source of truth である。上から順に読み、最初の
未完了項目を次の作業候補にする。詳細設計や参照メモは下位節や別文書に
分離するが、別のマイルストーン一覧を参照して実行順を決めない。

現在の最小 `hello world` milestone は完了済み。

到達済み:

- x86 raw function fixture を decode / lift / ARM64 emit できる。
- ARM64 machine code artifact をファイルへ出力できる。
- macOS ARM64 executable artifact として package できる。
- 生成 executable を OS 上で起動し、実 OS stdout に `hello world\n` を出せる。

ここから先は、fixture 専用の成功経路を実バイナリ対応へ広げる。

### B1: Hello World 成果物の安定化

- [x] `mov eax, 42; ret` を decode する。
- [x] `mov eax, 42; ret` を typed IR に lift する。
- [x] `mov eax, 42; ret` を ARM64 に emit する。
- [x] native ARM64 runner で実行し、42 が返ることを確認する。
- [x] `return_value` を stable JSON として出力する。
- [x] 生成 executable の smoke test を blackbox report に含める。
- [x] `link-fixture-arm64-stdout-main` の出力、stdout、exit status を stable JSON report にする。
- [x] temporary assembly / toolchain 呼び出しの失敗分類を整理する。
- [x] native artifact 関連の CLI テストを `main.rs` から責務別 module へ分割する。
- [x] `docs/hello-world-roadmap.md` を完了済みロードマップとして整理し、次フェーズ文書へ接続する。

### B2: 実行可能成果物モデル

- [x] raw ARM64 bytes、native assembly source、linked executable を区別する domain type を作る。
- [x] native artifact の metadata を JSON として出力する。
- [x] generated code、stdout data、toolchain command、output path の責務を分ける。
- [x] 外部 `clang` packaging と将来の pure Mach-O writer を差し替え可能な境界にする。
- [x] macOS ARM64 以外の host では classified unsupported として安定出力する。

### B3: Mach-O 出力境界

- [x] 既存の入力 Mach-O parser と出力 Mach-O writer の責務を分ける。
- [x] 最小 ARM64 Mach-O executable writer を pure function として設計する。
- [x] `_main` entry、`__TEXT`、`__const`、最小 load commands の公開仕様ベース model を定義する。
- [x] `clang` packaging 経路と pure writer 経路の出力差分を検証する。
- [x] writer が育つ場合は `bara-oracle` から独立した crate へ切り出す。

### B4: x86 syscall / libc 境界

- [x] x86 `syscall` を実行せず、まず public ABI 上の request として IR に表現する。
- [x] external symbol / import call を core logic で直接呼ばず、helper request に逃がす。
- [x] `helper_call_external`、`helper_unimplemented`、`helper_exit` の最小 ABI を定義する。
- [x] `puts` / `write` 相当を host helper 経由で stdout に出せるようにする。
- [x] stdout 相当を Bara host helper から native stdout emission へ変換する境界を明文化する。
- [x] macOS / Linux / Windows の OS ABI 差分を helper boundary で分離する。
- [x] libc / dyld / import 呼び出しを直接模倣せず、public symbol/import model として扱う。
- [x] unsupported syscall / external call の分類と report schema を安定させる。

### B5: Control Flow / Stack / Call

- [x] `add` / `sub` を control-flow fixture と regression corpus の中で扱う。
- [x] basic block 分割を導入する。
- [x] basic block は必ず typed terminator を持ち、fallthrough を暗黙にしない。
- [x] direct branch / conditional branch / fallthrough を typed terminator として扱う。
- [x] flags model を定義する。
- [x] `cmp eax, imm8/imm32` を flags-producing IR op として decode / lift する。
- [x] `test eax,eax` を flags-producing IR op として decode / lift する。
- [x] short `je/jz rel8` を `CondJump` terminator として decode / lift する。
- [x] short `jne/jnz rel8` を `CondJump` terminator として decode / lift する。
- [x] `cmp` / `test` と short `je/jne rel8` の ARM64 branch lowering を追加する。
- [x] parity 以外の `jcc` 条件と rel32 を段階的に追加する。
- [x] short `jmp rel8` を `DirectJump` terminator として decode / lift する。
- [x] direct `jmp` fixture を ARM64 emit / runtime まで通す。
- [x] if/else 相当の conditional branch fixture を通す。
- [x] 簡単な loop を通す。
- [x] `push` / `pop` を実装する。
- [x] stack pointer / return address / direct call の最小 semantics を実装する。
- [x] direct `call` / `ret` の最小実装を追加する。
- [x] nested call を含む fixture を native executable artifact として実行する。
- [x] conditional branch の ARM64 fixup を実装する。
- [x] direct call の ARM64 fixup を実装する。
- [x] branch / fallthrough / call target existence と source PC range overlap
  を validation report で検査する。

### B6: 実 Mach-O 入力からの standalone 実行

- [x] Mach-O backed `return_42` 入力を native executable artifact へ変換する。
- [x] Mach-O backed `hello world` 入力を native executable artifact へ変換する。
- [x] input Mach-O の entry / segment / stack metadata を output packaging に渡す。
- [x] fixture 専用 host trap JSON への依存を減らし、binary metadata から必要情報を得る。
- [x] malformed / unsupported Mach-O の blocker classification を artifact 生成でも維持する。
- [x] raw x86_64 bytes 入力ではなく、Mach-O executable image 全体から entry function / entry image を構成する。
- [x] relocation、import、symbol、unwind metadata を Normalized Program IR へ渡す最小 model を作る。
- [x] entry offset、code segment、const data、stdout request を binary metadata 由来で解決する。
- [x] pure Mach-O writer の offset / size / byte serialization 境界を実バイナリ入力経路から検証する。
- [x] output Mach-O の layout / serialization parity を公開仕様ベースで検証する。

### B7: Oracle / Regression 基盤

- [x] `clang -target x86_64-apple-macos...` で x86_64 テスト Mach-O を生成する。
- [x] x86_64 oracle runner を作る。
- [x] Rosetta 実行で `expected.json` を生成する。
- [x] Bara 変換結果を ARM64 native runner で実行し、`actual.json` を生成する。
- [x] `expected.json` と `actual.json` を比較する。
- [x] `compiled.ir.json`、`pcmap.json`、`fixups.json`、`helpers.json` を artifact metadata として出せるようにする。
- [x] state layout description、cache validation identity、helper requirements を artifact report に含める。
- [x] generated executable を実プロセスとして走らせる regression gate を追加する。
- [x] expected / actual に stdout、stderr、exit status、return value、artifact metadata を含める。
- [x] Rosetta black-box oracle 経路を clean-room ルール内で再検討する。
- [x] fixture shrink / failure classification / corpus update の運用を作る。
- [x] Haskell verifier 用 package / schema reader / small x86 semantics interpreter の導入可否を決める。
- [x] IR invariant、PC map invariant、fixup consistency、final state comparator を verifier で検査できるようにする。
  - [x] IR invariant を `validate_program` 経由で verifier report に接続する。
  - [x] Rust verifier report を作り、PC map が全 IR block start の source PC を保持していることを検査する。
  - [x] fixup consistency を verifier report で検査する。
  - [x] final state comparator を verifier report に接続する。
- [x] QuickCheck / Hedgehog 導入前の Rust deterministic 小ケース生成と failing case shrink を始める。
- [x] CI で quick tests と host-specific native artifact tests を分ける。
- [x] quick / oracle / nightly の CI lane を分け、失敗ケースを corpus に保存する。
- [x] wrong register / flags / memory / branch target / call return / external call を failure classification として扱う。

次の本流目標は、PE / Wine 接続へ進む前に、fixture 専用経路ではない
実 x86_64 macOS アプリを Bara の user-space runtime で起動できる
状態へ到達すること。その後に x86 32-bit アプリ対応を置く。32-bit
対応は blocker なら飛ばして B10 の PE / Wine 接続前段へ進んでよいが、
互換性の論点を先に発見できるため、B10 より前に処理するのが望ましい。
旧 B10 の user-space runtime architecture は、B8/B9 の設計制約として扱う。

### B8: 実 x86_64 macOS アプリ起動

- [x] 最初に起動対象とする実 x86_64 macOS アプリの scope と成功条件を定義する。
  詳細は [B8 GUI Hello World 起動スコープ](docs/b8-gui-hello-world-scope.md)
  に置く。
- [x] self-authored single-binary GUI Hello World source を追加し、x86_64
  Mach-O executable としてビルドできる host-specific fixture にする。
- [x] GUI Hello World fixture を Rosetta black-box oracle で実行し、
  `expected.json` と launch metadata の初期 schema を固定する。
- [x] Bara 側の GUI Hello World 起動 attempt を `actual.json` / launch
  report / blocker classification として保存できる CLI 境界を作る。
- [x] GUI Hello World の initial blocker を unsupported import /
  unsupported loader feature / unsupported ObjC runtime boundary のどれかに
  安定分類する。
- [x] raw function fixture ではなく、x86_64 Mach-O executable image 全体を入力として扱う。
- [x] entry、segments、sections、imports、relocations、必要な loader metadata を public Mach-O 仕様ベースで model 化する。
  - [x] actual launch report に public Mach-O probe 由来の loader metadata
    （file type、load command table、recognized entry points / segments、
    executable image conversion blocker）を保存する。
  - [x] sections metadata を public `LC_SEGMENT_64` / section table から model 化する。
  - [x] imports metadata を public dynamic-link load commands から model 化する。
  - [x] relocations / rebases / binds に必要な loader metadata の扱いを分ける。
- [x] user-space loader / runtime が image mapping、entry trampoline、stack / argv / envp、helper boundary を準備する責務を分ける。
- [x] kernel extension、private kernel hook、private dyld behavior を前提にしない。
- [x] loader、translation cache、runtime helper、artifact cache を user-space process 内に閉じる。
- [x] executable memory は public OS API (`mmap` / `mprotect` など) 経由に限定する。
- [x] JIT / AOT / fallback interpreter を同じ user-space runtime 境界から選べる設計にする。
- [x] syscall / OS API bridge は helper boundary として明示し、core IR / emit へ混ぜない。
- [x] source ISA mode、address size、operand size、stack width を型で表せるようにし、B9 の x86_32 対応の妨げにしない。
- [x] register model は `rax` だけでなく、部分レジスタへ拡張できる形にする。
- [x] signal / exception / thread / TLS / memory protection は user-space loader model として段階的に扱う。
- [x] macOS code signing / W^X / hardened runtime 制約は public API と documented behavior の範囲で整理する。
- [x] unsupported instruction、unsupported import、unsupported loader feature を stable blocker classification として report する。
- [x] unimplemented instruction、unknown indirect target、unsupported loader feature の fallback 方針を決める。
- [x] 小さな interpreter fallback または外部 fallback engine 接続を検討する。
- [x] 起動結果を stdout、stderr、exit status、launch metadata、blocker classification を含む stable JSON report にする。
- [x] B8 の短期ターゲットと reviewable slice を再定義し、一般アプリ対応を
  1 つの完了条件にしない切り方にする。
- [x] B8-H1: Rosetta expected / Bara actual の feedback cycle を作り、
  AppKit lifecycle helper capability で deterministic lifecycle event を一致させる。
  - [x] Rosetta expected と Bara actual / launch report を同じ feedback report に束ね、現状の blocker と次の修正対象を stable JSON で出す。
  - [x] feedback report の `unsupported_loader_feature` に対して、public Mach-O loader metadata から最初の user-space loader 実行計画を作る。
  - [x] AppKit import / Objective-C runtime boundary を helper boundary または明示 blocker として進め、expected / actual 差分を縮める。
    - [x] public AppKit import / Objective-C runtime helper boundary plan と explicit `unsupported_import` next blocker を feedback report に出す。
    - [x] AppKit import helper capability または explicit blocker promotion を actual result に接続し、expected / actual 差分を縮める。
    - [x] Objective-C runtime boundary を helper boundary または明示 blocker として actual result に接続する。
  - [x] arm64 macOS 上で self-authored x86_64 GUI Hello World が Bara 経由で起動し、Rosetta expected / Bara actual 比較を通す。
    - [x] Objective-C runtime / AppKit lifecycle helper capability contract を domain model と actual / feedback report schema に追加する。
    - [x] Objective-C runtime / AppKit lifecycle helper capability の host execution を actual path に接続し、current blocker を解除する。
    - [x] GUI Hello World actual result を Rosetta expected と一致させる。
- [x] B8-G1: GUI window に Hello World のフォント描画を行う最小アプリを、
  実際の変換レイヤーを通して GUI 上で確認できるようにする。
  - [x] 自動 expected / actual 判定と手動の GUI 目視確認を分け、CI では
    stable JSON、開発者確認では一定時間表示される window を使う scope を固定する。
  - [x] x86_64 entry path が decode / lift / emit / runtime execution を通ったことを
    launch report に保存し、host AppKit helper 単独実行と区別できるようにする。
  - [x] translated code から AppKit lifecycle helper capability を呼び出す最小
    helper ABI または host trap contract を定義する。
  - [x] AppKit helper は public AppKit API だけで window と `hello world` label を
    実描画し、手動確認 mode ではすぐ閉じないようにする。
  - [x] Rosetta expected と Bara actual の stable comparison を保ったまま、
    GUI 上の Hello World 描画を開発者が確認できる CLI 手順を追加する。
- [ ] B8-G2 以降: self-authored fixture から少しずつ一般の x86_64 macOS
  GUI application に近づける。一般アプリ対応そのものは長期拡張であり、
  1 つの PR 完了条件にはしない。

B8-G2 以降の長期ゴール:

- Rosetta 確認済みの self-authored x86_64 Mach-O GUI executable を入力とし、
  B8-G1 専用 sentinel / host trap ではなく、実 `LC_MAIN` entry から
  decode / lift / emit / runtime execution を開始する。
- 実行を進める途中で出る未対応命令、未対応 import、未対応 relocation / bind、
  unknown indirect target、Objective-C / AppKit boundary を stable blocker
  classification として report し、1 つずつ helper boundary または実装済み
  translation に昇格する。
- AppKit / Objective-C runtime の内部構造や private dyld behavior は実装根拠に
  しない。public Mach-O metadata、public ObjC runtime / AppKit API、
  self-authored fixture、Rosetta black-box observable result だけを根拠にする。
- B8 の reviewable 到達点は「任意 GUI app が動く」ではなく、各 slice で
  `expected.json` / `actual.json` / launch report / blocker report が安定し、
  次の unsupported boundary が明確になることとする。

PR 提出地点の運用:

- B8-D0 以降は、実装計画の中に `PR Gate` を明示する。
- `/advance-pr` は `TODO.md` の最初の未完了 `PR Gate` だけを対象にし、完了条件を
  満たしたら branch を push して draft PR を開き、次の `PR Gate` へは進まない。
- `PR Gate` は branch 名、完了条件、PR に含めない作業、検証、停止条件を持つ。
- debug bundle が利用可能になった後は、次の PR Gate を debug bundle の blocker
  report から選ぶ。計画にない作業が必要になった場合は、先に TODO へ PR Gate を
  追加または修正する。

実行計画:

- [x] B8-D0: 一般アプリ化に入る前の debug bundle foundation を作る。
  - [x] B8 GUI input binary から、probe、entry extraction、decode、lift、emit、
    runtime attempt、loader plan、helper request、blocker を 1 directory に保存する
    debug bundle schema を定義する。
  - [x] `target/b8-debug/<case_id>/` に `input.probe.json`、`entry.bytes.bin`、
    `entry.bytes.json`、`decode.report.json`、`lift.ir.json`、`emit.report.json`、
    `pcmap.json`、`fixups.json`、`helpers.json`、`loader.plan.json`、
    `runtime-attempt.json`、`blocker.json`、`repro.sh` を保存する CLI 境界を作る。
  - [x] debug bundle は clean-room 境界を守り、Rosetta から得る情報は
    public process observation と expected JSON だけに限定する。
  - [x] debug bundle の保存は通常の actual / launch report と分け、失敗分析用の
    sidecar として扱う。core decode / lift / emit は I/O を持たず、debug 情報は
    report value または明示 collector から作る。

B8-D0 で固定した debug bundle は、実 `LC_MAIN` first-block translation ではなく、
B8-G1 の translated host trap entry を entry bytes / decode / lift / emit /
runtime attempt の foundation として保存する。実 `LC_MAIN` entryoff と executable
segment metadata から entry bytes を切り出す作業は、次の B8-G2 PR Gate に残す。

#### PR Gate: B8-D0 Debug Bundle Foundation

branch: `task/b8-d0-debug-bundle`

完了条件:

- `target/b8-debug/<case_id>/` の directory layout が固定されている。
- `input.probe.json`、`entry.bytes.bin`、`entry.bytes.json`、
  `decode.report.json`、`lift.ir.json`、`emit.report.json`、`pcmap.json`、
  `fixups.json`、`helpers.json`、`loader.plan.json`、`runtime-attempt.json`、
  `blocker.json`、`repro.sh` が CLI 境界から保存される。
- debug bundle は通常の actual / launch / feedback report を置き換えず、
  failure analysis 用 sidecar として扱われる。
- core decode / lift / emit / validation に I/O が混ざっていない。
- regression test または host-specific fixture test が追加されている。
- `docs/progress.md` の現在の作業スナップショットが B8-D0 完了状態へ更新されている。

PR に含めない:

- 新しい x86_64 命令実装。
- 実 `LC_MAIN` entry からの first-block translation attempt。
- import / Objective-C / AppKit helper bridge の拡張。
- JIT / on-demand translation / fallback interpreter の実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止し、B8-G2 へは自動で進まない。

B8-D0 以降でぶつかりそうな大きな壁:

1. Debug bundle / failure reproduction。実 Mach-O entry に入る前に、失敗時の
   input、entry bytes、decode / lift / emit、loader plan、runtime attempt、
   blocker、再現 command を保存できないと、後続の unsupported boundary を
   安定して潰せない。
2. 実 Mach-O entry extraction と first-block translation。B8-G1 専用 sentinel から
   離れ、`LC_MAIN` entryoff と executable segment metadata から実 x86_64 bytes を
   切り出して処理する必要がある。
3. x86_64 ISA coverage。compiler output の prologue / epilogue、RIP-relative
   addressing、memory operands、`lea`、call / jump stubs、flags、SSE などが順に
   blocker になる。
4. Mach-O image mapping と relocation / rebase / bind。`__TEXT` / `__DATA` /
   `__LINKEDIT`、slide、page protection、rebase、bind、lazy bind を public
   metadata から runtime image へ反映する必要がある。
5. Dynamic library / import resolution。`LC_LOAD_DYLIB`、symbol stubs、public system
   framework imports、libc / AppKit / Objective-C runtime symbol を helper
   boundary へ接続する必要がある。
6. Calling convention / helper marshaling。x86_64 macOS ABI の register arguments、
   stack alignment、return value、variadic call、struct return、ObjC message send
   ABI を helper request / return value と対応づける必要がある。
7. Objective-C runtime / AppKit boundary。`objc_msgSend`、class / selector lookup、
   autorelease pool、main run loop、window / view lifecycle、callbacks into translated
   code が大きな境界になる。
8. Process state。initial stack、argv / envp、heap / malloc、TLS、file descriptors、
   current working directory、signals / exceptions、initial thread を user-space
   runtime metadata と helper boundary で扱う必要がある。
9. Indirect control flow と translation cache。function pointers、ObjC IMPs、callbacks、
   lazy stubs、unknown indirect target が増えると、AOT だけでは到達先を事前確定
   しにくくなり、on-demand translation / JIT / fallback interpreter の必要度が上がる。
10. macOS 実行制約。executable memory、W^X、code signing、hardened runtime、
    framework loading、bundle / resource を public API と documented behavior の
    範囲で扱う必要がある。
11. `.app` bundle / resource。single executable の限界が blocker になった時点で、
    Info.plist、bundle identifier、resources、assets、nib/storyboard 相当を scope 化する。

当面は AOT 的 pipeline を主軸にし、JIT は最初から実装前提にしない。JIT または
on-demand translation は、unknown indirect target、callback、lazy binding、
runtime-generated target が stable blocker として頻出し始めた段階で、必要な範囲から
導入する。
- [x] B8-G2: 実 Mach-O entry からの first-block translation report を作る。
  - [x] B8-G1 専用 `0f0b4238473131c0c3` entry とは別に、入力 Mach-O の
    public `LC_MAIN` entryoff と executable segment metadata から実 entry bytes を
    切り出す。
  - [x] 実 entry bytes に対して decode / lift / emit / runtime attempt を行い、
    最初の unsupported instruction / terminator / helper boundary を stable
    JSON report に保存する。
  - [x] GUI 表示は要求せず、実 x86_64 entry に到達した事実、処理した PC range、
    次 blocker、B8-G1 host trap path との差分を launch report に保存する。
  - [x] B8-G2 の debug bundle は `blocker.json` で
    `unsupported_instruction` / `DecodeUnsupportedOpcode { opcode: 85 }` を返す。
    これは x86_64 `push rbp` prologue であり、B8-G3 の最初の ISA slice として扱う。

#### PR Gate: B8-G2 Real LC_MAIN First-Block Report

branch: `task/b8-g2-entry-first-block`

完了条件:

- [x] B8-G1 専用 sentinel / host trap entry とは別に、public `LC_MAIN` entryoff と
  executable segment metadata から実 entry bytes を切り出している。
- [x] 実 entry bytes の decode / lift / emit / runtime attempt が debug bundle と
  launch report に保存される。
- [x] 最初の unsupported instruction / terminator / helper boundary が stable
  `blocker.json` と launch report に保存される。
- [x] GUI 表示を完了条件にせず、処理した source PC range と B8-G1 host trap path
  との差分が report される。
- [x] B8-D0 の debug bundle 出力を使い、次の B8-G3 以降で潰すべき blocker が
  具体的に分かる。

PR に含めない:

- blocker として見つかった新規命令群の広範な実装。
- Mach-O image mapping / rebase / bind の本実装。
- import / Objective-C / AppKit helper bridge の一般化。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止し、B8-G3 へは自動で進まない。

- [x] B8-G3: self-authored GUI fixture の compiler output に必要な x86_64
  ISA subset を、最初の blocker slice から corpus-driven に拡張する。
  - [x] B8-G2 の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 85 }`
    (`push rbp`) を最初の ISA blocker として focused fixture に固定する。
  - [x] `push rbp` (`0x55`) の decode / lift / emit に必要な最小範囲だけを
    public ISA 仕様に基づいて追加する。
  - [x] B8-G3 の debug bundle は `push_rbp` を通過し、次 blocker として
    `DecodeUnsupportedOpcode { opcode: 72 }` (`48 89 e5`, 次の prologue 命令) を
    stable `UnsupportedInstruction` として返す。

#### PR Gate: B8-G3 First ISA Blocker Slice

branch: `task/b8-g3-first-isa-blocker`

完了条件:

- [x] B8-G2 の debug bundle / blocker report から、最初に潰す x86_64 ISA blocker を
  1 つ選んでいる。
- [x] 選んだ blocker の最小 bytes が regression corpus または focused fixture として
  保存されている。
- [x] decode / lift / emit / validation / runtime attempt のうち、その blocker に必要な
  最小範囲だけを実装している。
- [x] 同じ blocker が `UnsupportedInstruction` ではなく、次の blocker まで進むことを
  debug bundle または launch report で確認できる。

PR に含めない:

- B8-G3 全体の ISA coverage を一括で増やす作業。
- loader mapping、import resolution、Objective-C / AppKit bridge の本実装。
- 選んだ blocker と理由が違う命令・ABI・runtime service の便乗実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。残りの ISA blocker は
  debug bundle の結果を見て次の `PR Gate` として追加する。

- [x] B8-G3b: 実 prologue の `REX.W mov rbp,rsp` slice を追加する。
  - [x] B8-G3 の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 89 e5`, `mov rbp,rsp`) を次の ISA blocker として focused fixture に固定する。
  - [x] `mov rbp,rsp` に必要な register model、decode、lift、emit planning を
    最小範囲で追加する。
  - [x] debug bundle が同じ blocker を越えて次の unsupported boundary を返すことを
    確認する。
  - [x] B8-G3b の debug bundle は `mov_rbp_rsp` を通過し、次 blocker として
    `DecodeUnsupportedOpcode { opcode: 65 }` (`41 57`, `push r15`) を返す。

#### PR Gate: B8-G3b REX Mov RBP/RSP Prologue Slice

branch: `task/b8-g3b-mov-rbp-rsp`

完了条件:

- [x] B8-G3 の debug bundle / blocker report から、次に潰す x86_64 ISA blocker として
  `48 89 e5` (`mov rbp,rsp`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] decode / lift / emit のうち、その blocker に必要な最小範囲だけを実装している。
- [x] debug bundle または launch report で `opcode 72` blocker を越えて次の blocker が
  stable に report される。

PR に含めない:

- prologue / epilogue 全体、RIP-relative addressing、`lea`、memory operands、
  call/jump stubs の一括実装。
- loader mapping、import resolution、Objective-C / AppKit bridge の本実装。
- RSP/RBP 以外の register move の便乗一般化。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。残りの ISA blocker は
  debug bundle の結果を見て次の `PR Gate` として追加する。

- [x] B8-G3c: 実 prologue の `REX.B push r15` slice を追加する。
  - [x] B8-G3b の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 65 }`
    (`41 57`, `push r15`) を次の ISA blocker として focused fixture に固定する。
  - [x] `push r15` に必要な register model、decode、lift、emit planning を
    最小範囲で追加する。
  - [x] debug bundle が同じ blocker を越えて次の unsupported boundary を返すことを
    確認する。
  - [x] B8-G3c の debug bundle は `push_r15` を通過し、次 blocker として
    `DecodeUnsupportedOpcode { opcode: 65 }` (`41 56`, `push r14`) を返す。

#### PR Gate: B8-G3c REX Push R15 Prologue Slice

branch: `task/b8-g3c-push-r15`

完了条件:

- [x] B8-G3b の debug bundle / blocker report から、次に潰す x86_64 ISA blocker として
  `41 57` (`push r15`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] decode / lift / emit のうち、その blocker に必要な最小範囲だけを実装している。
- [x] debug bundle または launch report で `opcode 65` blocker を越えて次の blocker が
  stable に report される。

PR に含めない:

- prologue / epilogue 全体、RIP-relative addressing、`lea`、memory operands、
  call/jump stubs の一括実装。
- loader mapping、import resolution、Objective-C / AppKit bridge の本実装。
- R15 以外の extended register 命令の便乗一般化。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。残りの ISA blocker は
  debug bundle の結果を見て次の `PR Gate` として追加する。

- [x] B8-G3d: 実 prologue の `REX.B push r14` slice を追加する。
  - [x] B8-G3c の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 65 }`
    (`41 56`, `push r14`) を次の ISA blocker として focused fixture に固定する。
  - [x] `push r14` に必要な register model、decode、lift、emit planning を
    最小範囲で追加する。
  - [x] debug bundle が同じ blocker を越えて次の unsupported boundary を返すことを
    確認する。
  - [x] B8-G3d の debug bundle は `push_r14` を通過し、次 blocker として
    `DecodeUnsupportedOpcode { opcode: 83 }` (`53`, `push rbx`) を返す。

#### PR Gate: B8-G3d REX Push R14 Prologue Slice

branch: `task/b8-g3d-push-r14`

完了条件:

- [x] B8-G3c の debug bundle / blocker report から、次に潰す x86_64 ISA blocker として
  `41 56` (`push r14`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] decode / lift / emit のうち、その blocker に必要な最小範囲だけを実装している。
- [x] debug bundle または launch report で `opcode 65` blocker を越えて次の blocker が
  stable に report される。

PR に含めない:

- prologue / epilogue 全体、RIP-relative addressing、`lea`、memory operands、
  call/jump stubs の一括実装。
- loader mapping、import resolution、Objective-C / AppKit bridge の本実装。
- R14 以外の extended register 命令の便乗一般化。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。残りの ISA blocker は
  debug bundle の結果を見て次の `PR Gate` として追加する。

- [x] B8-G3e: opcode-only blocker batch を追加する。
  - [x] B8-G3d の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 83 }`
    (`53`, `push rbx`) を batch の最初の ISA blocker として focused fixture に固定する。
  - [x] debug bundle が次に返す blocker が opcode 追加だけで解ける間は、同じ PR で
    連続して decode / lift / emit / JSON projection を追加する。
  - [x] opcode 追加だけで解けない loader / import / helper ABI / runtime service /
    broad refactor が必要になったら、そこで batch を止めて次の PR Gate として記録する。
  - [x] B8-G3e batch は `push_rbx` (`53`) と `mov_rbx_rax` (`48 89 c3`) を通過し、
    `DecodeUnsupportedOpcode { opcode: 72 }` (`48 8b 05 disp32`, RIP-relative load)
    で停止する。

#### PR Gate: B8-G3e Opcode-Only Blocker Batch

branch: `task/b8-g3e-opcode-batch`

完了条件:

- [x] B8-G3d の debug bundle / blocker report から、次に潰す x86_64 ISA blocker として
  `53` (`push rbx`) を選んでいる。
- [x] 各追加 opcode の最小 bytes が focused fixture または debug bundle regression として
  保存されている。
- [x] 各 step は decode / lift / emit / artifact JSON projection の最小追加に収まっている。
- [x] debug bundle または launch report で、batch 最後の opcode blocker を越えた次の
  blocker が stable に report される。
- [x] batch 停止理由を TODO / progress / design note に記録している。

PR に含めない:

- prologue / epilogue 全体、RIP-relative addressing、`lea`、memory operands、
  call/jump stubs の一括実装。
- loader mapping、import resolution、Objective-C / AppKit bridge の本実装。
- opcode 追加だけで説明できない loader / import / helper ABI / runtime service の実装。
- opcode 追加を超える broad refactor や register allocator / full ABI model の導入。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- opcode-only blocker が続く間は同じ branch で作業を続ける。opcode 追加だけで
  進めなくなったら commit / push / draft PR 作成で停止し、次の non-opcode 境界または
  次 batch を `PR Gate` として追加する。

- [x] B8-G3f: RIP-relative `mov rax,[rip+disp32]` load slice を追加する。
  - [x] B8-G3e の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 8b 05 ff 19 00 00`) を RIP-relative memory load blocker として focused fixture
    に固定する。
  - [x] RIP-relative source address、read width、image metadata / mapped bytes との境界を
    typed model として最小範囲で追加する。
  - [x] debug bundle が同じ blocker を越えて次の unsupported boundary を返すことを
    確認する。

#### PR Gate: B8-G3f RIP-Relative MOV Load Slice

branch: `task/b8-g3f-rip-relative-mov-load`

完了条件:

- [x] B8-G3e の debug bundle / blocker report から、次に潰す boundary として
  `48 8b 05 disp32` (`mov rax, qword ptr [rip+disp32]`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] RIP-relative address calculation と memory load operand を decode / lift / emit の
  最小範囲で表現している。
- [x] debug bundle または launch report で `48 8b 05` blocker を越えて次の blocker が
  stable に report される。

PR に含めない:

- loader mapping、relocation / rebase / bind 適用、import resolution の本実装。
- general memory subsystem、full x86 addressing modes、RIP-relative store、
  arbitrary-width memory operations の一括実装。
- Objective-C / AppKit bridge や helper ABI の本実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の blocker は debug bundle の
  結果を見て次の `PR Gate` として追加する。

- [x] B8-G3g: register-indirect `mov rdx,[rax]` load boundary を追加する。
  - [x] B8-G3f の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 8b 10`) を register-indirect memory load blocker として focused fixture に固定する。
  - [x] `rdx` register model、register-indirect 64-bit load operand、mapped image /
    runtime memory boundary のどこまでをこの slice で扱えるかを typed model として
    最小範囲で決める。
  - [x] loader mapping、rebase / bind、import resolution が必要な場合は silent fallback
    せず stable blocker として report する。

#### PR Gate: B8-G3g RAX-Indirect MOV RDX Load Boundary

branch: `task/b8-g3g-rax-indirect-mov-load`

完了条件:

- [x] B8-G3f の debug bundle / blocker report から、次に潰す boundary として
  `48 8b 10` (`mov rdx, qword ptr [rax]`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] `rdx` register model と register-indirect 64-bit memory load operand を、実行可能な
  最小範囲または stable loader/memory blocker として表現している。
- [x] debug bundle または launch report で `48 8b 10` blocker を越えるか、必要な
  loader / mapped memory boundary が stable に report される。

PR に含めない:

- full x86 addressing modes、store、arbitrary-width memory operations の一括実装。
- relocation / rebase / bind、import resolution、Objective-C / AppKit bridge の本実装。
- 汎用 register allocation や JIT/on-demand translation cache の本実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の blocker は debug bundle の
  結果を見て次の `PR Gate` として追加する。

- [x] B8-G3h: RIP-relative `lea rdi,[rip+disp32]` address materialization boundary を追加する。
  - [x] B8-G3g の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 8d 3d b3 10 00 00`) を RIP-relative LEA blocker として focused fixture に固定する。
  - [x] `rdi` destination の RIP-relative address materialization を、memory read ではない
    typed address operand または最小 IR op として表現する。
  - [x] general LEA addressing modes が必要な場合は silent fallback せず stable blocker として
    report する。

#### PR Gate: B8-G3h RIP-Relative LEA RDI Address Boundary

branch: `task/b8-g3h-rip-relative-lea-rdi`

完了条件:

- [x] B8-G3g の debug bundle / blocker report から、次に潰す boundary として
  `48 8d 3d disp32` (`lea rdi, [rip+disp32]`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] memory load と区別して、RIP-relative effective address materialization を decode /
  lift / emit の最小範囲または stable blocker として表現している。
- [x] debug bundle または launch report で `48 8d 3d` blocker を越えるか、次に必要な
  ISA / loader / metadata boundary が stable に report される。

PR に含めない:

- full LEA addressing modes、scaled-index addressing、arbitrary destination registers の
  一括実装。
- relocation / rebase / bind、import resolution、Objective-C / AppKit bridge の本実装。
- 汎用 register allocation や JIT/on-demand translation cache の本実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の blocker は debug bundle の
  結果を見て次の `PR Gate` として追加する。

- [x] B8-G3i: RIP-relative `lea rsi,[rip+disp32]` address materialization boundary を追加する。
  - [x] B8-G3h の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 8d 35 b6 10 00 00`) を RIP-relative LEA RSI blocker として focused fixture に固定する。
  - [x] `rsi` register model と RIP-relative address materialization を、memory read ではない
    typed address operand として表現する。
  - [x] general LEA addressing modes や arbitrary destination registers が必要な場合は
    silent fallback せず stable blocker として report する。

#### PR Gate: B8-G3i RIP-Relative LEA RSI Address Boundary

branch: `task/b8-g3i-rip-relative-lea-rsi`

完了条件:

- [x] B8-G3h の debug bundle / blocker report から、次に潰す boundary として
  `48 8d 35 disp32` (`lea rsi, [rip+disp32]`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] `rsi` register model と、memory load ではない RIP-relative effective address
  materialization を decode / lift / emit の最小範囲または stable blocker として表現している。
- [x] debug bundle または launch report で `48 8d 35` blocker を越えるか、次に必要な
  ISA / loader / metadata boundary が stable に report される。

PR に含めない:

- full LEA addressing modes、scaled-index addressing、arbitrary destination registers の
  一括実装。
- relocation / rebase / bind、import resolution、Objective-C / AppKit bridge の本実装。
- 汎用 register allocation や JIT/on-demand translation cache の本実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の blocker は debug bundle の
  結果を見て次の `PR Gate` として追加する。

- [x] B8-G3j: RIP-relative `mov rdi,qword ptr [rip+disp32]` load boundary を追加する。
  - [x] B8-G3i の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 8b 3d 22 3b 00 00`) を RIP-relative MOV RDI load blocker として focused fixture に固定する。
  - [x] `rdi` destination の RIP-relative 64-bit memory load を、`lea` の address
    materialization と区別して typed memory operand として表現する。
  - [x] arbitrary destination registers、relocation / rebase / bind 適用、import resolution が
    必要な場合は silent fallback せず stable blocker として report する。

#### PR Gate: B8-G3j RIP-Relative MOV RDI Load Boundary

branch: `task/b8-g3j-rip-relative-mov-rdi-load`

完了条件:

- [x] B8-G3i の debug bundle / blocker report から、次に潰す boundary として
  `48 8b 3d disp32` (`mov rdi, qword ptr [rip+disp32]`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] `rdi` destination の RIP-relative 64-bit memory load を decode / lift / emit の
  最小範囲または stable blocker として表現している。
- [x] debug bundle または launch report で `48 8b 3d` blocker を越えるか、次に必要な
  ISA / loader / metadata boundary が stable に report される。

PR に含めない:

- REX.W MOV の全 ModRM 形式や arbitrary destination registers の一括実装。
- relocation / rebase / bind、import resolution、Objective-C / AppKit bridge の本実装。
- 汎用 register allocation や JIT/on-demand translation cache の本実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の blocker は debug bundle の
  結果を見て次の `PR Gate` として追加する。

- [x] B8-G3k: 連続する RIP-relative MOV load boundary を次の non-load blocker まで追加する。
  - [x] B8-G3j の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 8b 35 eb 3a 00 00`) を RIP-relative MOV RSI load blocker として focused fixture に固定する。
  - [x] 同じ load 系が連続する場合は、debug bundle / entry bytes で確認できる範囲に限り
    次の RIP-relative MOV load blocker まで同じ PR で進める。
  - [x] `rsi` / `r14` destination の RIP-relative 64-bit memory load を、`lea` の address
    materialization と区別して typed memory operand として表現する。
  - [x] 次 blocker が indirect call など non-load になった時点で batch を止める。
  - [x] arbitrary destination registers、relocation / rebase / bind 適用、import resolution が
    必要な場合は silent fallback せず stable blocker として report する。

#### PR Gate: B8-G3k RIP-Relative MOV Load Batch Boundary

branch: `task/b8-g3k-rip-relative-load-batch`

完了条件:

- [x] B8-G3j の debug bundle / blocker report から、次に潰す boundary として
  `48 8b 35 disp32` (`mov rsi, qword ptr [rip+disp32]`) を選んでいる。
- [x] 選んだ `rsi` blocker の最小 bytes が focused fixture として保存されている。
- [x] `rsi` destination の RIP-relative 64-bit memory load を decode / lift / emit の
  最小範囲または stable blocker として表現している。
- [x] `rsi` load を越えた次が `4c 8b 35 disp32` (`mov r14, qword ptr [rip+disp32]`) である場合、
  同じ RIP-relative MOV load batch として focused fixture と decode / lift / emit に追加している。
- [x] debug bundle または launch report で連続する load blocker を越え、次に必要な
  ISA / loader / metadata boundary が stable に report される。

PR に含めない:

- REX.W MOV の全 ModRM 形式や arbitrary destination registers の一括実装。
- relocation / rebase / bind、import resolution、Objective-C / AppKit bridge の本実装。
- 汎用 register allocation や JIT/on-demand translation cache の本実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の blocker は debug bundle の
  結果を見て次の `PR Gate` として追加する。

- [x] B8-G3l: indirect `call r14` boundary を追加する。
  - [x] B8-G3k の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 65 }`
    (`41 ff d6`) を indirect call blocker として focused fixture に固定する。
  - [x] `call r14` を direct call と混同せず、unknown indirect target / helper boundary /
    unsupported terminator のどれで扱うかを stable に表現する。
  - [x] indirect control flow や translation cache が必要な場合は silent fallback せず
    stable blocker として report する。

#### PR Gate: B8-G3l Indirect CALL R14 Boundary

branch: `task/b8-g3l-indirect-call-r14`

完了条件:

- [x] B8-G3k の debug bundle / blocker report から、次に潰す boundary として
  `41 ff d6` (`call r14`) を選んでいる。
- [x] 選んだ blocker の最小 bytes が focused fixture として保存されている。
- [x] indirect call を direct call / RIP-relative load と混ぜず、decode / lift / emit の
  最小範囲または stable unsupported boundary として表現している。
- [x] debug bundle または launch report で `41 ff d6` blocker を越えるか、次に必要な
  ISA / loader / metadata boundary が stable に report される。

PR に含めない:

- arbitrary indirect call targets、translation cache、fallback JIT/interpreter の本実装。
- relocation / rebase / bind、import resolution、Objective-C / AppKit bridge の本実装。
- 汎用 register allocation の本実装。

検証:

- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の blocker は debug bundle の
  結果を見て次の `PR Gate` として追加する。

- [ ] B8-G4: user-space Mach-O image mapping と relocation / rebase / bind 適用を
  実行可能な loader step にする。
  - [ ] `LC_SEGMENT_64` file ranges から executable image / const data / data
    mapping を作り、entry PC と runtime address の関係を typed metadata にする。
  - [ ] public rebase / bind metadata を使い、import symbol identity と
    helper boundary request を解決する。private dyld behavior は使わない。
  - [x] B8-G4a: `LC_SEGMENT_64` file range から materialize する executable image を
    segment-relative offset ではなく Mach-O VM address space で map し、entry PC と
    mapped bytes の関係を debug bundle / program image metadata に保存する。
  - [x] B8-G4b: public rebase / bind / import metadata を使い、`call r14` の target
    identity と helper boundary request を stable blocker または解決済み import として
    report する。
  - [x] B8-G4c: public `LC_DYLD_CHAINED_FIXUPS` payload を decode し、現在の
    `call r14` target pointer load を import symbol identity へ近づける。

#### PR Gate: B8-G4a User-Space Mach-O VM Image Mapping

branch: `task/b8-g4-user-space-macho-image-mapping`

完了条件:

- [x] `MachOExecutableImagePlan` が selected `LC_SEGMENT_64` の file range だけでなく
  segment `vmaddr` と entry virtual address を typed metadata として持つ。
- [x] materialized `ExecutableImage` の code segment base と entry PC が
  segment-relative offset ではなく Mach-O VM address になる。
- [x] `ProgramImageMetadata` の mapped bytes / code / const-data range が同じ
  Mach-O VM address space を使う。
- [x] B8 debug bundle の `entry.bytes.json`、`decode.report.json`、`launch.report.json`、
  `loader.plan.json`、`blocker.json` が VM-addressed source PC / call site を保存する。
- [x] rebase / bind / import 解決は silent fallback せず、次の deferred loader/import
  boundary として report される。

PR に含めない:

- public rebase / bind opcode stream の適用本体。
- import symbol identity から helper request への解決本体。
- arbitrary indirect call targets、translation cache、fallback JIT/interpreter の本実装。
- `.app` bundle / resource、Objective-C / AppKit bridge の一般化。

検証:

- `nix develop -c cargo test -p bara-oracle mach_o_executable_image -- --nocapture`
- `nix develop -c cargo test -p bara-oracle entry_function_pipeline -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の B8-G4b は debug bundle の
  `relocation_binding` deferred report と `register_indirect_call` blocker を見て進める。

#### PR Gate: B8-G4b Public Chained Fixups Import Boundary

branch: `task/b8-g4b-public-bind-import-boundary`

完了条件:

- [x] B8 debug bundle の `loader.plan.json` が `call r14` を import boundary として
  report し、`target_register`、`call_site`、`return_to` を保存する。
- [x] `call r14` の直前にある `mov r14, qword ptr [rip+disp32]` を
  `target_pointer_load` として report し、resolved pointer address を保存する。
- [x] public Mach-O load command metadata から dylib import command、dyld info range、
  `LC_DYLD_CHAINED_FIXUPS` linkedit data range、symbol table count を
  `public_metadata` として保存する。
- [x] 現 fixture が `LC_DYLD_CHAINED_FIXUPS` を使っている場合は import identity を
  silent fallback せず、`helper_boundary_request` を
  `import_symbol_identity_unresolved` の stable blocker として保存する。
- [x] 次 action が public chained fixups import decoder であることを
  `decode_public_dyld_chained_fixups_imports` として report する。

PR に含めない:

- public `LC_DYLD_CHAINED_FIXUPS` payload decoder の本実装。
- chained fixups import table からの symbol identity 解決本体。
- helper boundary request の実行、Objective-C / AppKit helper bridge の一般化。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- manual `generate-b8-debug-bundle` で `loader.plan.json` の
  `import_boundary.status=blocked`、`target_pointer_load.address=4294979672`、
  `dyld_chained_fixups dataoff=24576 datasize=584` を確認する。
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の B8-G4c は
  `loader.plan.json` の `decode_public_dyld_chained_fixups_imports` blocker を見て進める。

#### PR Gate: B8-G4c Public Chained Fixups Import Decoder

branch: `task/b8-g4c-public-chained-fixups-import-decoder`

完了条件:

- [x] public `LC_DYLD_CHAINED_FIXUPS` payload の header / starts / imports のうち、
  現 fixture の `call r14` target pointer load に必要な最小範囲を decode する。
- [x] decoded chained fixups metadata を private dyld behavior に依存せず typed report
  として保存する。
- [x] `target_pointer_load.address=4294979672` が chained fixups import metadata で
  解決可能か、または不足している public metadata を stable blocker として report する。

PR に含めない:

- 全 Mach-O chained fixups opcode / bind target の網羅。
- import helper execution、Objective-C / AppKit helper bridge の一般化。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p bara-oracle chained_fixups -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は decoded import
  identity と helper boundary request の結果を見て追加または更新する。
- [x] B8-G5: import stub / external symbol call を汎用 helper request に接続する。
  - [x] symbol stubs、lazy bind 相当、`objc_msgSend`、public libc / AppKit symbol を
    core IR に直接埋め込まず、helper capability request と stable blocker に分ける。
  - [x] unsupported import は symbol identity、call site、argument model の不足理由を
    report する。

#### PR Gate: B8-G5 Import Helper Boundary Request

branch: `task/b8-g5-import-helper-boundary-request`

完了条件:

- [x] B8-G4c の decoded chained fixups result から、`call r14` target が
  `/usr/lib/libobjc.A.dylib` の `_objc_msgSend` import identity であることを
  helper boundary planning input として扱う。
- [x] import identity、call site、target register、return PC、必要な argument /
  return marshaling の不足理由を stable helper boundary request または blocker として
  `loader.plan.json` / launch report に保存する。
- [x] core IR / ARM64 emit に Objective-C runtime や AppKit 固有処理を混ぜない。

PR に含めない:

- `_objc_msgSend` の host execution。
- Objective-C runtime / AppKit helper bridge の一般化。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は helper boundary
  request の `x86_64_argument_marshaling_unimplemented` /
  `helper_return_marshaling_unimplemented` を受けて、B8-G5a として追加する。
- [x] B8-G5a: import helper call の x86_64 argument / return marshaling contract を
  定義する。
  - [x] B8-G5 の `helper_boundary_request.required_marshaling` から、
    x86_64 call argument source と `rax` return destination を helper boundary の
    typed contract として扱う。
  - [x] `_objc_msgSend` host execution はまだ行わず、selector / receiver /
    return value materialization が不足する場合は stable blocker として report する。
  - [x] core IR / ARM64 emit に Objective-C runtime や AppKit 固有処理を混ぜない。

#### PR Gate: B8-G5a Import Helper Marshaling Contract

branch: `task/b8-g5a-import-helper-marshaling-contract`

完了条件:

- [x] B8-G5 の helper request から、`x86_64_argument_marshaling_unimplemented` と
  `helper_return_marshaling_unimplemented` を次に潰す boundary として扱う。
- [x] x86_64 call arguments と `rax` return value の helper marshaling contract を
  stable report に保存する。
- [x] `_objc_msgSend` の host execution、Objective-C / AppKit bridge、arbitrary
  indirect call target execution は行わない。

PR に含めない:

- `_objc_msgSend` の host execution。
- Objective-C runtime / AppKit helper bridge の一般化。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は marshaling
  contract の `objc_receiver_materialization_unimplemented` /
  `objc_selector_materialization_unimplemented` /
  `helper_return_value_materialization_unimplemented` を受けて、B8-G5b〜G5e として
  1 つの PR Gate にまとめる。
- [x] B8-G5b〜B8-G5e: `_objc_msgSend` receiver / selector / return materialization
  boundary を一続きの helper boundary slice として定義する。
  - [x] B8-G5a の marshaling contract から、`rdi` receiver、`rsi` selector、
    `rax` return destination を次に必要な materialization boundary として扱う。
  - [x] current fixture の receiver / selector address を public mapped image metadata
    から materialize し、mapped raw qword を public chained fixups / rebase / bind metadata
    から解釈する。
  - [x] helper return value を実行結果として生成せず、x86_64 `rax` write-back plan と
    remaining blocker を stable report に保存する。
  - [x] `_objc_msgSend` host execution と Objective-C / AppKit helper bridge はまだ行わない。

#### PR Gate: B8-G5b-G5e ObjC Materialization And Return Boundary

branch: `task/b8-g5b-g5e-objc-materialization-boundary`

完了条件:

- [x] B8-G5a の `b8_import_helper_marshaling_contract_v0` から、receiver / selector /
  return destination materialization を次に潰す boundary として扱う。
- [x] `rdi` receiver と `rsi` selector の materialization source を `call r14` 直前の
  RIP-relative qword load として `loader.plan.json` / launch report に保存する。
- [x] public `LC_SEGMENT_64` file-backed mapped image metadata から、current fixture の
  receiver / selector qword load address を読めるようにする。
- [x] mapped raw qword を public chained fixups / rebase / bind metadata から解釈し、
  receiver identity と selector VM address を stable report に保存する。
- [x] B8-G5d 後に残る `helper_return_value_materialization_unimplemented` を、
  x86_64 `rax` return destination の stable write-back boundary として具体化する。
- [x] `_objc_msgSend` の host execution、Objective-C / AppKit bridge、arbitrary
  indirect call target execution は行わない。

PR に含めない:

- `_objc_msgSend` の host execution。
- Objective-C runtime / AppKit helper bridge の一般化。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p bara-oracle chained_fixups -- --nocapture`
- `nix develop -c cargo test -p bara-oracle maps_public_file_backed_segments_into_program_image_metadata -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は helper return
  materialization 後の blocker を見て B8-G6a ObjC Helper Execution Boundary として
  更新する。
- [x] B8-G6a: ObjC helper execution boundary を stable report に分離する。
  - [x] B8-G5e 後に残る `objc_helper_execution_unimplemented` を受けて、helper execution
    request の source import、receiver identity、selector VM address、return write-back
    boundary を 1 つの stable report として保存する。
  - [x] Objective-C runtime / AppKit API の host execution はまだ行わず、実行に必要な
    public helper capability と不足条件だけを分類する。

#### PR Gate: B8-G6a ObjC Helper Execution Boundary

branch: `task/b8-g6a-objc-helper-execution-boundary`

完了条件:

- [x] B8-G5e の `objc_helper_execution_unimplemented` を、ObjC helper execution request
  boundary として stable report に分離する。
- [x] `_objc_msgSend` import identity、receiver identity、selector VM address、return
  write-back boundary を helper execution request の input として保存する。
- [x] Objective-C runtime / AppKit helper の host execution、arbitrary indirect call
  target execution、translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- `_objc_msgSend` の host execution。
- Objective-C runtime / AppKit helper bridge の実行実装。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は B8-G6b ObjC Runtime
  Helper Bridge Contract として更新する。
- [x] B8-G6b: ObjC runtime message-send helper bridge contract を stable report にする。
  - [x] B8-G6a の `objc_runtime_message_send_helper` required capability と
    `objc_helper_execution_unimplemented` を受けて、public ObjC runtime helper bridge の
    input / output / error contract を stable report に追加する。
  - [x] `_objc_msgSend` host execution はまだ行わず、bridge 実行に必要な public API
    capability と不足条件だけを分類する。

#### PR Gate: B8-G6b ObjC Runtime Helper Bridge Contract

branch: `task/b8-g6b-objc-runtime-helper-bridge-contract`

完了条件:

- [x] B8-G6a の helper execution request が要求する
  `objc_runtime_message_send_helper` capability を、public Objective-C runtime helper
  bridge contract として stable report に分離する。
- [x] bridge contract は `_objc_msgSend` import identity、receiver identity、
  selector VM address、return write-back boundary、helper output / error classification
  を input / output contract として保存する。
- [x] Objective-C runtime / AppKit helper の host execution、arbitrary indirect call
  target execution、translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- `_objc_msgSend` の host execution。
- Objective-C runtime / AppKit helper bridge の実行実装。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は B8-G6c ObjC Runtime
  Helper Bridge Host Execution Slice として更新する。
- [x] B8-G6c: ObjC runtime message-send helper bridge の host execution slice を追加する。
  - [x] B8-G6b の `objc_runtime_helper_execution_unimplemented` を受けて、self-authored
    GUI fixture に必要な `_objc_msgSend` helper execution を public Objective-C runtime /
    AppKit API 境界として実行する。
  - [x] 実行結果は helper output / return write-back boundary に接続するが、arbitrary
    indirect call target execution、translation cache、fallback JIT/interpreter はまだ行わない。

#### PR Gate: B8-G6c ObjC Runtime Helper Bridge Host Execution Slice

branch: `task/b8-g6c-objc-runtime-helper-bridge-execution`

完了条件:

- [x] B8-G6b の bridge contract が残す
  `objc_runtime_helper_execution_unimplemented` を、self-authored fixture に必要な範囲の
  public Objective-C runtime / AppKit helper execution として扱う。
- [x] helper output を `objc_helper_return_value` として report し、既存の x86_64 `rax`
  return write-back boundary へ接続する。
- [x] host execution は public Objective-C runtime / AppKit API と self-authored fixture
  に限定し、private dyld behavior や Objective-C / AppKit 内部構造を実装根拠にしない。
- [x] arbitrary indirect call target execution、translation cache、fallback JIT/interpreter
  は行わない。

PR に含めない:

- 任意 Objective-C / AppKit application の一般実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- private Objective-C runtime、private AppKit、private dyld behavior への依存。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は helper execution result
  の blocker を見て B8-G6d または focused process-state / AppKit lifecycle slice として
  更新する。
- [x] B8-G6d: ObjC helper return continuation boundary を追加する。
  - [x] B8-G6c の `objc_helper_return_continuation_unimplemented` を受けて、
    `_objc_msgSend` helper output を x86_64 `rax` value として持ったまま `return_to`
    PC から継続する boundary を stable report にする。
  - [x] arbitrary indirect call target execution、translation cache、fallback JIT/interpreter
    はまだ行わず、継続対象 PC / register state / next blocker だけを明示する。

#### PR Gate: B8-G6d ObjC Helper Return Continuation Boundary

branch: `task/b8-g6d-objc-helper-return-continuation`

完了条件:

- [x] B8-G6c の helper execution result が残す
  `objc_helper_return_continuation_unimplemented` を、`call r14` の `return_to`
  PC から再開するための explicit continuation boundary として扱う。
- [x] continuation input は helper output 由来の `objc_helper_return_value` と
  x86_64 `rax` write-back value を stable report に保存する。
- [x] continuation は次に読むべき source PC / register state / blocker を report するだけにし、
  arbitrary indirect call target execution、translation cache、fallback JIT/interpreter は
  行わない。

PR に含めない:

- `return_to` 以降の一般命令実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation
  report の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6e Return-To Continuation Decode Boundary

branch: `task/b8-g6e-return-to-continuation-decode`

完了条件:

- [x] B8-G6d の `return_to_continuation_execution_unimplemented` を受けて、
  continuation boundary の `next_source_pc` から読むべき x86_64 continuation block を
  stable report にする。
- [x] continuation input は G6d が保存した x86_64 `rax` register state を保持し、
  次の decoded instruction / boundary / blocker と関連付ける。
- [x] `return_to` 以降の一般実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- `return_to` 以降の命令の一般実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation block
  の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6f Return-To Continuation R15 RIP-Relative Load Slice

branch: `task/b8-g6f-continuation-r15-rip-relative-load`

完了条件:

- [x] B8-G6e の `return_to_continuation_unsupported_instruction` を受けて、
  `return_to` continuation block 先頭の `4c 8b 3d ...` を x86_64
  `mov r15, qword ptr [rip+disp32]` として decode / lift / emit または stable boundary
  に進める。
- [x] continuation input の x86_64 `rax` register state を保持したまま、次の decoded
  instruction / boundary / blocker を stable report に保存する。
- [x] `return_to` 以降の一般実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- `return_to` 以降の一般実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation block
  の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6g Return-To Continuation R15-Indirect RDI Load Slice

branch: `task/b8-g6g-continuation-r15-indirect-rdi-load`

完了条件:

- [x] B8-G6f の `return_to_continuation_unsupported_instruction` を受けて、`return_to`
  continuation block の次 bytes `49 8b 3f` を x86_64
  `mov rdi, qword ptr [r15]` として decode / lift / emit または stable boundary に進める。
- [x] continuation input の x86_64 `rax` register state と、直前の
  `mov r15, qword ptr [rip+disp32]` で materialize した `r15` state を保持したまま、
  次の decoded instruction / boundary / blocker を stable report に保存する。
- [x] `return_to` 以降の一般実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- `return_to` 以降の一般実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation block
  の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6h Return-To Continuation NSApp Global Load Boundary

branch: `task/b8-g6h-continuation-nsapp-global-load`

完了条件:

- [x] B8-G6g の `return_to_continuation_import_global_load_unimplemented` を受けて、
  `mov rdi, qword ptr [r15]` の base `r15` が public chained fixups 上で AppKit
  `_NSApp` import に解決されることを source of truth として、`_NSApp` imported global
  pointee load を focused stable boundary または fixture-scoped helper として扱う。
- [x] continuation input の x86_64 `rax` register state と `r15` import identity を保持し、
  `rdi` materialization の available / blocked state、次の decoded instruction /
  boundary / blocker を stable report に保存する。
- [x] 一般的な imported global memory model、任意の dynamic library data symbol read、
  `return_to` 以降の一般実行、arbitrary indirect call target execution、translation cache、
  fallback JIT/interpreter は行わない。

PR に含めない:

- 一般的な imported global memory model。
- 任意の dynamic library data symbol read。
- `return_to` 以降の一般実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation block
  の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6i Return-To Continuation XOR EDX Zero Slice

branch: `task/b8-g6i-continuation-xor-edx-zero`

完了条件:

- [x] B8-G6h の `return_to_continuation_unsupported_instruction` を受けて、
  `return_to` continuation block の次 bytes `31 d2` を x86_64
  `xor edx, edx` として decode / lift / emit または stable boundary に進める。
- [x] 32-bit register zeroing semantics により `rdx` が 64-bit zero へ materialize
  されることを continuation report に保存し、次の decoded instruction / boundary /
  blocker を stable report に保存する。
- [x] `return_to` 以降の一般実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- `xor r32, r32` 全体の一般化。
- `return_to` 以降の一般実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation block
  の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6j Return-To Continuation Call R14 Boundary Planning

branch: `task/b8-g6j-continuation-call-r14-boundary`

完了条件:

- [x] B8-G6i の `return_to_continuation_execution_unimplemented` を受けて、
  `return_to` continuation block の decoded `call r14` at `4294973018` /
  `return_to=4294973021` を focused stable boundary として扱う。
- [x] continuation input の x86_64 `rax` state、`r15` AppKit `_NSApp` import identity、
  `rdi` `_NSApp` value、`rdx=0` zeroing state を保持し、`call r14` の target /
  argument / blocked state と次 blocker を stable report に保存する。
- [x] `return_to` 以降の一般実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- `return_to` 以降の一般実行。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。
- 一般的な continuation call execution engine。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation block
  の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6k Return-To Continuation setActivationPolicy Helper Boundary

branch: `task/b8-g6k-continuation-set-activation-policy-helper`

完了条件:

- [x] B8-G6j の `return_to_continuation_execution_unimplemented` を受けて、
  continuation call boundary の target `_objc_msgSend`、receiver `_NSApp`、
  selector `setActivationPolicy:`、argument `rdx=0` を focused Objective-C helper
  boundary として扱う。
- [x] `setActivationPolicy:` helper request / bridge contract / available-or-blocked state を
  stable report に保存し、次の decoded instruction / boundary / blocker を記録する。
- [x] `return_to` 以降の一般実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter、Objective-C runtime / AppKit lifecycle
  全体の一般化は行わない。

PR に含めない:

- 一般的な continuation call execution engine。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。
- `setActivationPolicy:` 以外の arbitrary Objective-C message send。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- 完了したら commit / push / draft PR 作成で停止する。次の gate は continuation block
  の next blocker を見て focused ISA / process-state / AppKit lifecycle slice として
  更新する。

#### PR Gate: B8-G6l Return-To Continuation setActivationPolicy Host Execution Slice

branch: `task/b8-g6l-continuation-set-activation-policy-host-execution`

実装 branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6k の `return_to_continuation_objc_helper_execution_unimplemented` を受けて、
  `_objc_msgSend(NSApp, setActivationPolicy:, 0)` だけを focused host execution slice
  として扱う。
- [x] public Objective-C runtime / AppKit API と self-authored B8 GUI fixture に基づき、
  helper execution result または stable blocked state を report に保存する。
- [x] helper execution 後の next source PC / next decoded boundary / next blocker を
  stable report に保存する。
- [x] `setActivationPolicy:` 以外の arbitrary Objective-C message send、
  return-to continuation の一般実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- 一般的な continuation call execution engine。
- arbitrary Objective-C message send の一般 bridge。
- arbitrary indirect call target execution、translation cache、fallback JIT/interpreter。
- Objective-C runtime / AppKit lifecycle 全体の一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として完了。次の gate は B8-G6m。

#### PR Gate: B8-G6m Return-To Continuation objc_alloc_init Return Value Register Copy Slice

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6l の `return_to_continuation_unsupported_instruction` を受けて、
  `4294973043` の `48 89 c2` / `mov rdx, rax` を focused x86_64 register-copy
  slice として decode / report する。
- [x] `mov rdx, rax` の source `rax` が直前の `_objc_alloc_init` `call rel32`
  return value であることを stable boundary として扱い、まだ helper 実行できない場合は
  `objc_alloc_init` return materialization blocker として分類する。
- [x] `mov rdx, rax` 後の `call r14` が
  `_objc_msgSend(NSApp, setDelegate:, delegate)` に進む事実または stable blocker を
  next boundary として保存する。
- [x] `objc_alloc_init` 全般、arbitrary class allocation、arbitrary register-copy execution、
  general call-rel32 helper execution、arbitrary Objective-C message send、
  translation cache、fallback JIT/interpreter は行わない。

PR に含めない:

- 一般的な register transfer execution engine。
- arbitrary call-rel32 import/helper execution。
- arbitrary Objective-C allocation / initialization bridge。
- general continuation execution、translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として完了。次の gate は B8-G6n。

#### PR Gate: B8-G6n Return-To Continuation call_rel32 objc_alloc_init Helper Boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6m の
  `return_to_continuation_call_rel32_return_value_materialization_unimplemented` を受けて、
  `call_rel32` at `4294973028` / target `4294973108` / return_to `4294973033` を
  focused helper boundary として保存する。
- [x] public Mach-O stub / symbol / import metadata から `_objc_alloc_init` identity を
  解決できる場合は保存し、まだ解決できない場合は stable unresolved-stub blocker として
  分類する。
- [x] `objc_alloc_init` return value が `rax` に入り、`mov rdx, rax` によって
  `setDelegate:` argument へ渡る dataflow を helper return materialization boundary として
  保存する。
- [x] arbitrary call-rel32 helper execution、arbitrary Objective-C allocation /
  initialization、general dynamic symbol resolver、translation cache、fallback
  JIT/interpreter は行わない。

PR に含めない:

- 一般的な call-rel32 execution engine。
- arbitrary Objective-C allocation / initialization bridge。
- dynamic linker / dyld stub binding の一般実装。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6n は完了し、次の gate は B8-G6o。

#### PR Gate: B8-G6o Return-To Continuation objc_alloc_init Helper Execution Boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6n の
  `return_to_continuation_call_rel32_helper_execution_unimplemented` を受けて、
  `_objc_alloc_init` helper execution request を focused boundary として保存する。
- [x] `call_rel32` at `4294973028` の x86_64 argument `rdi` を public Mach-O mapped
  image / chained fixup metadata から materialize し、class identity を解決できる場合は
  保存する。解決できない場合は stable class-argument blocker として分類する。
- [x] `_objc_alloc_init` の return value が `rax` に入り、後続の `mov rdx, rax` で
  `setDelegate:` argument へ渡る execution/dataflow 境界を更新する。
- [x] arbitrary Objective-C class allocation / initialization bridge、arbitrary call-rel32
  execution、general dynamic symbol resolver、translation cache、fallback JIT/interpreter は
  行わない。

PR に含めない:

- 一般的な call-rel32 execution engine。
- arbitrary Objective-C allocation / initialization bridge。
- dynamic linker / dyld stub binding の一般実装。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6o は完了し、次の gate は B8-G6p。

#### PR Gate: B8-G6p Return-To Continuation objc_alloc_init Delegate Class Bridge

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6o の
  `return_to_continuation_objc_alloc_init_class_bridge_unimplemented` を受けて、
  `class_rebase.resolved_vm_address=4294988184` を self-authored fixture の delegate class
  bridge boundary として扱う。
- [x] public Mach-O / Objective-C metadata から `BaraGuiHelloWorldDelegate` identity を
  解決できる場合は保存し、まだ解決できない場合は stable class-identity blocker として
  分類する。
- [x] `_objc_alloc_init` の host-side substitute が必要な場合は、self-authored B8 GUI
  fixture の delegate class に限る bridge contract として保存する。arbitrary Objective-C
  class allocation / initialization bridge にはしない。
- [x] `_objc_alloc_init` return value が `rax` に入り、後続の `mov rdx, rax` で
  `setDelegate:` argument へ渡る dataflow を維持する。

PR に含めない:

- 任意の Objective-C class / instance bridge。
- 一般的な call-rel32 execution engine。
- dynamic linker / dyld stub binding の一般実装。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6p は完了し、次の gate は B8-G6q。

#### PR Gate: B8-G6q Return-To Continuation objc_alloc_init Fixture Delegate Bridge Contract

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6p の
  `return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented` を受けて、
  `_OBJC_CLASS_$_BaraGuiHelloWorldDelegate` に限る fixture delegate bridge contract として
  保存する。
- [x] `b8_return_to_continuation_objc_alloc_init_class_identity_v0` の
  `public_mach_o_symtab_nlist64` 解決結果を contract input に接続し、private Objective-C
  runtime metadata には依存しない。
- [x] `_objc_alloc_init` host-side substitute が返す値は x86_64 `rax` writeback の
  producer として扱い、後続の `mov rdx, rax` / `setDelegate:` dataflow を維持する。
- [x] 次 blocker を fixture delegate host execution の未実装境界として分類する。

PR に含めない:

- 任意の Objective-C class / instance bridge。
- Objective-C object layout、isa pointer、private runtime metadata の解釈。
- 一般的な `_objc_alloc_init` execution engine。
- dynamic linker / dyld stub binding の一般実装。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6q は完了し、次の gate は B8-G6r。

#### PR Gate: B8-G6r Return-To Continuation objc_alloc_init Fixture Delegate Host Execution

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6q の
  `return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_unimplemented` を受けて、
  self-authored fixture delegate substitute を public Objective-C / AppKit API helper で
  実行する。
- [x] host execution result を
  `b8_return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_v0` として保存し、
  `host_pointer_u64` output を `_objc_alloc_init` return value として扱う。
- [x] `_objc_alloc_init` return value を x86_64 `rax` writeback に接続し、後続の
  `mov rdx, rax` が `setDelegate:` argument として available になることを保存する。
- [x] 次 blocker を `setDelegate:` helper execution boundary へ進める。

PR に含めない:

- x86_64 binary 内の Objective-C object layout / method table / isa pointer 解釈。
- 任意の Objective-C class / instance bridge。
- 一般的な `_objc_alloc_init` execution engine。
- delegate callback into translated code。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6r は完了し、次の gate は B8-G6s。

#### PR Gate: B8-G6s Return-To Continuation setDelegate Helper Execution Boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6r の `return_to_continuation_objc_helper_execution_unimplemented` を受けて、
  selector `setDelegate:` の helper request / bridge contract / host execution report を
  `setActivationPolicy:` 専用 contract から分離する。
- [x] `_objc_alloc_init` fixture delegate substitute の `host_pointer_u64` を raw
  cross-process pointer として扱わず、fixture-scoped host object / session / handle
  boundary または same-helper-process substitute の実行条件として stable report する。
- [x] public Objective-C / AppKit API helper で
  `NSApp setDelegate:<BaraGuiHelloWorldDelegate instance>` を実行できる場合は execution
  result を保存し、実行できない場合は host object / session continuity blocker として
  stable に分類する。
- [x] 次 blocker を `setDelegate:` return continuation (`return_to=4294973049`) または
  host object / session continuity の次 action へ進める。

PR に含めない:

- 任意 Objective-C message send の一般実行。
- raw Objective-C object pointer の process 間再利用。
- delegate callback into translated code。
- AppKit run loop / window lifecycle の一般化。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6s は完了し、次の gate は B8-G6t。

#### PR Gate: B8-G6t Return-To Continuation setDelegate Void Return Continuation Decode

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6s の
  `return_to_continuation_objc_helper_void_return_continuation_unimplemented` を受けて、
  `setDelegate:` の void return では x86_64 `rax` value を要求しない continuation
  input model を追加する。
- [x] `return_to=4294973049` から public Mach-O code segment bytes を decode し、
  `mov rdi, qword ptr [r15]` / `mov rsi, qword ptr [rip+disp32]` / `call r14` を
  stable report に保存する。
- [x] preserved `r15` `_NSApp` import global と preserved `_objc_msgSend` target を使って、
  receiver `NSApp` と selector `run` を available state として materialize する。
- [x] 次 blocker を `NSApp run` helper execution boundary または focused unsupported
  instruction / lifecycle boundary へ進める。

PR に含めない:

- AppKit run loop の一般実行。
- window lifecycle / delegate callback into translated code。
- arbitrary void-return Objective-C message continuation。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6t は完了し、次の gate は B8-G6u。

#### PR Gate: B8-G6u Return-To Continuation NSApp run Helper Boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6t の `return_to_continuation_objc_helper_execution_unimplemented` を受けて、
  selector `run` の `_objc_msgSend(NSApp, run)` を no-argument Objective-C helper
  request として扱い、x86_64 `rdx` argument を要求しない contract に分ける。
- [x] public Objective-C / AppKit API と self-authored fixture の範囲で、`NSApp run`
  を helper execution または AppKit run-loop lifecycle boundary として stable report
  する。
- [x] 次 blocker を `return_to=4294973062` continuation、window lifecycle / delegate
  callback boundary、または focused AppKit run-loop boundary へ進める。

PR に含めない:

- AppKit run loop の一般実行。
- window lifecycle / delegate callback into translated code の一般化。
- arbitrary no-argument Objective-C message send。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6u は完了し、次の gate は B8-G6v。

#### PR Gate: B8-G6v AppKit run-loop lifecycle observation boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6u の `return_to_continuation_appkit_run_loop_lifecycle_unimplemented` を受けて、
  `NSApp run` を self-authored B8 GUI fixture の AppKit lifecycle observation
  boundary として扱う。
- [x] public Objective-C / AppKit API helper で、fixture delegate の
  `applicationDidFinishLaunching:` 相当が window / label creation event
  `gui_window_created` を観測できる場合は stable report する。
- [x] automated oracle mode では run loop が無期限化しないよう、self-authored fixture
  の bounded termination policy を report し、必要なら次 blocker を timer /
  termination lifecycle boundary へ進める。

PR に含めない:

- arbitrary AppKit application lifecycle の一般化。
- translated-code delegate callback execution。
- `.app` bundle / resource / nib / storyboard 一般対応。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6v は完了し、次の gate は B8-G6w。

#### PR Gate: B8-G6w Post-Run main continuation unsupported instruction boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6v の `return_to_continuation_unsupported_instruction` を受けて、
  `NSApp run` 後の `return_to=4294973062` continuation を focused に扱う。
- [x] self-authored B8 GUI fixture の post-run continuation 先頭命令
  `48 89 df` / `mov rdi, rbx` を stable decode / report できるようにし、
  `_objc_autoreleasePoolPop` へ渡す autorelease pool token handoff または
  focused blocker を保存する。
- [x] 次 blocker を `_objc_autoreleasePoolPop` helper boundary、post-run epilogue
  decode、または narrower continuation blocker へ進める。

PR に含めない:

- arbitrary register-to-register move の一般化。
- arbitrary autorelease pool lifecycle execution。
- full x86_64 function epilogue completion。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6w は完了し、次の gate は B8-G6x。

#### PR Gate: B8-G6x Autorelease pool saved-register token materialization boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6w の
  `return_to_continuation_saved_register_value_materialization_unimplemented` を受けて、
  post-run `mov rdi, rbx` の source register `rbx` を focused に扱う。
- [x] self-authored B8 GUI fixture の initial `_objc_autoreleasePoolPush` return value が
  `mov rbx, rax` で saved register に保持され、`NSApp run` 後の
  `_objc_autoreleasePoolPop` argument へ渡る handoff を stable report する。
- [x] 次 blocker を `_objc_autoreleasePoolPop` helper boundary、post-run epilogue
  decode、または narrower continuation blocker へ進める。

PR に含めない:

- arbitrary callee-saved register dataflow generalization。
- arbitrary autorelease pool lifecycle execution。
- full x86_64 function epilogue completion。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6x は完了し、次の gate は B8-G6y。

#### PR Gate: B8-G6y Autorelease pool pop helper boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6x の
  `return_to_continuation_call_rel32_helper_execution_unimplemented` を受けて、
  post-run `call_rel32` at `4294973065` / target `_objc_autoreleasePoolPop` を
  focused helper boundary として扱う。
- [x] `rdi` token argument は initial `_objc_autoreleasePoolPush` return value 由来の
  saved `rbx` handoff であることを保ったまま、public Objective-C runtime
  autorelease pool helper contract を stable report する。
- [x] 次 blocker を post-run epilogue decode、function return completion、または
  narrower continuation blocker へ進める。

PR に含めない:

- arbitrary call-rel32 helper execution。
- arbitrary autorelease pool lifecycle execution。
- raw helper-process pointer reuse across helper processes。
- full x86_64 function epilogue completion。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6y は完了し、次の gate は B8-G6z。

#### PR Gate: B8-G6z Post-run epilogue stack adjustment boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6y の `return_to_continuation_unsupported_instruction` を受けて、
  post-run epilogue の `48 83 c4 08` / `add rsp, 8` at `4294973072` を
  focused に decode / report する。
- [x] stack restore instruction は post-run helper boundary 後の epilogue として
  stable report し、次 blocker を `pop rbx`、`pop r14`、`pop r15`、`pop rbp`、
  `ret`、または narrower epilogue blocker へ進める。
- [x] `_objc_autoreleasePoolPop` helper boundary が executed のまま維持されることを
  regression として確認する。

PR に含めない:

- arbitrary stack pointer arithmetic。
- full x86_64 epilogue completion。
- general stack frame unwinding。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6z は完了し、次の gate は B8-G6aa。

#### PR Gate: B8-G6aa Post-run epilogue preserved rbx restore boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6z の `return_to_continuation_unsupported_instruction` を受けて、
  post-run epilogue の `5b` / `pop rbx` at `4294973076` を focused に decode /
  report する。
- [x] `pop rbx` は post-run epilogue の preserved-register restore として stable
  report し、次 blocker を `pop r14` at `4294973077`、`pop r15`、`pop rbp`、
  `ret`、または narrower epilogue blocker へ進める。
- [x] B8-G6z の `_objc_autoreleasePoolPop` executed boundary と
  `b8_return_to_continuation_epilogue_stack_adjustment_v0` regression を維持する。

PR に含めない:

- arbitrary pop / stack-memory semantics。
- full callee-saved register restoration。
- full epilogue completion。
- general stack frame unwinding。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6aa は完了し、次の gate は B8-G6ab。

#### PR Gate: B8-G6ab Post-run epilogue preserved r14 restore boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6aa の `return_to_continuation_unsupported_instruction` を受けて、
  post-run epilogue の `41 5e` / `pop r14` at `4294973077` を focused に decode /
  report する。
- [x] `pop r14` は post-run epilogue の preserved-register restore として stable
  report し、次 blocker を `pop r15` at `4294973079`、`pop rbp`、`ret`、または
  narrower epilogue blocker へ進める。
- [x] B8-G6z/G6aa の `_objc_autoreleasePoolPop` executed boundary、
  `b8_return_to_continuation_epilogue_stack_adjustment_v0`、
  `b8_return_to_continuation_epilogue_register_restore_v0` regression を維持する。

PR に含めない:

- arbitrary REX-prefixed pop semantics。
- full callee-saved register restoration。
- full epilogue completion。
- general stack frame unwinding。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6ab は完了し、次の gate は B8-G6ac。

#### PR Gate: B8-G6ac Post-run epilogue preserved r15 restore boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6ab の `return_to_continuation_unsupported_instruction` を受けて、
  post-run epilogue の `41 5f` / `pop r15` at `4294973079` を focused に decode /
  report する。
- [x] `pop r15` は post-run epilogue の preserved-register restore として stable
  report し、次 blocker を `pop rbp` at `4294973081`、`ret`、または narrower
  epilogue blocker へ進める。
- [x] B8-G6z-G6ab の `_objc_autoreleasePoolPop` executed boundary、
  epilogue stack adjustment report、epilogue register restore report regression を維持する。

PR に含めない:

- arbitrary REX-prefixed pop semantics。
- full callee-saved register restoration。
- full epilogue completion。
- general stack frame unwinding。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6ac は完了し、次の gate は B8-G6ad。

#### PR Gate: B8-G6ad Post-run epilogue frame-pointer restore boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6ac の `return_to_continuation_unsupported_instruction` を受けて、
  post-run epilogue の `5d` / `pop rbp` at `4294973081` を focused に decode /
  report する。
- [x] `pop rbp` は post-run epilogue の frame-pointer restore として stable report し、
  次 blocker を `ret` at `4294973082`、function completion、または narrower epilogue
  blocker へ進める。
- [x] B8-G6z-G6ac の `_objc_autoreleasePoolPop` executed boundary、epilogue stack
  adjustment report、epilogue register restore report regression を維持する。

PR に含めない:

- arbitrary pop / stack-memory semantics。
- full frame unwinding。
- full epilogue completion。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6ad は完了し、次の gate は B8-G6ae。

#### PR Gate: B8-G6ae Post-run epilogue return terminator completion boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6ad の `return_to_continuation_unsupported_instruction` を受けて、
  post-run epilogue の `c3` / `ret` at `4294973082` を focused に stable report する。
- [x] `ret` 後の trailing zero / padding blocker `DecodeUnsupportedOpcode { opcode: 0 }`
  at `4294973083` は function completion または post-ret padding boundary として
  分離し、Hello World GUI 完遂に必要な次 blocker を stable に分類する。
- [x] B8-G6z-G6ad の `_objc_autoreleasePoolPop` executed boundary、epilogue stack
  adjustment report、epilogue register restore / frame-pointer restore report regression を
  維持する。

PR に含めない:

- general return-to-continuation execution。
- arbitrary post-ret byte interpretation。
- whole-function unwinding / stack-frame validation。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。B8-G6ae は完了し、次の gate は B8-G6af。

#### PR Gate: B8-G6af Self-authored continuation execution completion boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6ae の `return_to_continuation_execution_unimplemented` を受けて、
  self-authored GUI fixture の modeled return-to-continuation execution completion を
  focused に stable report する。
- [x] `b8_return_to_continuation_epilogue_return_completion_v0` と
  AppKit run loop / autorelease pool / epilogue restore reports を維持したまま、
  Hello World GUI 完遂に必要な次 blocker が残るか、modeled real-entry launch path が
  completed と見なせるかを stable に分類する。
- [x] B8-HWGUI 大目標の完遂条件に対して、automated expected/actual comparison と
  manual visible mode に残る差分を reviewable に報告できる。

PR に含めない:

- general return-to-continuation execution。
- arbitrary Objective-C message send / arbitrary continuation call execution。
- decoded continuation block の native execution。
- translation cache、fallback JIT/interpreter。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- B8-HWGUI 大目標の途中 slice として commit / push 後も、次 blocker が focused
  slice として切れる限り継続する。

#### PR Gate: B8-HWGUI Final expected/actual and manual visible review boundary

branch: `task/b8-hello-world-gui-complete`

完了条件:

- [x] B8-G6af の `review_b8_hello_world_gui_completion` を受けて、self-authored
  GUI fixture の real-entry modeled helper continuation chain が
  `b8_return_to_continuation_modeled_execution_completion_v0` /
  `launch_path_status=completed` で安定していることを確認する。
- [x] automated mode で Rosetta expected / Bara actual の stable JSON comparison が
  Hello World GUI 完遂条件に対して一致するか、残る差分が B8-HWGUI 完遂 blocker か
  post-completion 拡張対象かを stable report / review package に記録する。
- [x] manual visible mode で Bara 経由の実 entry path から `hello world` window / label を
  表示できるか、実行環境上の制約があれば blocker として記録する。
- [x] B8-HWGUI 大目標の完遂条件を満たした場合は、TODO / progress を完了状態にし、
  branch を push して draft PR を開き、review gate で停止する。

PR に含めない:

- B8-OSS0 source-built OSS GUI app automation。
- general continuation execution、arbitrary Objective-C message send、
  translation cache、fallback JIT/interpreter。
- `.app` bundle / resource 一般化。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`
- manual visible mode の実行手順と結果を review package に記録する。

review gate:

- B8-HWGUI 完遂時点で draft PR を開いて停止する。merge までは B8-OSS0 に進まない。
- Draft PR: https://github.com/serika12345/Bara/pull/49

completion evidence:

- 2026-06-13 19:53 JST: automated expected / actual は
  `target/b8-hwgui-review/expected.json` と `target/b8-hwgui-review/actual.json` の
  `compare-expected-actual` が `{"issues":[]}` で一致した。debug bundle の real-entry
  modeled helper continuation chain も
  `b8_return_to_continuation_modeled_execution_completion_v0` /
  `launch_path_status=completed` を確認した。
- 2026-06-13 20:01 JST: manual visible は
  `run-arm64-gui-hello-world-translated-visible` で
  `target/b8-hwgui-review/manual-visible.launch-report.json` を保存した。
  WindowServer から `Bara GUI Hello World` window の on-screen title / bounds を確認し、
  launch report は `mode=manual_visible`、`status=gui_visible_ready`、
  `stdout={"event":"gui_window_created","title":"Bara GUI Hello World","text":"hello world"}`、
  `exit_status=0` を保存している。
- 2026-06-13 20:09 JST: B8-HWGUI review gate として draft PR
  https://github.com/serika12345/Bara/pull/49 を開いた。merge review までは
  B8-OSS0 に進まない。

- [ ] B8-G6: Objective-C runtime / AppKit helper bridge を B8-G1 専用 lifecycle
  event から一般化する。
  - [ ] class lookup、selector lookup、message send、autorelease pool、main run loop
    lifecycle を public ObjC runtime / AppKit API helper capability として扱う。
  - [ ] x86_64 calling convention から helper argument / return value へ渡す
    marshaling boundary を定義する。
- [ ] B8-G7: process state と runtime service を最小 GUI app に必要な範囲で増やす。
  - [ ] initial stack、argv / envp、heap allocation、file descriptor、TLS、
    initial thread、signals / exceptions を user-space runtime metadata と
    helper boundary で扱う。
  - [ ] 未対応 process-wide state は stable blocker として分類し、silent fallback
    しない。
- [ ] B8-G8: `.app` bundle と resource を含む fixture へ拡張するか判断する。
  - [ ] single executable の限界が blocker になった時点で、Info.plist、bundle
    identifier、resources、assets、nib/storyboard 相当を scope 化する。
  - [ ] bundle 化は B8-G2 から B8-G7 の実行経路が必要になるまで先送りする。

#### Large Target: B8-HWGUI Self-Authored Hello World GUI Completion

branch: `task/b8-hello-world-gui-complete`

目的:

- [x] self-authored x86_64 Mach-O GUI Hello World fixture を、B8-G1 専用
  sentinel / host trap ではなく、実 `LC_MAIN` entry から進めて GUI 起動完遂まで通す。
- [x] public Mach-O metadata、public Objective-C runtime / AppKit API、自前 fixture、
  Rosetta black-box observable result だけを根拠にする。
- [x] debug bundle の next blocker を source of truth にし、必要な ISA / loader /
  helper / process-state boundary を focused step として追加する。

完遂条件:

- [x] automated mode で Rosetta expected / Bara actual の stable JSON comparison が通る。
- [x] manual visible mode で Bara 経由の実 entry path から `hello world` window / label を
  表示できる。
- [x] launch report / debug bundle が、実 `LC_MAIN` entry から GUI lifecycle helper
  boundary まで進んだ事実を保存し、B8-G1 専用 sentinel / host trap path と区別できる。
- [x] self-authored GUI Hello World fixture の expected launch path 上に残る blocker が
  `unsupported_instruction` / `unsupported_import` / `unsupported_loader_feature` /
  Objective-C / AppKit helper boundary として未処理ではない。
- [x] 完遂後に残る next blocker が、Hello World GUI 完遂後の拡張対象
  （例: arbitrary ObjC message send、translation cache、OSS app 固有 boundary）として
  stable report されている。

自動進行方針:

- `/advance-pr` は従来通り、次の unfinished `PR Gate` だけを進める。
- `/advance-large` は、ユーザーが B8-HWGUI を明示した場合に限り、この大目標を対象に
  してよい。その場合も各 coherent step は debug bundle blocker 由来の小さな
  TODO-backed slice として commit し、想定外の広い boundary に当たったら TODO を更新して
  停止する。
- B8-HWGUI の途中で、arbitrary Objective-C message send、general continuation execution、
  arbitrary indirect target execution、translation cache / JIT / fallback interpreter、
  `.app` bundle / resource が必須 blocker になった場合は、先に focused PR Gate または
  sub-target として TODO に追加する。
- B8-HWGUI 完遂時点は review gate とする。draft PR を開いて停止し、merge までは次の
  OSS GUI app target へ進まない。

PR に含めない:

- 任意 GUI app の一般対応。
- 外部 OSS app の実行対応。
- Wine / PE 接続。
- Rosetta disassembly、private dyld behavior、private Objective-C / AppKit internals に
  基づく実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`
- manual visible mode の GUI 目視確認手順と結果を review package に記録する。

review gate:

- 完遂したら commit / push / draft PR 作成で停止する。review / merge 後に、
  B8-OSS0 へ進むか、x86 32-bit / PE-Wine 前段へ戻るかを判断する。

#### PR Gate: B8-ARCH0 Post-HWGUI Runtime Architecture Record

branch: `task/b8-hello-world-gui-complete`

目的:

- [x] B8-HWGUI 完遂後の議論を、Rosetta 同等の user-space runtime と Wine 接続を見据えた
  architecture direction として固定する。
- [x] Bara の主経路を、ユーザー visible な変換済み app 出力ではなく、内部
  translation artifact / cache / dispatcher / OS personality として定義する。
- [x] 同 OS / 異アーキテクチャを主対象にし、異 OS 互換性は Wine などの OS personality
  へ委譲する責務分担を記録する。

完了条件:

- [x] [docs/runtime-architecture-roadmap.md](docs/runtime-architecture-roadmap.md) に
  final goal、layer boundaries、Wine 接続、B8-HWGUI 後の roadmap が記録されている。
- [x] [docs/design-todo.md](docs/design-todo.md) に B8-HWGUI 後の architecture decision が
  記録されている。
- [x] B8-HWGUI 後に進む抽象化 milestone が TODO に定義されている。
- [x] [docs/progress.md](docs/progress.md) の snapshot が review gate と次 action を
  反映している。

PR に含めない:

- code refactor / module extraction。
- translation artifact / cache 実装。
- runtime dispatcher 実装。
- B8-OSS0 target app 選定。
- Wine bridge 実装。

検証:

- `git diff --check`
- `nix develop -c ./scripts/check-no-invisible-chars`

review gate:

- docs-only architecture record と milestone definition を commit / push したら停止する。
  merge review 後は B8-ARCH1 から開始し、B8-OSS0 は抽象化 milestone の進行状況を見て
  開始する。

#### PR Gate: B8-ARCH1 Post-HWGUI Responsibility Split Audit

branch: `task/b8-arch1-responsibility-split-audit`

完了条件:

- [x] merge 後の最初の作業として、この Future Target を concrete `PR Gate` に落とした。
- [x] 最初の PR は audit / classification / extraction order 決定を主対象にし、
  behavior-changing refactor や大きな file move から始めていない。
- [x] `crates/btbc-cli/src/b8_debug_bundle.rs`、`crates/btbc-cli/src/main.rs`、
  `crates/bara-oracle/src/binary_format/` に残る B8-specific logic を棚卸しした。
- [x] logic を loader image model、runtime modeling、helper bridge、report DTO、
  fixture / oracle I/O、CLI boundary に分類した。
- [x] 抽出順を smallest coherent follow-up gate として定義した。
- [x] behavior は変えず、既存 B8-HWGUI verification を維持する方針を固定した。
- [x] audit 結果を [docs/design-todo.md](docs/design-todo.md) と
  [docs/runtime-architecture-roadmap.md](docs/runtime-architecture-roadmap.md) に反映した。

PR に含めない:

- Rust code refactor / module extraction。
- B8-HWGUI fixture 専用 path の追加機能。
- B8-OSS0 target app 選定。
- translation cache、fallback JIT/interpreter、Wine bridge 実装。

検証:

- `git diff --check`
- `nix develop -c ./scripts/check-no-invisible-chars`

review gate:

- docs-only audit と follow-up gate definition を commit / push / draft PR 作成で停止する。
  次の gate は B8-ARCH1a。

#### PR Gate: B8-ARCH1a ISA Semantic Coverage Plan

branch: `task/b8-arch1a-isa-semantic-coverage-plan`

B8-ARCH1 が review / merge 済みになるまで開始しない。B8-ARCH1 の responsibility
split audit と同じ docs / design phase に属するが、PR Gate は分ける。

- [x] 一般アプリ化で必要になる x86_64 instruction coverage を、opcode list ではなく
  semantic bucket catalog として整理する。
- [x] B8-HWGUI で追加した focused instruction slice を、prologue / epilogue、
  RIP-relative addressing、register-indirect load、integer zeroing、import/helper call、
  fixture-scoped host service などの bucket に再分類する。
- [x] decode-only、lift-ready、direct ARM64 lowering、helper-required、
  fallback-required、stable blocker の状態語彙を固定する。
- [x] direct lowering へ進める bucket と、helper / interpreter fallback へ逃がす bucket の
  判断基準を design TODO に記録する。
- [x] unsupported instruction report が opcode だけでなく semantic bucket、operand shape、
  required runtime service を返せるようにするための report schema 方針を定義する。
- [x] permissive decoder 依存を検討する場合の clean-room / license / supply-chain
  checklist を定義する。lift / IR / runtime semantics は Bara の domain model として保持する。
- [x] Intel SDM / Arm A64 docs / ABI specs / Mach-O public docs を primary source とし、
  Intel XED / iced-x86 / Zydis / Capstone / Remill / McSema / FEX / Box64 / DynamoRIO を
  dependency candidate または research reference として分類した reference inventory を作る。
- [x] permissive dependency を採用する前の gate として、license / notice / transitive
  dependency / Nix packaging / `verify-supply-chain` の確認項目を定義する。

completion evidence:

- [docs/design-todo.md](docs/design-todo.md) の D4a に
  `b8_arch1a_isa_semantic_bucket_catalog_v0` として semantic bucket catalog、
  B8-HWGUI focused slice の再分類、status vocabulary、direct/helper/fallback 判断基準、
  unsupported report schema 方針、decoder dependency adoption checklist を記録した。
- [docs/runtime-architecture-roadmap.md](docs/runtime-architecture-roadmap.md) の R1a に
  B8-ARCH1a audit result を追加し、OSS app cycle で次に潰す対象を opcode ではなく
  semantic bucket として選ぶ方針を固定した。
- Reference inventory は同 roadmap の
  `Reference Materials And Permissive Candidates` を source of truth とし、B8-ARCH1a は
  依存採用ではなく候補分類と採用前 gate の定義に留めた。

PR に含めない:

- decoder dependency 採用。
- ISA implementation / lowering 追加。
- supply-chain lockfile / toolchain 変更。

検証:

- `git diff --check`
- `nix develop -c ./scripts/check-no-invisible-chars`

review gate:

- docs-only semantic coverage plan を commit / push / draft PR 作成で停止する。
  依存採用や implementation は後続 gate に分ける。

#### PR Gate: B8-ARCH2a B8 Debug Bundle Report DTO Module Split

branch: `task/b8-arch2a-debug-report-dto-module-split`

完了条件:

- [x] B8-ARCH1 audit の抽出順に従い、`crates/btbc-cli/src/b8_debug_bundle.rs` 内の
  schema-only `B8Debug*Report` DTO を `b8_debug_bundle` 配下の report module へ分ける。
- [x] JSON schema 名、field 名、既存 B8-HWGUI debug bundle output を維持する。
- [x] helper process execution、bundle file I/O、loader image model、runtime dispatcher は
  まだ移動しない。
- [x] module split 後も behavior-changing refactor を混ぜず、既存 verification を維持する。

completion evidence:

- `crates/btbc-cli/src/b8_debug_bundle/report.rs` を追加し、entry bytes、
  decode instruction、unsupported instruction、artifact、launch、runtime attempt、
  blocker、stage / source / memory-width report DTO を移した。
- JSON `schema` 文字列、serde tag / rename、field 名は維持し、親 module 側は既存の
  orchestration から `report` module の DTO constructor を呼ぶだけにした。
- helper process execution、bundle file I/O、loader/import projection、modeled continuation、
  runtime dispatcher 相当の処理は `b8_debug_bundle.rs` に残し、後続 gate に分けた。

PR に含めない:

- `bara-oracle` からの loader/image model 抽出。
- Objective-C / AppKit helper bridge 一般化。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- report DTO split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2b B8 Debug Bundle I/O Boundary Split

branch: `task/b8-arch2b-debug-bundle-io-boundary`

B8-ARCH2a が review / merge 済みになるまで開始しない。B8-ARCH1 audit の抽出順に従い、
report DTO split の次に bundle file I/O と repro script generation だけを分ける。

完了条件:

- [x] `crates/btbc-cli/src/b8_debug_bundle.rs` 内の bundle directory layout、
  JSON/bin/text file read/write helper、repro script generation を
  `crates/btbc-cli/src/b8_debug_bundle/io.rs` へ分ける。
- [x] `generate_b8_debug_bundle` の output path JSON、file 名、repro script content、
  B8-HWGUI debug bundle output を維持する。
- [x] decode/lift/emit/runtime attempt orchestration、report DTO、loader/import projection、
  helper process execution、runtime dispatcher は移動しない。
- [x] module split 後も behavior-changing refactor を混ぜず、既存 verification を維持する。

completion evidence:

- `crates/btbc-cli/src/b8_debug_bundle/io.rs` を追加し、`B8DebugBundleOutputPaths`、
  `B8DebugReproScript`、bundle directory creation、binary/text/JSON file read/write helper を
  移した。
- output path JSON の field 名、bundle file 名、repro script command string は維持し、
  `generate_b8_debug_bundle` は既存 orchestration から `io` module の boundary helper を
  呼ぶだけにした。
- decode/lift/emit/runtime attempt orchestration、report DTO、loader/import projection、
  helper process execution、runtime dispatcher 相当の処理は移動していない。

PR に含めない:

- `B8RealEntryAttempt` / attempt orchestration 抽出。
- `GuestImage` / loader image model 抽出。
- Objective-C / AppKit helper bridge 一般化。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- bundle I/O boundary split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2c B8 Debug Bundle Attempt Orchestration Split

branch: `task/b8-arch2c-debug-attempt-orchestration-split`

B8-ARCH2b が review / merge 済みになるまで開始しない。B8-ARCH1 audit の抽出順に従い、
bundle file I/O split の次に real-entry attempt orchestration と report assembly の入口だけを
分ける。

完了条件:

- [x] `crates/btbc-cli/src/b8_debug_bundle.rs` 内の `B8RealEntryAttempt` と
  decode/lift/emit/runtime attempt orchestration helper を
  `crates/btbc-cli/src/b8_debug_bundle/attempt.rs` へ分ける。
- [x] `generate_b8_debug_bundle` の JSON output、blocker classification、runtime attempt
  behavior、B8-HWGUI debug bundle output を維持する。
- [x] bundle file I/O、report DTO、loader/import projection、helper process execution、
  runtime dispatcher は移動しない。
- [x] module split 後も behavior-changing refactor を混ぜず、既存 verification を維持する。

completion evidence:

- `crates/btbc-cli/src/b8_debug_bundle/attempt.rs` を追加し、`B8RealEntryAttempt`、
  decode/lift/emit/runtime attempt orchestration、unsupported terminator frontier helper を
  移した。
- `generate_b8_debug_bundle` は既存の attempt result fields を読む形を維持し、JSON output、
  blocker classification、runtime attempt behavior は変えない。
- bundle file I/O、report DTO、loader/import projection、helper process execution、
  runtime dispatcher 相当の処理は移動していない。

PR に含めない:

- `GuestImage` / loader image model 抽出。
- Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- attempt orchestration split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2d B8 Debug Bundle Loader Plan Shell Split

branch: `task/b8-arch2d-debug-loader-plan-shell-split`

B8-ARCH2c が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の preparatory slice として、`GuestImage` 本体抽出に入る前に B8 debug
bundle の loader plan shell だけを分ける。

完了条件:

- [x] `crates/btbc-cli/src/b8_debug_bundle.rs` 内の `B8DebugLoaderPlanReport` と直接の
  loader plan shell DTO を `crates/btbc-cli/src/b8_debug_bundle/loader.rs` へ分ける。
- [x] `generate_b8_debug_bundle` の `loader.plan.json` schema 名、field 名、JSON output、
  helper boundary request の launch report 接続を維持する。
- [x] import boundary projection、helper process execution、modeled continuation state、
  runtime dispatcher、`GuestImage` / `MachOImage` domain model 本体抽出は移動しない。
- [x] module split 後も behavior-changing refactor を混ぜず、既存 verification を維持する。

completion evidence:

- `crates/btbc-cli/src/b8_debug_bundle/loader.rs` を追加し、
  `B8DebugLoaderPlanReport`、direct loader plan metadata DTO、loader deferred step DTO を
  移した。
- `generate_b8_debug_bundle` は `B8DebugLoaderPlanReport::real_lc_main_attempted` を呼び、
  `helper_boundary_request()` で launch report へ既存 helper boundary request を接続する。
- import boundary projection、helper process execution、modeled continuation state、
  runtime dispatcher、`GuestImage` / `MachOImage` 本体抽出は移動していない。

PR に含めない:

- `GuestImage` / `MachOImage` domain model 本体抽出。
- import/fixup projection の意味変更または schema 変更。
- Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- loader plan shell split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2e B8 Debug Bundle Import Boundary Projection Split

branch: `task/b8-arch2e-debug-import-boundary-projection-split`

B8-ARCH2d が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の preparatory slice として、loader plan shell の次に import boundary projection
と public import metadata report を分ける。

完了条件:

- [x] `crates/btbc-cli/src/b8_debug_bundle.rs` 内の `B8DebugImportBoundaryReport` と
  public import metadata projection DTO を
  `crates/btbc-cli/src/b8_debug_bundle/import_boundary.rs` へ分ける。
- [x] `loader.plan.json` の import boundary schema 名、field 名、JSON output、
  helper boundary request の launch report 接続を維持する。
- [x] helper request / marshaling、Objective-C / AppKit helper process execution、
  modeled continuation state、runtime dispatcher、`GuestImage` / `MachOImage` domain model
  本体抽出は移動しない。
- [x] module split 後も behavior-changing refactor を混ぜず、既存 verification を維持する。

completion evidence:

- `crates/btbc-cli/src/b8_debug_bundle/import_boundary.rs` を追加し、
  `B8DebugImportBoundaryReport`、public import metadata report、dyld info / dylib /
  linkedit projection DTO、import boundary resolution / next action enum を移した。
- `loader.rs` は `B8DebugImportBoundaryReport::from_probe_and_decode_report` を呼び、
  `helper_boundary_request()` で launch report へ既存 helper boundary request を接続する。
- helper request / marshaling、Objective-C / AppKit helper process execution、
  modeled continuation state、runtime dispatcher、`GuestImage` / `MachOImage` 本体抽出は
  移動していない。

PR に含めない:

- `GuestImage` / `MachOImage` domain model 本体抽出。
- import/fixup projection の意味変更または schema 変更。
- helper request / marshaling DTO の module split。
- Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- import boundary projection split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2f B8 Debug Bundle Helper Boundary Marshaling Split

branch: `task/b8-arch2f-debug-helper-boundary-marshaling-split`

B8-ARCH2e が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の preparatory slice として、import boundary projection の次に helper
boundary request と import helper marshaling contract の shell を分ける。

完了条件:

- [x] `crates/btbc-cli/src/b8_debug_bundle.rs` 内の `B8DebugHelperBoundaryRequestReport`、
  `B8DebugImportHelperRequestReport`、`B8DebugHelperMarshalingReport`、import helper
  marshaling contract DTO、直接の helper marshaling enum / value-source DTO を
  `crates/btbc-cli/src/b8_debug_bundle/helper_boundary.rs` へ分ける。
- [x] `loader.plan.json` と launch report の helper boundary request schema 名、field 名、
  JSON output を維持する。
- [x] Objective-C / AppKit helper process execution、modeled continuation state、runtime
  dispatcher、`GuestImage` / `MachOImage` domain model 本体抽出は移動しない。
- [x] module split 後も behavior-changing refactor を混ぜず、既存 verification を維持する。

completion evidence:

- `crates/btbc-cli/src/b8_debug_bundle/helper_boundary.rs` を追加し、
  `B8DebugHelperBoundaryRequestReport`、`B8DebugImportHelperRequestReport`、
  `B8DebugHelperMarshalingReport`、`B8DebugImportHelperMarshalingContractReport`、
  helper calling convention / argument / return / value-source DTO、helper boundary
  blocker / blocked reason を移した。
- `loader.rs`、`import_boundary.rs`、`report.rs` は `helper_boundary` module の
  helper boundary request type を使う形にした。`loader.plan.json` と launch report の
  helper boundary request field 名、schema 名、JSON output は変えない。
- Objective-C / AppKit helper process execution、modeled continuation state、runtime
  dispatcher、`GuestImage` / `MachOImage` 本体抽出は移動していない。

PR に含めない:

- `GuestImage` / `MachOImage` domain model 本体抽出。
- import/fixup projection の意味変更または schema 変更。
- Objective-C / AppKit helper bridge 一般化、または helper process execution の移動。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- helper boundary marshaling split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2g B8 Debug Bundle Guest Image Mapping Shell Split

branch: `task/b8-arch2g-debug-guest-image-mapping-shell`

B8-ARCH2f が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の preparatory slice として、loader plan shell に残っている guest image
mapping summary を分ける。これは `GuestImage` / `MachOImage` domain model 本体抽出の
前段であり、既存 debug bundle JSON を維持する。

完了条件:

- [x] `crates/btbc-cli/src/b8_debug_bundle/loader.rs` 内の image mapping summary DTO
  （entry PC、code segment VM address / byte length、segment source、address space、
  mapped bytes source）を `crates/btbc-cli/src/b8_debug_bundle/guest_image.rs` へ分ける。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] import/fixup projection、helper boundary、Objective-C / AppKit helper process execution、
  modeled continuation state、runtime dispatcher、`GuestImage` / `MachOImage` domain model
  本体抽出は移動しない。
- [x] module split 後も behavior-changing refactor を混ぜず、既存 verification を維持する。

completion evidence:

- `crates/btbc-cli/src/b8_debug_bundle/guest_image.rs` を追加し、
  `B8DebugGuestImageMappingReport` と直接の segment source / address space /
  mapped bytes source enum を `loader.rs` から分けた。
- `loader.rs` は `B8DebugGuestImageMappingReport::from_entry_input` を呼び、
  `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output は
  変えない。
- import/fixup projection、helper boundary、Objective-C / AppKit helper process execution、
  modeled continuation state、runtime dispatcher、`GuestImage` / `MachOImage` 本体抽出は
  移動していない。

PR に含めない:

- `GuestImage` / `MachOImage` domain model 本体抽出。
- import/fixup projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- guest image mapping shell split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2h Runtime GuestImage Domain Shell

branch: `task/b8-arch2h-guest-image-domain-shell`

B8-ARCH2g が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の最初の実装 slice として、runtime-facing の `GuestImage` domain shell を
追加し、B8 debug bundle の existing image mapping report をその shell から作る。
これは `MachOImage` 本体抽出や import/fixup/symbol identity 移動の前段であり、既存
debug bundle JSON を維持する。

完了条件:

- [x] `crates/bara-runtime/src/guest_image/` に parser 非依存の `GuestImage` shell を追加し、
  entry point、code segment range、segment source、address space、mapped bytes source を
  domain type として保持する。
- [x] B8 debug bundle の `image_mapping` report は `MachOEntryFunctionInput` を直接読むだけでなく、
  runtime-facing `GuestImage` shell へ射影してから既存 JSON DTO を作る。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `MachOImage` domain model 本体、imports/fixups/symbol identity の移動、
  `bara-oracle` からの loader domain 抽出、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `crates/bara-runtime/src/guest_image/mod.rs` を追加し、`GuestImage`、
  `GuestImageEntryPoint`、`GuestImageSegment`、`GuestImageSegments`、mapping source enum、
  validation error を定義した。
- `crates/btbc-cli/src/b8_debug_bundle/guest_image.rs` は
  `MachOEntryFunctionInput` から `GuestImage::mach_o_executable` へ射影し、その domain shell から
  existing `B8DebugGuestImageMappingReport` を作る。
- `loader.plan.json` の `image_mapping` schema / field / serde 値は変えない。

PR に含めない:

- `MachOImage` domain model 本体抽出。
- public Mach-O imports / fixups / symbol identity を runtime domain へ移す作業。
- import/fixup projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`
- `nix develop -c ./scripts/verify-supply-chain`

review gate:

- runtime `GuestImage` domain shell 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2i Runtime GuestImage Mapped Bytes Shell

branch: `task/b8-arch2i-guest-image-mapped-bytes-shell`

B8-ARCH2h が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImage` が mapped bytes domain object を
保持するようにする。B8 debug bundle は既存 `ProgramImageMetadata.mapped_bytes` を
`GuestImage` へ渡し、既存 debug bundle JSON を維持する。

完了条件:

- [x] `bara-runtime::GuestImage` が mapped bytes source だけでなく
  `ProgramImageMappedBytes` を保持し、runtime-facing image shell から read-only に参照できる。
- [x] B8 debug bundle の `GuestImage` projection は
  `MachOEntryFunctionInput::program_image_metadata().mapped_bytes()` を runtime shell へ渡す。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] imports/fixups/symbol identity、`MachOImage` 本体、`bara-oracle` からの loader domain
  抽出、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `GuestImage` に `ProgramImageMappedBytes` field と accessor を追加した。
- `GuestImage::mach_o_executable` は B8 由来の mapped bytes を受け取り、
  runtime-facing image shell に保持する。
- B8 debug bundle は existing `ProgramImageMetadata.mapped_bytes()` を clone して
  `GuestImage` へ渡し、existing `B8DebugGuestImageMappingReport` output は変えない。

PR に含めない:

- imports/fixups/symbol identity の runtime domain 化。
- `MachOImage` domain model 本体抽出。
- import/fixup projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime `GuestImage` mapped bytes shell 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2j Runtime GuestImage Imports Shell

branch: `task/b8-arch2j-guest-image-imports-shell`

B8-ARCH2i が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImage` が import collection domain object を
保持するようにする。B8 debug bundle は既存 `ProgramImageMetadata.imports` を
`GuestImage` へ渡し、既存 debug bundle JSON を維持する。

完了条件:

- [x] `bara-runtime::GuestImage` が `ProgramImageImports` を保持し、runtime-facing image
  shell から read-only に参照できる。
- [x] B8 debug bundle の `GuestImage` projection は
  `MachOEntryFunctionInput::program_image_metadata().imports()` を runtime shell へ渡す。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] fixups/symbol identity、`MachOImage` 本体、`bara-oracle` からの loader domain
  抽出、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `GuestImage` に `ProgramImageImports` field と accessor を追加した。
- `GuestImage::mach_o_executable` は B8 由来の imports collection を受け取り、
  runtime-facing image shell に保持する。
- B8 debug bundle は existing `ProgramImageMetadata.imports()` を clone して
  `GuestImage` へ渡し、existing `B8DebugGuestImageMappingReport` output は変えない。

PR に含めない:

- fixups/symbol identity の runtime domain 化。
- `MachOImage` domain model 本体抽出。
- import/fixup projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime `GuestImage` imports shell 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2k Runtime GuestImage Relocations Shell

branch: `task/b8-arch2k-guest-image-relocations-shell`

B8-ARCH2j が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImage` が relocation/fixup collection
domain object を保持するようにする。B8 debug bundle は既存
`ProgramImageMetadata.relocations` を `GuestImage` へ渡し、既存 debug bundle JSON を維持する。

完了条件:

- [x] `bara-runtime::GuestImage` が `ProgramImageRelocations` を保持し、runtime-facing
  image shell から read-only に参照できる。
- [x] B8 debug bundle の `GuestImage` projection は
  `MachOEntryFunctionInput::program_image_metadata().relocations()` を runtime shell へ渡す。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] symbol identity、`MachOImage` 本体、`bara-oracle` からの loader domain
  抽出、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `GuestImage` に `ProgramImageRelocations` field と accessor を追加した。
- `GuestImage::mach_o_executable` は B8 由来の relocations collection を受け取り、
  runtime-facing image shell に保持する。
- B8 debug bundle は existing `ProgramImageMetadata.relocations()` を clone して
  `GuestImage` へ渡し、existing `B8DebugGuestImageMappingReport` output は変えない。

PR に含めない:

- symbol identity の runtime domain 化。
- `MachOImage` domain model 本体抽出。
- import/fixup projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime `GuestImage` relocations shell 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2l Runtime GuestImage Metadata Collections Shell

branch: `task/b8-arch2l-guest-image-metadata-collections-shell`

B8-ARCH2k が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImage` に
`ProgramImageMetadata` 由来の collection shell をまとめて接続する。既存 mapped bytes /
imports / relocations は `GuestImageMetadata` aggregate に寄せ、残りの sections / symbols /
unwind も runtime-facing image shell から read-only に参照できるようにする。既存 debug
bundle JSON は維持する。

完了条件:

- [x] `bara-runtime::GuestImage` が `GuestImageMetadata` aggregate を保持し、mapped bytes /
  imports / relocations / sections / symbols / unwind を read-only に参照できる。
- [x] B8 debug bundle の `GuestImage` projection は
  `MachOEntryFunctionInput::program_image_metadata()` から `GuestImageMetadata` を作る。
- [x] `GuestImage::mach_o_executable` / `GuestImage::new` の引数肥大化を止め、metadata
  collection 群を個別引数で増やさない。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `MachOImage` 本体、`bara-oracle` からの loader domain 抽出、helper bridge、
  runtime dispatcher は移動しない。

completion evidence:

- `GuestImageMetadata` aggregate を追加し、`GuestImage` は metadata collection 群を
  個別 field ではなく aggregate として保持する。
- `GuestImageMetadata::from_program_image_metadata` は existing `ProgramImageMetadata` から
  sections / mapped bytes / symbols / relocations / imports / unwind を clone して
  runtime-facing image shell に渡す。
- B8 debug bundle は existing `MachOEntryFunctionInput::program_image_metadata()` を
  `GuestImageMetadata` へ射影し、existing `B8DebugGuestImageMappingReport` output は
  変えない。

PR に含めない:

- `MachOImage` domain model 本体抽出。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime `GuestImage` metadata collections shell 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2m Runtime MachOImage Domain Shell

branch: `task/b8-arch2m-mach-o-image-domain-shell`

B8-ARCH2l が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `MachOImage` domain shell を追加する。
これは `GuestImage` / `GuestImageMetadata` を Mach-O specific image model から参照できるようにする
前段であり、B8 debug bundle は existing `loader.plan.json` output を維持したまま
`MachOImage` shell 経由で image mapping report を作る。

完了条件:

- [x] `bara-runtime` に `MachOImage` shell を追加し、valid `GuestImage` と
  `GuestImageMetadata` を read-only に参照できる。
- [x] B8 debug bundle の image mapping projection は `GuestImage` 直ではなく
  `MachOImage` shell を作ってから existing `B8DebugGuestImageMappingReport` へ射影する。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、import/fixup/symbol projection の意味変更、
  helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::MachOImage` shell を追加し、`GuestImage` / `GuestImageMetadata` を
  read-only に参照できるようにした。
- B8 debug bundle は `MachOImage::executable` を作ってから existing
  `B8DebugGuestImageMappingReport::from_guest_image` へ射影する。
- existing `B8DebugGuestImageMappingReport` DTO と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime `MachOImage` domain shell 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2n Runtime MachOImage Code Range Constructor

branch: `task/b8-arch2n-mach-o-code-range-constructor`

B8-ARCH2m が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、Mach-O executable code range から runtime-facing
`MachOImage` を作る constructor を追加する。B8 debug bundle は existing
`MachOEntryFunctionInput` から `ProgramImageRange` と `GuestImageMetadata` を作るが、
Mach-O code segment の source / address-space 決定は runtime `MachOImage` 側へ寄せる。
existing `loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime::MachOImage` に executable code range から `GuestImageSegment` を作る
  constructor を追加し、Mach-O code segment source / address-space 決定を runtime 側に閉じる。
- [x] B8 debug bundle の image mapping projection は `GuestImageSegment` を直接作らず、
  `ProgramImageRange` と `GuestImageMetadata` から `MachOImage` を作ってから existing
  `B8DebugGuestImageMappingReport` へ射影する。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、import/fixup/symbol projection の意味変更、
  helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::MachOImage::executable_from_code_range` を追加し、Mach-O executable code
  segment の source / address-space 決定を runtime constructor 側に閉じた。
- B8 debug bundle は existing `MachOEntryFunctionInput` から `ProgramImageRange` と
  `GuestImageMetadata` を作り、`MachOImage::executable_from_code_range` を通して existing
  `B8DebugGuestImageMappingReport::from_guest_image` へ射影する。
- existing `B8DebugGuestImageMappingReport` DTO と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime `MachOImage` code range constructor 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2o Runtime MachOImage Program Metadata Constructor

branch: `task/b8-arch2o-mach-o-program-metadata-constructor`

B8-ARCH2n が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`ProgramImageMetadata` から runtime-facing
`GuestImageMetadata` を作る判断を `MachOImage` constructor 側へ寄せる。B8 debug bundle は
existing `MachOEntryFunctionInput` から `ProgramImageRange` と `ProgramImageMetadata` を
渡すが、mapped bytes source と `GuestImageMetadata` assembly は runtime image shell 側に閉じる。
existing `loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime::MachOImage` に `ProgramImageMetadata` から executable image を作る
  constructor を追加し、`GuestImageMappedBytesSource::ProgramImageMetadata` の選択を
  runtime 側に閉じる。
- [x] B8 debug bundle の image mapping projection は `GuestImageMetadata` を直接作らず、
  `ProgramImageRange` と `ProgramImageMetadata` から `MachOImage` を作ってから existing
  `B8DebugGuestImageMappingReport` へ射影する。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、import/fixup/symbol projection の意味変更、
  helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::MachOImage::executable_from_program_image_metadata` を追加し、
  `GuestImageMappedBytesSource::ProgramImageMetadata` の選択と `GuestImageMetadata` assembly を
  runtime constructor 側に閉じた。
- B8 debug bundle は existing `MachOEntryFunctionInput` から `ProgramImageRange` と
  `ProgramImageMetadata` を渡し、`MachOImage::executable_from_program_image_metadata` 経由で
  existing `B8DebugGuestImageMappingReport::from_guest_image` へ射影する。
- existing `B8DebugGuestImageMappingReport` DTO と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime `MachOImage` program metadata constructor 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2p Runtime MachO Executable Code Range Domain Type

branch: `task/b8-arch2p-mach-o-code-range-domain-type`

B8-ARCH2o が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `MachOImage` constructor の executable code
range を汎用 `ProgramImageRange` ではなく Mach-O specific domain type で表現する。B8 debug
bundle は existing `MachOEntryFunctionInput` から code range を計算して domain type に変換するが、
loader parsing / materialization は移動しない。existing `loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に Mach-O executable code range domain type を追加し、runtime
  `MachOImage` constructor は汎用 `ProgramImageRange` ではなくその型を受け取る。
- [x] B8 debug bundle の image mapping projection は calculated `ProgramImageRange` を
  Mach-O code range domain type に変換してから `MachOImage` を作る。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、code range 計算、import/fixup/symbol projection の
  意味変更、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::MachOExecutableCodeRange` を追加し、`MachOImage` の code range constructor は
  汎用 `ProgramImageRange` ではなく Mach-O specific code range domain type を受け取る。
- B8 debug bundle は existing `MachOEntryFunctionInput` から calculated `ProgramImageRange` を
  作り、`MachOExecutableCodeRange` に変換してから
  `MachOImage::executable_from_program_image_metadata` へ渡す。
- existing `B8DebugGuestImageMappingReport` DTO と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- code range 計算の runtime への移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O executable code range domain type 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2q Runtime MachO Code Section Range Constructor

branch: `task/b8-arch2q-mach-o-code-section-range-constructor`

B8-ARCH2p が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `MachOImage` constructor が executable
code range を caller 計算値ではなく `ProgramImageMetadata.sections()` の code section から
選ぶようにする。B8 debug bundle は existing `MachOEntryFunctionInput` から code range を
計算せず、entry point と `ProgramImageMetadata` を runtime constructor へ渡す。
existing `loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime::MachOExecutableCodeRange` が `ProgramImageMetadata` から単一の
  `ProgramImageSectionKind::Code` range を選び、missing / ambiguous code section を
  classified `GuestImageError` として返す。
- [x] `bara-runtime::MachOImage::executable_from_program_image_metadata` は caller から
  `MachOExecutableCodeRange` を受け取らず、`ProgramImageMetadata` 内の code section から
  executable code range を作る。
- [x] B8 debug bundle の image mapping projection は code bytes length から
  `ProgramImageRange` を計算せず、entry point と `ProgramImageMetadata` だけで
  `MachOImage` を作る。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection の意味変更、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `MachOExecutableCodeRange::from_program_image_metadata` を追加し、single code section
  range を runtime-facing Mach-O domain type へ変換する。code section missing /
  ambiguous cases は `GuestImageError` として test coverage を追加した。
- `MachOImage::executable_from_program_image_metadata` は `entry_point` と
  `ProgramImageMetadata` だけを受け取り、code range selection と
  `GuestImageMetadata` assembly を runtime constructor 側に閉じる。
- B8 debug bundle は existing `MachOEntryFunctionInput` から calculated
  `ProgramImageRange` を作らず、existing `B8DebugGuestImageMappingReport` へ射影する。
  existing DTO と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O code section range constructor 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2r Runtime MachO Entry Point Domain Type

branch: `task/b8-arch2r-mach-o-entry-point-domain-type`

B8-ARCH2q が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `MachOImage` constructor の entry point を
generic `GuestImageEntryPoint` ではなく Mach-O specific domain type で表現する。
B8 debug bundle は existing `MachOEntryFunctionInput` から entry address を
`MachOExecutableEntryPoint` に変換して渡すが、entry extraction / loader parsing /
materialization は移動しない。existing `loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に Mach-O executable entry point domain type を追加し、runtime
  `MachOImage` constructor は generic `GuestImageEntryPoint` ではなくその型を受け取る。
- [x] `MachOImage::entry_point` は Mach-O specific entry point type を返し、underlying
  `GuestImage` への変換は `MachOImage` constructor 内に閉じる。
- [x] B8 debug bundle の image mapping projection は existing entry address を
  `MachOExecutableEntryPoint` に変換してから `MachOImage` を作る。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、entry extraction、public Mach-O parser /
  resolver logic、import/fixup/symbol projection の意味変更、helper bridge、
  runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::MachOExecutableEntryPoint` を追加し、Mach-O executable entry point address
  を runtime-facing domain type として表す。
- `MachOImage::executable`、`MachOImage::executable_from_code_range`、
  `MachOImage::executable_from_program_image_metadata` は generic `GuestImageEntryPoint` ではなく
  `MachOExecutableEntryPoint` を受け取る。
- B8 debug bundle は existing `MachOEntryFunctionInput` の entry address を
  `MachOExecutableEntryPoint` に変換し、existing `B8DebugGuestImageMappingReport` へ射影する。
  existing DTO と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O entry point domain type 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2s Runtime MachO Code Segment Domain Type

branch: `task/b8-arch2s-mach-o-code-segment-domain-type`

B8-ARCH2r が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `MachOImage` の executable code segment を
generic `GuestImageSegment` ではなく Mach-O specific domain type で表現する。
`MachOImage` constructor は Mach-O executable code segment type を受け取り、underlying
`GuestImageSegment` への変換を constructor 内に閉じる。B8 debug bundle は existing
`GuestImage` projection と `loader.plan.json` output を維持する。

完了条件:

- [x] `bara-runtime` に Mach-O executable code segment domain type を追加し、runtime
  `MachOImage` constructor は generic `GuestImageSegment` ではなくその型を受け取る。
- [x] `MachOImage::code_segment` は Mach-O specific code segment type を返し、underlying
  `GuestImageSegment` への変換は `MachOImage` constructor 内に閉じる。
- [x] `MachOImage::executable_from_code_range` と
  `MachOImage::executable_from_program_image_metadata` は Mach-O code segment type 経由で
  generic guest image segment を作る。
- [x] B8 debug bundle の image mapping projection と `loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection の意味変更、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::MachOExecutableCodeSegment` を追加し、Mach-O executable code segment の
  range、source、address-space を runtime-facing domain type として表す。
- `MachOImage::executable` は generic `GuestImageSegment` ではなく
  `MachOExecutableCodeSegment` を受け取り、underlying `GuestImageSegment` への変換を
  constructor 内に閉じる。
- `MachOImage::code_segment` は `Option<GuestImageSegment>` ではなく
  `MachOExecutableCodeSegment` を返す。`MachOImage` が valid code segment を持つ invariant を
  API に反映する。
- B8 debug bundle は existing `MachOEntryFunctionInput` から `MachOImage` を作り、
  existing `GuestImage` projection 経由で `B8DebugGuestImageMappingReport` へ射影する。
  existing DTO と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O code segment domain type 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2t Runtime GuestImage Mapped Bytes Value Object

branch: `task/b8-arch2t-guest-image-mapped-bytes-value`

B8-ARCH2s が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImageMetadata` の mapped bytes を
source と payload のばら field ではなく value object として保持する。B8 debug bundle は
existing `MachOEntryFunctionInput` から `MachOImage` を作り、existing `GuestImage`
projection 経由で `B8DebugGuestImageMappingReport` へ射影する。existing
`loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に `GuestImageMappedBytes` value object を追加し、mapped bytes source と
  `ProgramImageMappedBytes` payload を一体の runtime-facing value として扱う。
- [x] `GuestImageMetadata` は `mapped_bytes_source` と `ProgramImageMappedBytes` を別々に
  constructor へ受け取らず、`GuestImageMappedBytes` を受け取って保持する。
- [x] `GuestImage` / `GuestImageMetadata` の existing `mapped_bytes_source()` と
  `mapped_bytes()` accessor は維持し、既存 caller の JSON projection は変えない。
- [x] B8 debug bundle の image mapping projection と `loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection の意味変更、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::GuestImageMappedBytes` を追加し、mapped bytes source と
  `ProgramImageMappedBytes` payload を一体の runtime-facing value object として表す。
- `GuestImageMetadata::new` は `GuestImageMappedBytesSource` と `ProgramImageMappedBytes` を
  別々に受け取らず、`GuestImageMappedBytes` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageMappedBytes::from_program_image_metadata` 経由で source selection と payload clone を
  value object 側に閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `mapped_bytes_source()` と
  `mapped_bytes()` accessor は維持し、B8 debug bundle の existing
  `B8DebugGuestImageMappingReport` projection と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage mapped bytes value object 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2u Runtime GuestImage Sections Value Object

branch: `task/b8-arch2u-guest-image-sections-value`

B8-ARCH2t が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImageMetadata` の sections を
`ProgramImageSections` 直持ちではなく value object として保持する。B8 debug bundle は
existing `MachOEntryFunctionInput` から `MachOImage` を作り、existing `GuestImage`
projection 経由で `B8DebugGuestImageMappingReport` へ射影する。existing
`loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に `GuestImageSections` value object を追加し、
  `ProgramImageSections` payload を runtime-facing value として扱う。
- [x] `GuestImageMetadata` は `ProgramImageSections` を直接 constructor へ受け取らず、
  `GuestImageSections` を受け取って保持する。
- [x] `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageSections::from_program_image_metadata` 経由で sections clone を value object 側に
  閉じる。
- [x] `GuestImage` / `GuestImageMetadata` の existing `sections()` accessor は維持し、
  既存 caller の JSON projection は変えない。
- [x] B8 debug bundle の image mapping projection と `loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection の意味変更、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- `bara-runtime::GuestImageSections` を追加し、`ProgramImageSections` payload を
  runtime-facing value object として表す。
- `GuestImageMetadata::new` は `ProgramImageSections` を直接受け取らず、
  `GuestImageSections` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageSections::from_program_image_metadata` 経由で sections clone を value object 側に
  閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `sections()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage sections value object 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2v Runtime GuestImage Symbols Value Object

branch: `task/b8-arch2v-guest-image-symbols-value`

B8-ARCH2u が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImageMetadata` の symbols を
`ProgramImageSymbols` 直持ちではなく value object として保持する。B8 debug bundle は
existing `MachOEntryFunctionInput` から `MachOImage` を作り、existing `GuestImage`
projection 経由で `B8DebugGuestImageMappingReport` へ射影する。existing
`loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に `GuestImageSymbols` value object を追加し、
  `ProgramImageSymbols` payload を runtime-facing value として扱う。
- [x] `GuestImageMetadata` は `ProgramImageSymbols` を直接 constructor へ受け取らず、
  `GuestImageSymbols` を受け取って保持する。
- [x] `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageSymbols::from_program_image_metadata` 経由で symbols clone を value object 側に
  閉じる。
- [x] `GuestImage` / `GuestImageMetadata` の existing `symbols()` accessor は維持し、
  既存 caller の JSON projection は変えない。
- [x] B8 debug bundle の image mapping projection と `loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection の意味変更、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- 意図: symbols payload を `GuestImageMetadata` のばら primitive collection としてではなく、
  runtime-facing value object として扱い、後続の symbol identity / import projection 境界を
  意味変更なしに切り出しやすくする。
- できるようになったこと: `GuestImageMetadata` construction では `ProgramImageSymbols` を
  直接受け取らず、`GuestImageSymbols` を受け取って保持できる。
- `bara-runtime::GuestImageSymbols` を追加し、`ProgramImageSymbols` payload を
  runtime-facing value object として表す。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageSymbols::from_program_image_metadata` 経由で symbols clone を value object 側に
  閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `symbols()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage symbols value object 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2w Runtime GuestImage Unwind Metadata Value Object

branch: `task/b8-arch2w-guest-image-unwind-value`

B8-ARCH2v が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImageMetadata` の unwind metadata を
`ProgramUnwindMetadata` 直持ちではなく value object として保持する。B8 debug bundle は
existing `MachOEntryFunctionInput` から `MachOImage` を作り、existing `GuestImage`
projection 経由で `B8DebugGuestImageMappingReport` へ射影する。existing
`loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に `GuestImageUnwindMetadata` value object を追加し、
  `ProgramUnwindMetadata` payload を runtime-facing value として扱う。
- [x] `GuestImageMetadata` は `ProgramUnwindMetadata` を直接 constructor へ受け取らず、
  `GuestImageUnwindMetadata` を受け取って保持する。
- [x] `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageUnwindMetadata::from_program_image_metadata` 経由で unwind clone を value object 側に
  閉じる。
- [x] `GuestImage` / `GuestImageMetadata` の existing `unwind()` accessor は維持し、
  既存 caller の JSON projection は変えない。
- [x] B8 debug bundle の image mapping projection と `loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection の意味変更、helper bridge、runtime dispatcher は移動しない。

completion evidence:

- 意図: unwind metadata payload を `GuestImageMetadata` の direct collection field から分け、
  loader / exception 関連 metadata を後続で扱う場合の runtime-facing 境界を先に作る。
- できるようになったこと: `GuestImageMetadata` construction では `ProgramUnwindMetadata` を
  直接受け取らず、`GuestImageUnwindMetadata` を受け取って保持できる。
- `bara-runtime::GuestImageUnwindMetadata` を追加し、`ProgramUnwindMetadata` payload を
  runtime-facing value object として表す。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageUnwindMetadata::from_program_image_metadata` 経由で unwind clone を value object 側に
  閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `unwind()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage unwind metadata value object 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2x Runtime GuestImage Imports Value Object

branch: `task/b8-arch2x-guest-image-imports-value`

B8-ARCH2w が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImageMetadata` の imports を
`ProgramImageImports` 直持ちではなく value object として保持する。B8 debug bundle は
existing `MachOEntryFunctionInput` から `MachOImage` を作り、existing `GuestImage`
projection 経由で `B8DebugGuestImageMappingReport` へ射影する。existing
`loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に `GuestImageImports` value object を追加し、
  `ProgramImageImports` payload を runtime-facing value として扱う。
- [x] `GuestImageMetadata` は `ProgramImageImports` を直接 constructor へ受け取らず、
  `GuestImageImports` を受け取って保持する。
- [x] `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageImports::from_program_image_metadata` 経由で imports clone を value object 側に
  閉じる。
- [x] `GuestImage` / `GuestImageMetadata` の existing `imports()` accessor は維持し、
  既存 caller の JSON projection は変えない。
- [x] B8 debug bundle の image mapping projection と `loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import projection semantics の意味変更、fixup/symbol projection の意味変更、
  helper bridge、runtime dispatcher は移動しない。

completion evidence:

- 意図: imports payload を `GuestImageMetadata` の direct collection field から分け、
  import projection semantics を変えずに後続の loader/import 境界を扱いやすくする。
- できるようになったこと: `GuestImageMetadata` construction では `ProgramImageImports` を
  直接受け取らず、`GuestImageImports` を受け取って保持できる。
- `bara-runtime::GuestImageImports` を追加し、`ProgramImageImports` payload を
  runtime-facing value object として表す。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageImports::from_program_image_metadata` 経由で imports clone を value object 側に
  閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `imports()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import projection semantics の意味変更または schema 変更。
- fixup/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage imports value object 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2y Runtime GuestImage Relocations Value Object

branch: `task/b8-arch2y-guest-image-relocations-value`

B8-ARCH2x が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、runtime-facing `GuestImageMetadata` の relocations を
`ProgramImageRelocations` 直持ちではなく value object として保持する。B8 debug bundle は
existing `MachOEntryFunctionInput` から `MachOImage` を作り、existing `GuestImage`
projection 経由で `B8DebugGuestImageMappingReport` へ射影する。existing
`loader.plan.json` output は維持する。

完了条件:

- [x] `bara-runtime` に `GuestImageRelocations` value object を追加し、
  `ProgramImageRelocations` payload を runtime-facing value として扱う。
- [x] `GuestImageMetadata` は `ProgramImageRelocations` を直接 constructor へ受け取らず、
  `GuestImageRelocations` を受け取って保持する。
- [x] `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageRelocations::from_program_image_metadata` 経由で relocations clone を
  value object 側に閉じる。
- [x] `GuestImage` / `GuestImageMetadata` の existing `relocations()` accessor は維持し、
  既存 caller の JSON projection は変えない。
- [x] B8 debug bundle の image mapping projection と `loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  relocation/fixup projection semantics の意味変更、import/symbol projection の意味変更、
  helper bridge、runtime dispatcher は移動しない。

completion evidence:

- 意図: relocations payload を `GuestImageMetadata` の direct collection field から分け、
  relocation/fixup projection semantics を変えずに後続の loader/fixup 境界を扱いやすくする。
- できるようになったこと: `GuestImageMetadata` construction では `ProgramImageRelocations` を
  直接受け取らず、`GuestImageRelocations` を受け取って保持できる。
- `bara-runtime::GuestImageRelocations` を追加し、`ProgramImageRelocations` payload を
  runtime-facing value object として表す。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageRelocations::from_program_image_metadata` 経由で relocations clone を value object 側に
  閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `relocations()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- relocation/fixup projection semantics の意味変更または schema 変更。
- import/symbol projection の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage relocations value object 接続を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2z Runtime GuestImage Metadata Module Split

branch: `task/b8-arch2z-guest-image-metadata-module-split`

B8-ARCH2y が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、肥大化した `bara-runtime::guest_image` module から
`GuestImageMetadata` aggregate と metadata value object 群を専用 module へ分ける。
B8 debug bundle は existing `GuestImage` projection 経由で
`B8DebugGuestImageMappingReport` へ射影し、existing `loader.plan.json` output は維持する。

完了条件:

- [x] `crates/bara-runtime/src/guest_image/metadata.rs` を追加し、
  `GuestImageMetadata`、`GuestImageMappedBytes`、`GuestImageMappedBytesSource`、
  `GuestImageSections`、`GuestImageImports`、`GuestImageRelocations`、
  `GuestImageSymbols`、`GuestImageUnwindMetadata` を移す。
- [x] `guest_image/mod.rs` は metadata module を re-export し、existing public API 名を維持する。
- [x] `GuestImage` / `MachOImage` / B8 debug bundle の caller-visible behavior と JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: `GuestImage` / `MachOImage` の image shell と metadata aggregate / value object 群の
  変更理由を分け、`guest_image/mod.rs` の肥大化を抑える。
- できるようになったこと: metadata payload 境界の追加や調整を
  `guest_image/metadata.rs` で扱えるようになり、core image shell 側を変更せずに
  metadata aggregate / value object 群を見直せる。
- `guest_image/mod.rs` は `metadata` module を re-export し、
  `bara_runtime::GuestImageMetadata` などの existing public API 名を維持する。
- `GuestImage` / `MachOImage` / B8 debug bundle の caller-visible behavior と
  `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage metadata module split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2aa Runtime MachOImage Module Split

branch: `task/b8-arch2aa-macho-image-module-split`

B8-ARCH2z が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`bara-runtime::guest_image` module から Mach-O specific
image shell と executable entry / code range / code segment value object 群を専用 module へ
分ける。B8 debug bundle は existing `MachOImage` / `GuestImage` projection 経由で
`B8DebugGuestImageMappingReport` へ射影し、existing `loader.plan.json` output は維持する。

完了条件:

- [x] `crates/bara-runtime/src/guest_image/mach_o.rs` を追加し、`MachOImage`、
  `MachOExecutableEntryPoint`、`MachOExecutableCodeRange`、
  `MachOExecutableCodeSegment` を移す。
- [x] `guest_image/mod.rs` は Mach-O module を re-export し、existing public API 名を維持する。
- [x] generic `GuestImage` shell は Mach-O specific constructor detail を呼び出し側 module へ
  必要最小限だけ公開し、caller-visible behavior と JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: generic `GuestImage` shell と Mach-O specific `MachOImage` shell の変更理由を分け、
  `guest_image/mod.rs` が Mach-O constructor / executable code range の詳細で肥大化しないようにする。
- できるようになったこと: Mach-O executable entry / code range / code segment の調整を
  `guest_image/mach_o.rs` で扱えるようになり、generic image shell 側を変更せずに
  Mach-O specific constructor boundary を見直せる。
- `guest_image/mod.rs` は `mach_o` module を re-export し、
  `bara_runtime::MachOImage` などの existing public API 名を維持する。
- `GuestImageSegment::mach_o_executable_code` を generic shell から外し、
  Mach-O executable code segment assembly は `MachOExecutableCodeSegment::new` 側に閉じる。
- `GuestImage` / `MachOImage` / B8 debug bundle の caller-visible behavior と
  `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime MachOImage module split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ab Runtime GuestImage Core Module Split

branch: `task/b8-arch2ab-guest-image-core-module-split`

B8-ARCH2aa が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`bara-runtime::guest_image` module から generic
`GuestImage` shell、entry point、segments、segment identity、image error を専用 module へ
分ける。B8 debug bundle は existing `MachOImage` / `GuestImage` projection 経由で
`B8DebugGuestImageMappingReport` へ射影し、existing `loader.plan.json` output は維持する。

完了条件:

- [x] `crates/bara-runtime/src/guest_image/image.rs` を追加し、`GuestImage`、
  `GuestImageFormat`、`GuestImageEntryPoint`、`GuestImageSegments`、
  `GuestImageSegment`、`GuestImageSegmentKind`、`GuestImageSegmentSource`、
  `GuestImageAddressSpace`、`GuestImageError` を移す。
- [x] `guest_image/mod.rs` は generic image module を re-export し、existing public API 名を維持する。
- [x] `metadata.rs` と `mach_o.rs` は existing public API / caller-visible behavior を維持したまま
  generic image shell を module boundary 経由で使う。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: generic `GuestImage` shell と親 `guest_image/mod.rs` の変更理由を分け、
  parent module を submodule wiring / re-export / tests に近づける。
- できるようになったこと: generic image invariant、entry point、segment identity、image error の
  調整を `guest_image/image.rs` で扱えるようになり、metadata / Mach-O specific module と
  親 module を変更せずに generic image shell を見直せる。
- `guest_image/mod.rs` は `image` module を re-export し、
  `bara_runtime::GuestImage` などの existing public API 名を維持する。
- `metadata.rs` と `mach_o.rs` は existing public API / caller-visible behavior を維持したまま、
  generic image shell を module boundary 経由で使う。
- `GuestImage` / `MachOImage` / B8 debug bundle の caller-visible behavior と
  `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage core module split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ac Runtime GuestImage Test Module Split

branch: `task/b8-arch2ac-guest-image-test-module-split`

B8-ARCH2ab が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`bara-runtime::guest_image` parent module に残った unit test
群を専用 test module へ分ける。production behavior、public API、B8 debug bundle の
`loader.plan.json` output は維持する。

完了条件:

- [x] `crates/bara-runtime/src/guest_image/tests.rs` を追加し、existing `guest_image` unit test
  群を `mod.rs` から移す。
- [x] `guest_image/mod.rs` は submodule wiring / re-export / test module declaration に寄せ、
  production type definitions を持たない。
- [x] existing `guest_image` tests の coverage と names を維持し、caller-visible behavior と
  JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: `guest_image/mod.rs` に残った unit test 群を分け、parent module を submodule wiring /
  re-export / test module declaration に近づける。
- できるようになったこと: production module wiring と test fixture / regression coverage の
  変更理由を分けられるようになり、`guest_image` 親 module の diff から production boundary を
  読み取りやすくなった。
- `crates/bara-runtime/src/guest_image/tests.rs` を追加し、existing `guest_image` unit test 群を
  移した。
- `guest_image/mod.rs` は production type definitions を持たず、`image` / `mach_o` /
  `metadata` module wiring と public re-export、test module declaration だけを持つ。
- existing `guest_image` tests の names と coverage、caller-visible behavior、
  `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage test module split を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ad Debug Guest Image MachO Projection Boundary

branch: `task/b8-arch2ad-debug-guest-image-macho-projection`

B8-ARCH2ac が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、B8 debug bundle の `image_mapping` report projection を
generic `GuestImage::code_segment()` lookup ではなく runtime の typed `MachOImage`
boundary から読むように寄せる。production behavior、public API、B8 debug bundle の
`loader.plan.json` output は維持する。

完了条件:

- [x] `B8DebugGuestImageMappingReport` に focused regression test を追加し、typed
  `MachOImage` code segment から `image_mapping` report が作られることを固定する。
- [x] `B8DebugGuestImageMappingReport::from_entry_input` は `MachOImage` を構築したあと、
  generic `GuestImage::code_segment()` ではなく `MachOImage::code_segment()` から report を
  組み立てる。
- [x] runtime `MachOImage` が code segment を non-optional に保持している前提を caller 側で使い、
  debug bundle 固有の impossible `MissingCodeSegment` error branch を削る。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: B8 debug bundle の `image_mapping` projection が runtime の typed `MachOImage`
  boundary を直接使うようにし、generic image invariant の再検証を CLI report DTO 側から外す。
- できるようになったこと: `MachOImage::code_segment()` の non-optional contract を caller 側で
  利用できるようになり、debug report projection から存在しない code segment 欠落 path を
  読まなくてよくなった。
- `crates/btbc-cli/src/b8_debug_bundle/guest_image.rs` に
  `image_mapping_report_uses_typed_mach_o_code_segment` を追加した。
- `B8DebugGuestImageMappingReport::from_entry_input` は `MachOImage` から report を組み立て、
  `GuestImage::code_segment()` lookup と `MissingCodeSegment` error branch を使わない。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli image_mapping_report_uses_typed_mach_o_code_segment -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- debug guest image Mach-O projection boundary を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ae Runtime MachO Code Segment Derived Accessors

branch: `task/b8-arch2ae-macho-code-segment-accessors`

B8-ARCH2ad が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、Mach-O executable code segment の vmaddr / byte length
derivation を B8 debug bundle DTO ではなく runtime domain type 側へ寄せる。
production behavior、public API 互換、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `MachOExecutableCodeSegment` に focused regression test を追加し、vmaddr と typed byte
  length が code segment range から導出されることを固定する。
- [x] `MachOExecutableCodeByteLen` value object を追加し、byte length の primitive `usize`
  変換は accessor に閉じる。
- [x] `MachOExecutableCodeSegment` は `vmaddr()` と `byte_len()` を公開し、range の start /
  end 差分計算と overflow classification を runtime domain 側で扱う。
- [x] `B8DebugGuestImageMappingReport` は code segment range を直接ほどかず、
  `MachOExecutableCodeSegment` の derived accessor を使う。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: Mach-O executable code segment から計算できる mapping 値を runtime domain 側に寄せ、
  CLI DTO が `ProgramImageRange` の start / end を直接解釈しないようにする。
- できるようになったこと: caller は `MachOExecutableCodeSegment::vmaddr()` と typed
  `MachOExecutableCodeSegment::byte_len()` を使えるようになり、byte length overflow も
  `GuestImageError` として runtime boundary で分類できる。
- `MachOExecutableCodeByteLen` を追加し、JSON DTO 用の primitive `usize` 変換は
  `as_usize()` accessor に閉じた。
- `as_usize()` は serialization DTO 境界で必要な deliberate primitive accessor として
  `docs/domain-primitive-baseline.txt` に追加した。
- `B8DebugGuestImageMappingReport` は `code_segment.range().range()` を使わず、
  `vmaddr()` と `byte_len()` を使って existing `image_mapping` JSON を組み立てる。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime mach_o_executable_code_segment_exposes_derived_mapping_values -- --nocapture`
- `nix develop -c cargo test -p btbc-cli image_mapping_report_uses_typed_mach_o_code_segment -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O code segment derived accessors を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2af Runtime MachO Image Mapping Snapshot

branch: `task/b8-arch2af-macho-image-mapping-snapshot`

B8-ARCH2ae が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、B8 debug bundle の `image_mapping` projection が必要とする
runtime mapping 構成を `MachOImage` から直接ばらばらに読むのではなく、runtime の typed
mapping snapshot として取得できるようにする。production behavior、public API 互換、
B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `MachOExecutableImageMapping` domain snapshot を追加し、Mach-O executable image の
  code segment、entry point、mapped bytes source を typed value として保持する。
- [x] `MachOImage` は `executable_mapping()` で `MachOExecutableImageMapping` を返す。
- [x] `B8DebugGuestImageMappingReport` は `MachOImage` の metadata / entry / code segment を
  個別に読むのではなく、`MachOExecutableImageMapping` から report を組み立てる。
- [x] `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: B8 debug bundle の `image_mapping` projection が必要とする runtime mapping 構成を
  `MachOImage` の内部 accessor 群から直接集めるのではなく、runtime domain の snapshot として
  受け取れるようにする。
- できるようになったこと: caller は `MachOExecutableImageMapping` を単位に code segment、
  entry point、mapped bytes source を扱えるようになり、debug DTO projection の理由で
  `MachOImage` の metadata 構造を知る必要がなくなる。
- `MachOExecutableImageMapping` を追加し、`MachOImage::executable_mapping()` から取得できる
  ようにした。crate root からも re-export する。
- `B8DebugGuestImageMappingReport` は `MachOExecutableImageMapping` から existing
  `image_mapping` JSON を組み立てる。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime mach_o_image_exposes_executable_mapping_snapshot -- --nocapture`
- `nix develop -c cargo test -p btbc-cli image_mapping_report_uses_typed_mach_o_code_segment -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O image mapping snapshot を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ag Runtime MachO Mapping Mapped Bytes Snapshot

branch: `task/b8-arch2ag-macho-mapping-bytes-payload`

B8-ARCH2af が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`MachOExecutableImageMapping` が mapped bytes source だけでなく
runtime-facing `GuestImageMappedBytes` value object も保持するようにする。production behavior、
existing report projection、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `MachOExecutableImageMapping` に focused regression test を追加し、snapshot が
  `GuestImageMappedBytes` value object と payload を保持することを固定する。
- [x] `MachOImage::executable_mapping()` は `GuestImageMetadata` から mapped bytes value object を
  clone して snapshot に渡す。
- [x] 既存 caller 用の `mapped_bytes_source()` は維持し、source は snapshot 内の
  `GuestImageMappedBytes` から導出する。
- [x] `B8DebugGuestImageMappingReport` の `loader.plan.json` output は mapped bytes source を
  既存と同じ値で report し、field 名、nested field 名、serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: B8 debug bundle と後続 runtime loader caller が mapping snapshot から mapped bytes
  value object まで読めるようにし、debug DTO projection の理由で `MachOImage` / metadata
  内部構造へ戻らない境界を強める。
- できるようになったこと: caller は `MachOExecutableImageMapping::mapped_bytes()` で
  `GuestImageMappedBytes` を参照し、source と payload を同じ runtime-facing snapshot から扱える。
- `MachOExecutableImageMapping::mapped_bytes_source()` は残し、既存 report DTO は source projection
  のために同じ API を使える。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime mach_o_executable_mapping_carries_mapped_bytes_value -- --nocapture`
- `nix develop -c cargo test -p btbc-cli image_mapping_report_uses_typed_mach_o_code_segment -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O mapping mapped bytes snapshot を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ah Runtime GuestImage Metadata Value Accessors

branch: `task/b8-arch2ah-guest-image-metadata-value-accessors`

B8-ARCH2ag が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`GuestImageMetadata` が mapped bytes 以外の metadata collection も
runtime-facing value object として返せるようにする。production behavior、existing payload
accessor、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `GuestImageMetadata` に focused regression test を追加し、sections / symbols /
  relocations / imports / unwind を value object として取得できることを固定する。
- [x] `GuestImageMetadata` は `sections_value()`、`symbols_value()`、
  `relocations_value()`、`imports_value()`、`unwind_value()` を公開する。
- [x] 既存 caller 用の `sections()` / `symbols()` / `relocations()` / `imports()` /
  `unwind()` payload accessor は維持する。
- [x] B8 debug bundle の `loader.plan.json` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: 後続 runtime loader caller が `GuestImageMetadata` から metadata collection を
  payload primitive ではなく runtime-facing value object として受け取れるようにし、
  B8-ARCH2 の image model 境界を強める。
- できるようになったこと: caller は `GuestImageMetadata::*_value()` で mapped bytes /
  sections / symbols / relocations / imports / unwind を同じ value object 境界から扱える。
- 既存 payload accessor は残し、existing B8 debug bundle behavior と `loader.plan.json` output は
  変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime guest_image_metadata_exposes_metadata_value_objects -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime GuestImage metadata value accessors を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ai Runtime MachO Executable Metadata Snapshot

branch: `task/b8-arch2ai-macho-metadata-snapshot`

B8-ARCH2ah が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`MachOImage` から Mach-O executable metadata snapshot を取得し、
metadata collection を Mach-O specific boundary から runtime-facing value object として読めるようにする。
production behavior、existing payload accessor、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `MachOExecutableImageMetadata` に focused regression test を追加し、Mach-O image から
  mapped bytes / sections / symbols / relocations / imports / unwind value object を取得できることを固定する。
- [x] `MachOImage` は `executable_metadata()` で `MachOExecutableImageMetadata` を返す。
- [x] `MachOExecutableImageMetadata` は `GuestImageMetadata` value object accessor 境界を使い、
  payload primitive を直接公開するための新しい API を増やさない。
- [x] B8 debug bundle の `loader.plan.json` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: 後続の import / relocation / symbol / unwind projection が generic metadata 内部ではなく
  Mach-O executable image snapshot を入口にできるようにし、B8-ARCH2 の Mach-O specific
  image model 境界を強める。
- できるようになったこと: caller は `MachOImage::executable_metadata()` から
  `MachOExecutableImageMetadata` を取得し、mapped bytes / sections / symbols / relocations /
  imports / unwind を Mach-O specific snapshot 経由で扱える。
- 既存 payload accessor は残し、existing B8 debug bundle behavior と `loader.plan.json` output は
  変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime mach_o_image_exposes_executable_metadata_snapshot -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O executable metadata snapshot を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2aj Runtime MachO Executable Image Snapshot

branch: `task/b8-arch2aj-macho-executable-image-snapshot`

B8-ARCH2ai が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、Mach-O executable mapping snapshot と metadata snapshot を
単一の executable image snapshot から取得できるようにする。B8 debug bundle はこの snapshot
入口から existing `image_mapping` report を組み立てるが、`loader.plan.json` output は維持する。

完了条件:

- [x] `MachOExecutableImageSnapshot` に focused regression test を追加し、Mach-O image から
  executable mapping と executable metadata の snapshot を同じ boundary で取得できることを固定する。
- [x] `MachOImage` は `executable_snapshot()` で `MachOExecutableImageSnapshot` を返す。
- [x] `MachOExecutableImageSnapshot` は mapping / metadata snapshot を返し、payload primitive を
  直接公開するための新しい API を増やさない。
- [x] B8 debug bundle の `image_mapping` projection は `MachOExecutableImageSnapshot` を入口にし、
  mapped bytes source は metadata snapshot 側の value object から読む。
- [x] B8 debug bundle の `loader.plan.json` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: 後続の import / relocation / symbol / unwind projection が mapping と metadata を別々の
  accessor から集めず、Mach-O executable image snapshot を単一の入口にできるようにし、
  B8-ARCH2 の image model 境界を強める。
- できるようになったこと: caller は `MachOImage::executable_snapshot()` から
  `MachOExecutableImageSnapshot` を取得し、mapping snapshot と metadata snapshot を同じ
  Mach-O specific boundary 経由で扱える。
- B8 debug bundle の通常経路は executable image snapshot から `image_mapping` report を
  組み立て、mapped bytes source は metadata snapshot の value object から読む。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime mach_o_image_exposes_executable_image_snapshot -- --nocapture`
- `nix develop -c cargo test -p btbc-cli image_mapping_report_uses_mach_o_executable_image_snapshot -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O executable image snapshot を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2ak Debug Loader MachO Snapshot Boundary

branch: `task/b8-arch2ak-loader-macho-snapshot-boundary`

B8-ARCH2aj が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、B8 debug bundle loader plan が `MachOEntryFunctionInput` から
Mach-O executable image snapshot を一度作り、その snapshot を image mapping projection へ渡す。
production behavior、existing public API、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `B8DebugLoaderPlanReport::real_lc_main_attempted` は
  `MachOExecutableImageSnapshot` を loader plan assembly の入口で構築する。
- [x] `B8DebugGuestImageMappingReport` は `MachOEntryFunctionInput` を直接受け取らず、
  borrowed `MachOExecutableImageSnapshot` から existing `image_mapping` report を組み立てる。
- [x] existing focused regression test は snapshot-borrowing projection を通し、
  `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output を
  維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: B8 debug bundle loader plan が Mach-O executable image snapshot を単一の入口として
  扱い始め、image mapping projection が entry input から image model を作り直さない境界にする。
- できるようになったこと: 後続の import / relocation / symbol / unwind projection は loader
  plan assembly で作った同じ snapshot を受け取る形へ段階的に移しやすくなる。
- `mach_o_executable_snapshot_from_entry_input` を追加し、entry input から runtime-facing
  `MachOExecutableImageSnapshot` を作る処理を `guest_image` projection module に閉じた。
- `B8DebugGuestImageMappingReport::from_mach_o_snapshot` は snapshot を借用し、loader plan は
  その snapshot から existing `image_mapping` JSON を組み立てる。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli image_mapping_report_uses_mach_o_executable_image_snapshot -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- debug loader Mach-O snapshot boundary を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2al Debug Import Boundary MachO Snapshot Boundary

branch: `task/b8-arch2al-import-boundary-macho-snapshot`

B8-ARCH2ak が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、B8 debug bundle loader plan で一度作った
`MachOExecutableImageSnapshot` を import boundary / helper boundary projection へ渡す。
production behavior、existing public API、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `B8DebugLoaderPlanReport::real_lc_main_attempted` は
  `B8DebugImportBoundaryReport` に borrowed `MachOExecutableImageSnapshot` を渡す。
- [x] `B8DebugImportBoundaryReport` と helper boundary request assembly は
  `MachOEntryFunctionInput` 由来の `ProgramImageMetadata` を loader から直接受け取らず、
  Mach-O executable image snapshot の metadata boundary を使う。
- [x] existing focused regression test は snapshot-backed import/helper projection を通し、
  `loader.plan.json` の `import_boundary` / `helper_boundary_request` field 名、nested field 名、
  serde 値、JSON output を維持する。
- [x] `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、helper bridge、runtime dispatcher は
  移動しない。

completion evidence:

- 意図: B8 debug bundle loader plan で作った `MachOExecutableImageSnapshot` を
  `image_mapping` だけでなく import boundary / helper boundary projection でも使い、
  loader plan assembly の image model 入口をさらに揃える。
- できるようになったこと: `B8DebugImportBoundaryReport::from_probe_and_decode_report` は
  loader から `ProgramImageMetadata` を直接受け取らず、borrowed
  `MachOExecutableImageSnapshot` を受け取る。helper boundary request は snapshot metadata から
  existing downstream materialization 用の `ProgramImageMetadata` view を作る。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。
- loader domain 抽出、public Mach-O parser / resolver logic、import/fixup/symbol projection
  semantics、helper bridge、runtime dispatcher は未移動。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry extraction / load command interpretation の runtime への移動。
- import/fixup/symbol projection semantics の意味変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher 抽出。
- translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- debug import boundary Mach-O snapshot boundary を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2am Runtime MachO Program Metadata View

branch: `task/b8-arch2am-macho-program-metadata-view`

B8-ARCH2al が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、Mach-O executable metadata snapshot から downstream
compatibility 用の `ProgramImageMetadata` view を runtime domain 側で組み立てる。
production behavior、existing public API、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `MachOExecutableImageMetadata` は typed metadata value object 群から
  `ProgramImageMetadata` compatibility view を返す。
- [x] B8 debug helper boundary は sections / mapped bytes / symbols / relocations / imports /
  unwind を CLI 側で手組みせず、Mach-O executable metadata snapshot の typed API を使う。
- [x] focused runtime regression test と existing B8 debug bundle regression test が
  metadata payload と existing JSON output の維持を検証する。
- [x] dependency、schema、import/fixup/symbol semantics、helper bridge、runtime dispatcher は
  変更しない。

completion evidence:

- 意図: B8-ARCH2al で snapshot-backed になった helper boundary から metadata aggregate の
  組み立て責務も runtime domain へ寄せ、CLI が Mach-O metadata value object の構成を
  知らなくてよい境界にする。
- できるようになったこと: `MachOExecutableImageMetadata::program_image_metadata()` が
  existing downstream materialization 用の compatibility view を返し、CLI の重複 assembly
  function を削除できる。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry code bytes、import/fixup/symbol projection semantics の変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher、translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime mach_o_executable_image_metadata_exposes_program_image_metadata_view -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O program metadata view を commit / push / draft PR 作成で停止する。

#### PR Gate: B8-ARCH2an Runtime MachO Snapshot Program Metadata View

branch: `task/b8-arch2an-snapshot-program-metadata-view`

B8-ARCH2am が review / merge 済みになるまで開始しない。B8-ARCH2 Guest Image Model
Extraction の次 slice として、`MachOExecutableImageSnapshot` 自体から downstream
compatibility 用の `ProgramImageMetadata` view を取得できるようにする。production behavior、
existing public API、B8 debug bundle の `loader.plan.json` output は維持する。

完了条件:

- [x] `MachOExecutableImageSnapshot` は nested metadata representation を caller に要求せず、
  `ProgramImageMetadata` compatibility view を返す。
- [x] B8 debug helper boundary は `snapshot.metadata()` を辿らず、snapshot-level typed API を使う。
- [x] focused runtime regression test と existing B8 debug bundle regression test が通る。
- [x] dependency、schema、metadata assembly semantics、helper bridge、runtime dispatcher は変更しない。

completion evidence:

- 意図: loader plan で一度作った Mach-O executable image snapshot を helper boundary の単一入口にし、
  caller が snapshot の内部構成を知る必要をさらに減らす。
- できるようになったこと: `MachOExecutableImageSnapshot::program_image_metadata()` が metadata
  snapshot の existing compatibility view へ委譲し、CLI は snapshot-level API だけを使う。
- existing B8 debug bundle behavior と `loader.plan.json` output は変えない。

PR に含めない:

- public Mach-O parser / resolver logic の `bara-oracle` からの移動。
- entry code bytes、import/fixup/symbol projection semantics の変更または schema 変更。
- helper boundary / Objective-C / AppKit helper bridge 一般化。
- return-to continuation dispatcher、translation artifact/cache/dispatcher 実装。

検証:

- `nix develop -c cargo test -p bara-runtime mach_o_executable_image_snapshot_exposes_program_image_metadata_view -- --nocapture`
- `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
- `nix develop -c ./scripts/verify`

review gate:

- runtime Mach-O snapshot program metadata view を commit / push / draft PR 作成で停止する。

#### Future Target: B8-ARCH2 Guest Image Model Extraction

- [ ] public Mach-O metadata から runtime が使う `GuestImage` / `MachOImage` domain model を
  切り出す。
- [ ] entry point、segments、mapped bytes、imports、relocations/fixups、symbol identity、
  unwind metadata を domain type で表現する。
- [ ] `bara-oracle` は external observation / expected generation / comparison の責務に寄せ、
  loader domain logic を分離する。
- [ ] B8 debug bundle は新しい image model API を使い、既存 JSON output を維持する。
- [ ] 将来 PE / ELF を同じ interface に載せる前提を document する。

#### Future Target: B8-ARCH3 Translation Artifact And Debug Export

- [ ] ARM64 block bytes、pcmap、fixups、helper requirements、source identity、
  cache validation identity を `TranslationArtifact` としてまとめる。
- [ ] translation artifact は内部 runtime / cache 用の domain object とし、ユーザー visible
  converted app output を主経路にしない。
- [ ] debug / review 用 export command で artifact bytes と metadata を保存できるようにする。
- [ ] simple fixture expected/actual は artifact path 経由でも通るようにする。
- [ ] B8-HWGUI では変換済み app ではなく artifact / launch report / debug bundle を見る、
  という運用を明文化する。

#### Future Target: B8-ARCH4 Runtime Dispatcher Foundation

- [ ] modeled continuation chain を、typed runtime state と dispatcher boundary へ移す。
- [ ] guest PC、register state、stack state、helper return state、host executable memory
  handle を domain type で表現する。
- [ ] direct fallthrough、direct call、return、helper return writeback の最小 path を
  dispatcher で扱う。
- [ ] indirect target、callback、exception、signal、thread、TLS は stable blocker として
  分類し、silent fallback しない。
- [ ] translation cache / fallback interpreter / JIT は dispatcher interface 上の
  future capability として定義する。

#### Future Target: B8-ARCH5 Helper And ABI Bridge Generalization

- [ ] B8 fixture 専用 Objective-C / AppKit helper を
  `GuestCall -> HostService -> GuestReturn` contract に一般化する。
- [ ] x86_64 macOS SysV argument materialization、return writeback、error classification を
  reusable helper bridge model にする。
- [ ] Objective-C helper、libSystem helper、future Wine thunk が同じ boundary model に載る
  ようにする。
- [ ] B8-HWGUI の `sharedApplication` / `setActivationPolicy:` / `setDelegate:` / `run` /
  autorelease pool は generic helper bridge 上の fixture-specific case に移す。

#### Future Target: B8-ARCH6 OS Personality Boundary

- [ ] core translator が OS を知らない boundary を固定する。
- [ ] macOS x86_64-on-macOS arm64 personality を最初の concrete personality として整理する。
- [ ] Linux x86_64-on-Linux arm64 と Windows x64-on-Wine は同じ interface の future
  personality として設計する。
- [ ] loader、ABI、syscall / libc / Objective-C / Win32 helper、TLS、thread、signal、
  exception の責務分担を document する。

#### Future Target: B8-OSS0 Source-Built OSS GUI App Automation

B8-HWGUI と B8-ARCH0 が review / merge 済みになるまで開始しない。B8-ARCH1 以降の
抽象化 milestone をどこまで先に進めるかは、review 後に判断する。

- [ ] public source から reproducible に x86_64 macOS binary を build できる、小さい OSS
  GUI app を候補にする。最初は任意の downloaded binary ではなく source-built fixture を
  優先する。
- [ ] license、再配布可否、build input、expected/actual/debug bundle 保存場所を scope
  document に固定する。
- [ ] OSS app 実行は B8-HWGUI の debug bundle / blocker-driven cycle を流用し、
  unsupported boundary を 1 つずつ stable report へ落とす。
- [ ] OSS app target の最初の作業は実装ではなく、候補選定、scope、success criteria、
  clean-room / supply-chain checklist の TODO 追加にする。

#### Future Target: B8-WINE0 Wine Bridge Planning

B8-ARCH1 以降で core / personality boundary が整理されるまで実装しない。

- [ ] Wine が担う PE loader / Windows API / registry / filesystem / windowing semantics と、
  Bara が担う x86_64 CPU/runtime backend の責務を分ける。
- [ ] Windows x64 ABI、guest callbacks、exception handoff、TLS、thread、thunk call boundary を
  first design scope として定義する。
- [ ] 最初の Windows x64 CLI fixture target と success criteria を定義する。
- [ ] Wine bridge は OS personality の 1 つとして扱い、Bara core に Win32 semantics を
  埋め込まない。

### B9: 実 x86 32-bit アプリ対応

- [ ] B9 は推奨ステップとする。blocker が大きい場合は記録したうえで B10 へ進んでよい。
- [ ] 最初に起動対象とする実 x86 32-bit アプリの scope と成功条件を定義する。
- [ ] source ISA mode を `x86_64` / `x86_32` として明示する domain type を導入する。
- [ ] address size、operand size、stack width、calling convention を source mode から決める。
- [ ] decoder / lifter は 64-bit 固有部分と 32-bit 共有可能部分を分ける。
- [ ] register model は `eax` / `ax` / `al` などの部分レジスタを明示できる形にする。
- [ ] 32-bit calling convention と 64-bit calling convention を ABI type で分離する。
- [ ] 32-bit Mach-O / PE / ELF 入力を将来追加できる binary format metadata にする。
- [ ] segmentation、x87、MMX、古い SSE code など、32-bit x86 固有の難所を unsupported 分類できるようにする。
- [ ] `x86 -> arm64` と `x86_64 -> arm64` を同じ IR / ARM64 backend に載せる境界を設計する。
- [ ] 必要なら `bara-isa-x86` 内を `x86_32` / `x86_64` / `shared` に分割するか、crate 分割を検討する。
- [ ] 起動結果を stdout、stderr、exit status、launch metadata、blocker classification を含む stable JSON report にする。

### B10: PE / Wine 接続前段

- [ ] B8/B9 の user-space loader / helper boundary を前提に、PE parser の最小 scope を決める。
- [ ] `.text` / `.rdata` / import table の domain model を設計する。
- [ ] Windows x64 ABI と helper boundary の対応を整理する。
- [ ] Wine へ渡すべき責務と Bara が持つべき責務を文書化する。
- [ ] hello world 相当を PE fixture から native artifact へ変換する長期計画を立てる。
- [ ] Wine 接続時に process-wide 互換性が必要な箇所を、kernel 統合ではなく user-space loader / runtime metadata として扱う。

B10 より先の未確立な派生研究は
[将来構想メモ](docs/future-research-concepts.md) に分離し、本流 TODO の
大項目としては扱わない。

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

## ドメイン別参照 TODO

以下は線形実装ロードマップを進めるときの参照バックログであり、実行順では
ない。ここから作業を選ぶ場合も、必ず上の `線形実装ロードマップ` の該当
B 項目へ対応づけてから進める。

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
    Fallthrough { target: X86Va },
    DirectJump { target: X86Va },
    CondJump { taken: X86Va, fallthrough: X86Va, condition: X86Cond },
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
- [x] basic block に分割する。
- [x] typed IR に lift する。
- [x] unsupported instruction を明示的に表現する。

初期対応命令:

- [x] `mov reg, imm`
- [ ] `mov reg, reg`
- [x] `add`
- [x] `sub`
- [x] `cmp eax, imm8/imm32`
- [x] `test eax,eax`
- [x] `jmp` (`rel8`)
- [x] `jcc` (`je/jz rel8`, `jne/jnz rel8`)
- [x] `jcc` その他条件 / rel32（parity 以外は ARM64 lowering まで）
- [x] `call direct` (`rel32` internal target)
- [x] `ret`
- [x] `push`
- [x] `pop`

後回し:

- [ ] parity `jcc` (`jp/jpe`, `jnp/jpo`) の ARM64 lowering は flags
  materialization 拡張で扱う。

- [ ] SIMD/SSE
- [ ] x87
- [ ] AVX
- [ ] segment register
- [ ] syscall
- [ ] self-modifying code

## IR TODO

- [x] basic block は必ず terminator を持つようにする。
- [x] fallthrough を暗黙にしない。
- [x] flags model を定義する。
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
- [x] `add/sub` の最小 emission を実装する。
- [x] `cmp` / `test` の最小 emission を実装する。
- [x] conditional branch の fixup を実装する。
- [x] direct call / return の最小実装を追加する。
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

- [x] `compiled.ir.json` を出せるようにする。
- [x] `pcmap.json` を出せるようにする。
- [x] `fixups.json` を出せるようにする。
- [x] `helpers.json` を出せるようにする。
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

- [x] `clang -target x86_64-apple-macos...` で x86_64 テストバイナリを生成する。
- [x] x86_64 oracle runner を作る。
- [x] oracle runner は結果を JSON で出す。
- [x] Rosetta 実行結果を `expected.json` として保存する。
- [x] BTB 出力を ARM64 native runner で実行する。
- [x] ARM64 実行結果を `actual.json` として保存する。
- [x] `expected.json` と `actual.json` を比較する。

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

2026-06-11 decision: B7 では Haskell package / schema reader / small x86
semantics interpreter をまだ導入しない。現時点の次ステップは、既存 Rust crate の
IR / PC map / fixup metadata に対する verifier を先に作り、JSON schema と
failure package を安定させることに置く。Haskell は、property-based generator /
shrinker と独立仕様モデルが必要になり、schema が固定され、Nix dev shell と
supply-chain 検証を同じ change で追加できる段階で `spec/` 配下に導入する。

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
- [x] Rust deterministic testcase generator を作る。
- [x] Rust deterministic failing case shrink candidate plan を作る。
- [ ] QuickCheck または Hedgehog で testcase generator を作る。
- [ ] failing case を自動 shrink できるようにする。

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

- [x] `DecodeError`
- [x] `UnsupportedInstruction`
- [x] `WrongRegisterValue`
- [x] `WrongFlags`
- [x] `WrongMemory`
- [x] `WrongBranchTarget`
- [x] `WrongCallReturn`
- [x] `WrongExternalCall`

## CI TODO

- [ ] 通常 Rust test を作る。
- [ ] oracle が不要な unit test を作る。
- [ ] arm64 macOS 上だけで Rosetta oracle test を走らせる。
- [x] quick test と oracle test を分離する。
- [x] nightly で deterministic small-case shrink test を走らせる。
- [x] 失敗ケースを corpus に保存する。

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
