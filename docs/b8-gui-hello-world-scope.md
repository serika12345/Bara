# B8 GUI Hello World 起動スコープ

この文書は、B8: 実 x86_64 macOS アプリ起動の最初のターゲットと成功条件を
固定するための focused scope である。B8 の詳細実装順は
[TODO.md](../TODO.md) を source of truth とし、この文書は scope と判定基準を
補足する。

## 起動ターゲット

B8 の最初の起動ターゲットは、Bara repository 内で自作する single-binary
GUI Hello World とする。

- source は repository 内に置く self-authored fixture とする。
- build target は `x86_64-apple-macos` の Mach-O `MH_EXECUTE` とする。
- `.app` bundle、resource bundle、外部同梱ファイル、installer は初期対象にしない。
- public system framework / dylib への dynamic link は許可する。これは
  "single-binary" が「配布単位として自前ファイルが 1 executable」という意味であり、
  macOS system framework への public import を禁止するものではないためである。
- GUI は最小の AppKit-based surface とする。window または alert のどちらを
  fixture として採用するかは、最初の buildable fixture step で deterministic
  observation を優先して決める。
- fixture は user input なしで短時間後に終了する。automated oracle が
  画面操作や人手確認に依存してはならない。

## 成功条件と milestone の切り方

B8 は「ある程度一般の x86_64 macOS application が実行可能」という長期目標を
1 つの完了条件にはしない。そこを B8 の完了扱いにすると reviewable PR を切れないため、
B8 は小さい GUI 起動 slice の積み上げとして進める。

2026-06-11 時点で到達済みの B8-H1 は、self-authored x86_64 Mach-O GUI fixture を
Bara actual path の入力として受け取り、Objective-C runtime / AppKit lifecycle
helper capability が host AppKit helper execution を行い、Rosetta expected と同じ
deterministic lifecycle event を stdout へ出す状態である。これは reviewable
intermediate slice であり、B8 全体の完了、Objective-C runtime や AppKit の内部再実装、
任意 GUI app 対応、または full process-wide x86_64 GUI translation の完了を
意味しない。

当面の B8-G1 成功条件は、GUI window に Hello World のフォント描画を行う最小 app を、
実際の変換レイヤーを通して GUI 上で確認できることである。ここでいう
「変換レイヤーを通す」とは、x86_64 entry path が Bara の decode / lift / emit /
runtime execution を通り、その実行結果として AppKit lifecycle helper capability
が呼ばれることを指す。host AppKit helper は public AppKit API への boundary として
許可するが、helper 単独実行だけで B8-G1 完了とはしない。

比較対象は少なくとも次を含む。

- `stdout`
- `stderr`
- `exit_status`
- `return_value` または process-level return equivalent
- launch metadata
- blocker classification

初期 GUI fixture は、GUI surface creation 後に deterministic な lifecycle event
を stdout または launch metadata へ出す。CI / automated oracle では public process
observation と stable JSON report を判定基準にする。開発者の B8-G1 確認では、
同じ fixture を一定時間表示できる mode で起動し、GUI window と `hello world`
label の実描画を目視確認できるようにする。

## B8-G1 手動可視確認用 binary

B8-G1 の最初の test binary は、automated oracle 用 fixture と同じ self-authored
AppKit source から build する x86_64 Mach-O executable である。automated oracle 用
binary は deterministic stdout を出したあと短時間で終了する。手動可視確認用 binary は
同じ window と `hello world` label を描画するが、auto-close timer を無効化し、
ユーザーが window を閉じるまで AppKit event loop を維持する。

build command:

```sh
nix develop -c cargo run -p btbc-cli -- \
  build-x86_64-gui-hello-world-visible-fixture \
  target/b8/b8_gui_hello_world_visible_x86_64
```

Rosetta manual check:

```sh
arch -x86_64 target/b8/b8_gui_hello_world_visible_x86_64
```

この手動確認では、single executable が Rosetta 上で起動し、AppKit window に
`hello world` label が表示されることを確認する。window close または `Command-Q`
で終了する。これは B8-G1 の入力 binary と GUI 描画要件の確認であり、
Bara の変換レイヤー経由実行は後続 step で接続する。

## B8-G1 translated GUI launch

B8-G1 の完了時点では、Rosetta 確認済みの x86_64 GUI binary を入力として public
Mach-O probe を行い、Bara 側では B8-G1 専用の x86_64 entry
`0f0b4238473131c0c3` を decode / lift / emit / runtime execution に通す。この entry
は Bara-defined `appkit_gui_hello_world` host trap を要求し、その request を
AppKit lifecycle helper capability に接続する。これにより、helper 単独実行ではなく、
translated entry path から GUI helper capability が起動されたことを launch report に
保存する。

automated stable comparison:

