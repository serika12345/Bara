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
  分ける。`integration_policy` は current user-space process を scope とし、
  kernel extension、private kernel hook、private dyld behavior を
  `not_required` として記録する。`process_boundary` は loader、
  translation cache、runtime helper、artifact cache を current user-space
  process 内の責務として記録する。現時点では `planned_not_executed` として扱う。
- `blocker`: 初期分類、選ばれた boundary、選択規則、候補 boundary、説明。
  現時点では complete x86_64 Mach-O GUI executable の loader 境界が最初の
  未対応境界であるため `unsupported_loader_feature` とする。候補分類は
  `unsupported_loader_feature`、`unsupported_import`、
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
9. unsupported import、unsupported loader feature、unsupported Objective-C
   runtime boundary を stable classification として report する。
10. helper boundary または runtime support を追加し、expected / actual 比較を
   通す。

## Clean-room 境界

GUI Hello World の実装根拠は、public macOS / Mach-O / AppKit documentation、
self-authored fixture、Rosetta black-box execution から得た外部観測結果に限定する。
Rosetta の disassembly、internal symbol、private metadata、private ABI は使わない。
