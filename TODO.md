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

- [ ] B8-G3h: RIP-relative `lea rdi,[rip+disp32]` address materialization boundary を追加する。
  - [ ] B8-G3g の `blocker.json` で見えた `DecodeUnsupportedOpcode { opcode: 72 }`
    (`48 8d 3d b3 10 00 00`) を RIP-relative LEA blocker として focused fixture に固定する。
  - [ ] `rdi` destination の RIP-relative address materialization を、memory read ではない
    typed address operand または最小 IR op として表現する。
  - [ ] general LEA addressing modes が必要な場合は silent fallback せず stable blocker として
    report する。

#### PR Gate: B8-G3h RIP-Relative LEA RDI Address Boundary

branch: `task/b8-g3h-rip-relative-lea-rdi`

完了条件:

- B8-G3g の debug bundle / blocker report から、次に潰す boundary として
  `48 8d 3d disp32` (`lea rdi, [rip+disp32]`) を選んでいる。
- 選んだ blocker の最小 bytes が focused fixture として保存されている。
- memory load と区別して、RIP-relative effective address materialization を decode /
  lift / emit の最小範囲または stable blocker として表現している。
- debug bundle または launch report で `48 8d 3d` blocker を越えるか、次に必要な
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

- [ ] B8-G4: user-space Mach-O image mapping と relocation / rebase / bind 適用を
  実行可能な loader step にする。
  - [ ] `LC_SEGMENT_64` file ranges から executable image / const data / data
    mapping を作り、entry PC と runtime address の関係を typed metadata にする。
  - [ ] public rebase / bind metadata を使い、import symbol identity と
    helper boundary request を解決する。private dyld behavior は使わない。
- [ ] B8-G5: import stub / external symbol call を汎用 helper request に接続する。
  - [ ] symbol stubs、lazy bind 相当、`objc_msgSend`、public libc / AppKit symbol を
    core IR に直接埋め込まず、helper capability request と stable blocker に分ける。
  - [ ] unsupported import は symbol identity、call site、argument model の不足理由を
    report する。
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