```sh
nix develop -c cargo run -p btbc-cli -- \
  generate-arm64-gui-hello-world-translated-actual \
  target/b8/b8_gui_hello_world_visible_x86_64 \
  target/b8/b8_gui_hello_world.translated.actual.json \
  target/b8/b8_gui_hello_world.translated.launch-report.json

nix develop -c cargo run -p btbc-cli -- \
  compare-expected-actual \
  tests/expected/b8_gui_hello_world.json \
  target/b8/b8_gui_hello_world.translated.actual.json
```

manual visible translated launch:

```sh
nix develop -c cargo run -p btbc-cli -- \
  run-arm64-gui-hello-world-translated-visible \
  target/b8/b8_gui_hello_world_visible_x86_64 \
  target/b8/b8_gui_hello_world.translated.visible.launch-report.json
```

manual visible translated launch は AppKit window が閉じられるまで戻らない。
window close または `Command-Q` で終了すると launch report が保存される。
この G1 は B8-G1 専用 host trap contract 経由の最小 GUI launch であり、
full Objective-C runtime / AppKit call translation や任意 x86_64 GUI app 実行を
意味しない。

## 初期 launch metadata schema

Rosetta black-box oracle から生成する初期 sidecar は
`b8_gui_hello_world_launch_metadata_v0` とする。保存場所は
`tests/expected/b8_gui_hello_world.launch.metadata.json` である。

初期 schema は次を含む。

- `case_id`: `b8_gui_hello_world`
- `oracle`: `rosetta_black_box`
- `fixture`: single Mach-O executable、source ISA、binary format、target triple、
  GUI framework
- `observed_events`: GUI surface creation を表す deterministic lifecycle event

## 初期 actual launch report schema

Bara 側の初期 sidecar は `b8_gui_hello_world_actual_launch_report_v0` とする。
保存された fixture は `tests/expected/b8_gui_hello_world.bara.launch-report.json`
である。

初期 schema は次を含む。

- `case_id`: `b8_gui_hello_world`
- `actual_runtime`: `bara_arm64_user_space`
- `status`: `blocked`
- `input`: Mach-O executable image、source ISA、binary format、target triple、
  GUI framework、public binary probe の top-level summary、public Mach-O probe
  由来の loader metadata。loader metadata は `file_type`、load command table、
  recognized entry points / segments、section table metadata、recognized dylib
  imports、symbol / dynamic symbol table metadata、dyld rebase / bind blob metadata、
  chained fixups metadata、executable image conversion blocker を含む。
- `runtime_preparation`: `bara-runtime` の user-space launch plan 由来の
  準備責務。image mapping は loader、entry trampoline と initial stack は
  runtime、imports / ObjC / OS API request は helper boundary の責務として
  分ける。`helper_boundary` は public AppKit framework import、import
  resolution、Objective-C runtime、OS API request を helper capability required
  として扱い、現在の explicit blocker を `unsupported_objc_runtime_boundary` として
  記録する。
  `helper_capability` は Objective-C runtime / AppKit の内部再実装ではなく、
  self-authored fixture の public AppKit GUI lifecycle event を helper boundary
  で観測する contract として、Objective-C runtime bridge、AppKit lifecycle
  event、stdout lifecycle observation を記録する。contract model step では
  planned not executed、B8-H1 helper execution slice では executed として記録する。
  `source_isa_profile` は x86_64 long mode、address size 64-bit、
  default operand size 32-bit、stack width 64-bit を typed profile として
  記録する。`executable_memory` は runtime の責務として、allocation を
  `mmap` private anonymous mapping、protection transition を `mprotect` の
  read-write から read-execute への切り替え、release を `munmap` に限定して
  記録する。`execution_strategy` は runtime の責務として、JIT、AOT、fallback
  interpreter が同じ `user_space_runtime` boundary から selectable であることを
  記録する。`bridge_boundary` は syscall bridge と OS API bridge を
  helper boundary の責務として記録し、bridge 実装を core IR / ARM64 emit に
  埋め込まないことを記録する。`integration_policy` は current user-space
  process を scope とし、kernel extension、private kernel hook、
  private dyld behavior を `not_required` として記録する。`process_boundary` は loader、
  translation cache、runtime helper、artifact cache を current user-space
  process 内の責務として記録する。`platform_model` は signal / exception を
  user-space loader boundary、thread を initial thread only、TLS を deferred、
  memory protection を public OS virtual memory として記録する。
  `macos_constraints` は code signing、W^X、hardened runtime を private bypass
  なしの documented behavior / public API 制約として記録する。
  `fallback_policy` は unimplemented instruction、unknown indirect target、
  unsupported loader feature を stable blocker classification に落とし、
  interpreter fallback と外部 fallback engine は候補だが未実装 / 未接続であること、
  Rosetta 比較フィードバックサイクルは ready not started であることを記録する。
  `loader_execution` は public Mach-O probe 由来の metadata を source とし、
  `LC_MAIN` entryoff、`LC_SEGMENT_64` file ranges、dylib load commands、
  link-edit rebase / bind metadata、Objective-C runtime helper boundary を使う
  user-space loader 実行計画として記録する。
  現時点では `planned_not_executed` として扱う。
