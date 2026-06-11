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

## 成功条件

B8 完了時の成功条件は、arm64 macOS 上で self-authored x86_64 GUI Hello World
が Bara 経由で起動し、Rosetta black-box oracle と Bara actual の決定的比較を
通すことである。

比較対象は少なくとも次を含む。

- `stdout`
- `stderr`
- `exit_status`
- `return_value` または process-level return equivalent
- launch metadata
- blocker classification

初期 GUI fixture は、GUI surface creation 後に deterministic な lifecycle event
を stdout または launch metadata へ出す。画面表示そのものは補助観測として扱い、
自動判定は public process observation と stable JSON report に限定する。

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
  として扱い、次の explicit blocker を `unsupported_import` として記録する。
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
  固定後は `unsupported_import` を current blocker とし、次の修正対象は
  `connect_appkit_import_objc_runtime_helper_boundary` とする。
- `blocker`: 初期分類、選ばれた boundary、選択規則、候補 boundary、説明。
  現時点では public AppKit framework import が最初の未対応 helper boundary で
  あるため `unsupported_import` とする。候補分類は `unsupported_import`、
  `unsupported_objc_runtime_boundary` に固定する。

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
   required、explicit `unsupported_import` next blocker を固定済みで、実 import
   解決、Objective-C runtime bridge、OS API mapping は後続 step で扱う。
   次の小ステップでは、その explicit `unsupported_import` を actual result と
   current blocker に接続済みで、loader blocker から import blocker へ進んだ。
   Objective-C runtime boundary の actual result 接続は後続 step で扱う。
22. helper boundary または runtime support を追加し、Rosetta expected と Bara
   actual の比較フィードバックサイクルを回して、最終的に expected / actual
   比較を通す。

## Clean-room 境界

GUI Hello World の実装根拠は、public macOS / Mach-O / AppKit documentation、
self-authored fixture、Rosetta black-box execution から得た外部観測結果に限定する。
Rosetta の disassembly、internal symbol、private metadata、private ABI は使わない。