- `launch_result`: process-level の actual result projection。初期 blocked state では
  `exit_status`、`return_value`、`stdout`、`stderr` を blocker classification から
  決定し、`actual.json` と launch report の値を揃える。
- `feedback_report`: Rosetta expected と Bara actual / launch report を束ねる
  `b8_gui_hello_world_feedback_report_v0`。observed result の comparison issues、
  current blocker、`loader_execution_plan`、`helper_boundary_plan`、次の修正対象を
  stable JSON として保存する。loader execution plan と helper boundary plan の
  fixed blocker promotion 後は `unsupported_objc_runtime_boundary` を current blocker
  とし、`helper_capability_plan` に AppKit lifecycle helper contract を保存する。
  B8-H1 helper execution slice では comparison issues は空、status は `matched`、
  current blocker は `none`、next action は `review_b8_milestone` とする。
  milestone 再定義後、この next action は B8 全体の完了ではなく H1 の
  reviewable slice 到達を意味する。B8-G1 の次 action は translated entry path から
  AppKit lifecycle helper capability を呼ぶ経路を追加することである。
- `blocker`: 初期段階では選ばれた boundary、選択規則、候補 boundary、説明を保存し、
  Objective-C runtime helper boundary は `unsupported_objc_runtime_boundary` として
  扱った。B8-H1 helper execution slice では helper capability execution により current
  blocker を解除し、`classification: none` として保存する。

## 初期 non-goals

次は B8 の最初の GUI Hello World target では扱わない。

- 任意の third-party GUI app の起動
- `.app` bundle layout、Info.plist、resource loading
- user interaction を必要とする GUI test
- private dyld behavior、private Apple ABI、kernel extension、kernel hook
- system-wide binary translation integration
- hardened runtime / notarization / production code signing policy の完全対応
- Objective-C runtime と AppKit の内部構造の再実装

## 実装の切り方

B8 は次の小ステップで進める。

1. この scope と成功条件を固定する。
2. self-authored GUI Hello World fixture を x86_64 Mach-O executable として
   build できるようにする。
3. Rosetta black-box execution から `expected.json` と launch metadata を作る。
4. Bara 側で同じ input Mach-O executable image を受け取り、actual launch report
   または stable blocker report を出す CLI 境界を作る。
5. Mach-O loader metadata を public format から段階的に増やす。最初は
   existing public Mach-O probe の entry / segment / conversion metadata を
   actual launch report に接続し、その後 sections、imports、relocations を分けて
   追加する。sections は `LC_SEGMENT_64` の public section table から、imports は
   public dylib load commands から、relocation / rebase / bind に必要な loader metadata
   は public link-edit load commands から model 化する。
6. user-space loader/runtime の image mapping、entry trampoline、
   stack / argv / envp、helper boundary を分ける。これは
   `bara-runtime` の pure launch plan と actual launch report の
   `runtime_preparation` に固定済みで、実行は後続 step で扱う。
7. kernel extension、private kernel hook、private dyld behavior を前提にしない。
   これは `runtime_preparation.integration_policy` に固定済みで、実行は
   後続 step で扱う。
8. loader、translation cache、runtime helper、artifact cache を user-space
   process 内に閉じる。これは `runtime_preparation.process_boundary` に
   固定済みで、実装は後続 step で扱う。
9. executable memory は public OS API (`mmap` / `mprotect` など) 経由に
   限定する。これは `runtime_preparation.executable_memory` に固定済みで、
   実行は既存 runtime executable memory 境界、GUI launch integration は後続
   step で扱う。
10. JIT / AOT / fallback interpreter を同じ user-space runtime 境界から選べる
   設計にする。これは `runtime_preparation.execution_strategy` に固定済みで、
   各 strategy の実装と selection policy は後続 step で扱う。
11. syscall / OS API bridge は helper boundary として明示し、core IR / emit へ
   混ぜない。これは `runtime_preparation.bridge_boundary` に固定済みで、
   bridge 実装と OS API mapping は後続 step で扱う。
12. source ISA mode、address size、operand size、stack width を型で表し、
   B9 の x86_32 対応を public API から閉じ出さない。これは
   `runtime_preparation.source_isa_profile` に固定済みで、x86_32 decode /
   lift 実装は B9 で扱う。
13. register model は `rax` だけでなく、`eax` / `ax` / `al` などの
   partial register view を表現できる形にする。これは `bara-ir::X86Reg` の
   register family / width model に固定済みで、既存 `eax` 命令の semantic
   normalization を partial-register semantics へ変える作業は後続 step で扱う。
14. signal / exception / thread / TLS / memory protection を user-space loader
   model として段階的に扱う。これは `runtime_preparation.platform_model` に
   固定済みで、実 signal handler、exception bridge、thread / TLS 実行は後続
   step で扱う。
15. macOS code signing / W^X / hardened runtime 制約を public API と
   documented behavior の範囲で整理する。これは
   `runtime_preparation.macos_constraints` に固定済みで、private bypass や
   production signing policy の完全対応は扱わない。
16. unsupported instruction、unsupported import、unsupported loader feature を
   stable classification として report する。これは initial blocker と fallback
   policy に固定済みで、追加 blocker は同じ分類境界へ寄せる。
17. unimplemented instruction、unknown indirect target、unsupported loader
   feature の fallback 方針を決める。これは
   `runtime_preparation.fallback_policy` に固定済みで、interpreter fallback と
   外部 fallback engine は候補だが未実装 / 未接続として扱う。
18. 起動結果を stdout、stderr、exit status、launch metadata、blocker
   classification を含む stable JSON report にする。これは top-level
   `launch_result` と `runtime_preparation` / `blocker` に固定済みで、
   Rosetta 比較フィードバックサイクルは ready not started で止める。
19. Rosetta expected と Bara actual / launch report を同じ feedback report に
   束ね、現状の blocker と次の修正対象を stable JSON で出す。これは
   `generate-arm64-gui-hello-world-feedback` と
   `b8_gui_hello_world_feedback_report_v0` に固定済みで、初期 next action は
   `implement_user_space_loader_for_mach_o_gui_executable` とする。
20. feedback report の `unsupported_loader_feature` に対して、public Mach-O
   loader metadata から最初の user-space loader 実行計画を作る。これは
   `runtime_preparation.loader_execution` と feedback report の
   `loader_execution_plan` に固定済みで、実 image mapping、import 解決、
   rebase / bind 適用、Objective-C runtime bridge 実行は後続 step で扱う。
21. AppKit import / Objective-C runtime boundary を helper boundary または
   明示 blocker として進める。最初の小ステップでは
   `runtime_preparation.helper_boundary` と feedback report の
   `helper_boundary_plan` に public AppKit framework import、helper capability
   required、explicit blocker を固定済みで、実 import 解決、Objective-C runtime
   bridge、OS API mapping は後続 step で扱う。
   その後の小ステップで explicit `unsupported_import` を actual result と
   current blocker に接続し、さらに `unsupported_objc_runtime_boundary` を
   actual result と current blocker に接続済みである。Objective-C runtime helper
   capability の実行接続は後続 step で扱う。
22. Objective-C runtime / AppKit lifecycle helper capability contract を
   `runtime_preparation.helper_capability` と feedback report の
   `helper_capability_plan` に固定する。これは Objective-C runtime や AppKit の
   内部構造の再実装ではなく、self-authored fixture の deterministic lifecycle
   event を helper boundary で扱うための contract model である。実 host execution
   と current blocker 解除は次 step で扱う。
23. B8-H1 として、Objective-C runtime / AppKit lifecycle helper capability の host execution を
   actual path に接続する。実行は self-authored AppKit fixture source を host
   AppKit helper として public `clang` / AppKit 経由で build/run し、stdout の
   deterministic lifecycle event を actual observation にする。これにより current
   blocker は `none` になり、launch report は `matched`、runtime preparation は
   `helper_capability_executed`、helper capability は `executed` になる。
24. B8-H1 として、Rosetta expected と Bara actual の比較フィードバックサイクルを回し、
   `b8_gui_hello_world_feedback_report_v0` の comparison issues が空、
   next action が `review_b8_milestone` になることを確認する。これは B8 全体の
   完了ではなく、helper capability slice の review gate である。
25. B8-G1 として、x86_64 entry path が Bara の decode / lift / emit /
   runtime execution を通る最小 GUI request を作る。translated code は
   AppKit lifecycle helper capability を helper ABI または host trap contract 経由で
   呼び、host helper 単独実行と区別できる launch report を保存する。
26. B8-G1 として、GUI window と `hello world` label を一定時間表示する
   developer-visible mode を追加する。automated oracle は stable JSON comparison を
   維持し、manual mode は GUI 上のフォント描画確認に限定する。

## Clean-room 境界

GUI Hello World の実装根拠は、public macOS / Mach-O / AppKit documentation、
self-authored fixture、Rosetta black-box execution から得た外部観測結果に限定する。
Rosetta の disassembly、internal symbol、private metadata、private ABI は使わない。
