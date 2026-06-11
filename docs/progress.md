# プロジェクト進行履歴

この文書は、コミット履歴を読まなくても Bara がどのように進行してきたかを
把握できるようにするための進行記録である。

詳細な実装 TODO は [TODO.md](../TODO.md)、詳細設計とリファクタリング TODO は
[docs/design-todo.md](design-todo.md)、`hello world` までの詳細な段階履歴は
[docs/hello-world-roadmap.md](hello-world-roadmap.md) に置く。

## 現在の作業スナップショット

最終更新: 2026-06-11 14:54 JST

状態:

- project_state: completed。B8 の helper boundary plan で固定した
  `unsupported_import` next blocker を actual result と current blocker に接続した。
- active_milestone: in_progress。[TODO.md](../TODO.md) の B8:
  実 x86_64 macOS アプリ起動。
- active_design_focus: B8 AppKit / Objective-C helper boundary。public AppKit
  framework import を current blocker として扱い、Objective-C runtime boundary は
  次の helper boundary actual 接続対象として残す。
- active_branch: `task/b8-gui-hello-launch-scope`。base commit は `3d9f1ba`。
  latest commit はこの小ステップの review package で確認する。
- related_todo: [TODO.md](../TODO.md) B8 の「AppKit import / Objective-C
  runtime boundary を helper boundary または明示 blocker として進め、
  expected / actual 差分を縮める」配下の explicit blocker promotion 小項目。
- completed_work: `GuiHelloWorldInitialBlockerPlan::current()` を
  helper boundary の `unsupported_import` next blocker に合わせ、B8 actual result の
  `stderr`、launch report の `blocker`、feedback report の `current_blocker` を
  `unsupported_import` に進めた。candidate boundary は import と Objective-C
  runtime に絞った。
- remaining_work: Objective-C runtime boundary を helper boundary または明示
  blocker として actual result に接続すること。
- next_action: commit / push 後、次の B8 小ステップで
  `unsupported_objc_runtime_boundary` を actual blocker / helper capability 接続へ
  進める。
- verification: targeted tests として
  `nix develop -c cargo test -p btbc-cli gui_hello_world -- --nocapture` が通過した。
  `nix develop -c cargo fmt --all -- --check`、
  `nix develop -c cargo clippy -p btbc-cli --all-targets -- -D warnings`、
  `git diff --check`、full `nix develop -c ./scripts/verify` も通過した。

直近で完了した作業:

- 2026-06-11 14:54 JST: B8 の AppKit import helper boundary step として、
  `helper_boundary_plan.next_blocker` の `unsupported_import` を actual result /
  launch report / feedback report に接続した。B8 actual の `stderr` は
  `unsupported_boundary: unsupported_import` になり、current blocker は public
  AppKit import boundary、candidate boundary は import と Objective-C runtime に
  絞られた。targeted tests、`btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 14:47 JST: B8 の AppKit import / Objective-C runtime boundary
  の最初の小ステップとして、`UserSpaceHelperBoundaryPlan` を詳細化した。
  public AppKit framework import、import resolution、Objective-C runtime、
  OS API request は helper capability required として report され、次 blocker は
  `unsupported_import` として保存される。B8 feedback report は
  `helper_boundary_plan` を含み、next action は
  `connect_appkit_import_objc_runtime_helper_boundary` になった。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 14:37 JST: B8 の `unsupported_loader_feature` に対する
  最初の修正フィードバック対象として、public Mach-O metadata 由来の
  user-space loader 実行計画を model 化した。`UserSpaceLaunchPlan` は
  `loader_execution` に `public_mach_o_probe`、`lc_main_entryoff`、
  `lc_segment_64_file_ranges`、`dylib_load_commands_to_helper_boundary`、
  `linkedit_rebase_bind_metadata`、`helper_boundary`、`planned_not_executed` を
  typed plan として保持し、B8 actual launch report と feedback report は
  `runtime_preparation.loader_execution` / `loader_execution_plan` に同じ plan を
  保存する。targeted tests、`bara-runtime` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 14:25 JST: B8 の Rosetta 比較フィードバックサイクル開始点として、
  `btbc-cli generate-arm64-gui-hello-world-feedback` を追加した。Rosetta expected
  JSON と Bara actual / launch report から
  `b8_gui_hello_world_feedback_report_v0` を生成し、observed result の
  `exit_status` / `stdout` / `stderr` mismatch、current blocker
  `unsupported_loader_feature`、next action
  `implement_user_space_loader_for_mach_o_gui_executable` を stable JSON に保存する。
  targeted tests、`target/b8` での手動 CLI 確認、`btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 14:05 JST: B8 の Rosetta 比較フィードバックサイクル直前の
  boundary 固定として、`bara-runtime::UserSpaceLaunchPlan` に
  `platform_model`、`macos_constraints`、`fallback_policy` を追加した。
  B8 actual launch report は signal / exception / thread / TLS / memory
  protection、macOS code signing / W^X / hardened runtime 制約、fallback
  方針、top-level `launch_result` を stable JSON に保存する。interpreter
  fallback と外部 fallback engine は候補だが未実装 / 未接続、feedback cycle は
  ready not started として記録される。targeted tests、`bara-runtime` /
  `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:54 JST: B8 の register model guardrail step として、
  `bara-ir::X86Reg` に `rax` / `eax` / `ax` / `al` と `rdi` / `edi` / `di` /
  `dil` を追加し、register family、view width、full-width register、
  partial view 判定を domain model として公開した。`btbc-cli` の function
  artifact projection も partial register view を stable JSON 名へ変換できる。
  targeted tests、`bara-ir` / `bara-arm64` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:43 JST: B8 の source ISA profile step として、
  `bara-runtime::UserSpaceLaunchPlan` に `source_isa_profile` を追加した。
  現在は x86_64 long mode、address size 64-bit、default operand size 32-bit、
  stack width 64-bit を typed profile として保持し、B8 actual launch report の
  `runtime_preparation.source_isa_profile` に保存する。profile model は
  x86_32 protected mode も表現できる。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 13:34 JST: B8 の syscall / OS API bridge boundary step として、
  `bara-runtime::UserSpaceLaunchPlan` に `bridge_boundary` を追加した。
  syscall bridge と OS API bridge は helper boundary の責務として B8 actual
  launch report に保存され、core IR / ARM64 emit の bridge 実装は
  `not_embedded` として保存される。targeted tests、`bara-runtime` / `btbc-cli`
  clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:25 JST: B8 の execution strategy selection boundary step
  として、`bara-runtime::UserSpaceLaunchPlan` に `execution_strategy` を追加した。
  JIT、AOT、fallback interpreter は同じ `user_space_runtime` boundary から
  `selectable` として B8 actual launch report に保存される。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 13:15 JST: B8 の executable memory public OS API boundary step
  として、`bara-runtime::UserSpaceLaunchPlan` に `executable_memory` を追加した。
  allocation は `mmap_private_anonymous`、protection transition は
  `mprotect_read_write_to_read_execute`、release は `munmap` として B8 actual
  launch report に保存される。targeted tests、`bara-runtime` / `btbc-cli`
  clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:07 JST: B8 の user-space process boundary step として、
  `bara-runtime::UserSpaceLaunchPlan` に `process_boundary` を追加した。
  loader、translation cache、runtime helper、artifact cache は current
  user-space process 内の責務として B8 actual launch report に保存される。
  targeted tests、`bara-runtime` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:01 JST: B8 の private kernel / dyld assumption exclusion
  step として、`bara-runtime::UserSpaceLaunchPlan` に `integration_policy` を
  追加した。B8 actual launch report は current user-space process を scope とし、
  kernel extension、private kernel hook、private dyld behavior をすべて
  `not_required` として保存する。targeted tests、`bara-runtime` / `btbc-cli`
  clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:50 JST: B8 の user-space loader/runtime preparation step として、
  `bara-runtime::UserSpaceLaunchPlan` を追加し、image mapping、entry trampoline、
  initial stack、helper boundary の準備責務を分けた。B8 actual launch report は
  `runtime_preparation` に `planned_not_executed` の plan projection を保存する。
  targeted tests、`bara-runtime` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:39 JST: B8 の loader metadata model 化ステップとして、
  entry、segments、sections、imports、relocations、rebases、binds に必要な
  public Mach-O metadata を parser/report 境界へ載せた。`LC_SYMTAB`、
  `LC_DYSYMTAB`、`LC_DYLD_INFO(_ONLY)`、`LC_DYLD_CHAINED_FIXUPS` は symbol table、
  dynamic symbol table、dyld rebase / bind blob、chained fixups metadata を typed
  summary として保持する。targeted tests、`bara-oracle` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:23 JST: B8 の 9 つ目の小ステップとして、public dylib load
  commands から Mach-O imports metadata を model 化した。`LC_LOAD_DYLIB` 系 command
  は dependent dylib path、timestamp、current version、compatibility version を
  typed metadata として保持する。B8 actual launch report の `imports` status は
  `modeled_from_dylib_load_commands` になった。targeted tests、`bara-oracle` /
  `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:13 JST: B8 の 8 つ目の小ステップとして、public
  `LC_SEGMENT_64` section table から Mach-O sections metadata を model 化した。
  `section_64` の section name、segment name、addr、size、offset、align、
  reloff、nreloc、flags を typed metadata として parser/report 境界に載せる。
  B8 actual launch report の `sections` status は
  `modeled_from_lc_segment_64_section_table` になった。targeted tests、`bara-oracle` /
  `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:55 JST: B8 の 7 つ目の小ステップとして、Bara 側の
  GUI Hello World actual launch report に public Mach-O probe 由来の loader
  metadata summary を保存するようにした。`input.loader_metadata` は
  `public_mach_o_probe` を source とし、file type、load command table、
  recognized entry points / segments、executable image conversion blocker を保持する。
  sections、imports、relocations は parser 未対応のため `not_modeled` として
  明示した。targeted tests、`btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:43 JST: B8 の 6 つ目の小ステップとして、Bara 側の
  GUI Hello World actual launch attempt が raw function fixture ではなく
  x86_64 Mach-O executable image 全体を入力として受け取るようにした。
  `btbc-cli generate-arm64-gui-hello-world-actual` は `<binary> <actual.json>
  <launch-report.json>` を受け取り、input binary 全体を `BinaryInput` として
  public Mach-O probe に通す。launch report の `input.kind` は
  `mach_o_executable_image` になり、probe の format / status summary を保存する。
  targeted tests、`btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:28 JST: B8 の 5 つ目の小ステップとして、GUI Hello World の
  initial blocker を stable な launch boundary 分類として固定した。
  `b8_gui_hello_world_actual_launch_report_v0` の `blocker` は
  `boundary`、`selected_by`、`candidate_boundaries` を持ち、分類候補を
  `unsupported_loader_feature`、`unsupported_import`、
  `unsupported_objc_runtime_boundary` に限定する。選択規則は
  `first_unsupported_launch_boundary` で、現時点では loader が最初の未対応境界
  であるため `unsupported_loader_feature` を initial blocker とする。targeted
  tests、`btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:17 JST: B8 の 4 つ目の小ステップとして、Bara 側の
  GUI Hello World 起動 attempt を `actual.json` と launch report sidecar へ保存
  する CLI 境界を追加した。`tests/expected/b8_gui_hello_world.bara.actual.json`
  は現在の blocked process observation を保存し、
  `tests/expected/b8_gui_hello_world.bara.launch-report.json` は
  `b8_gui_hello_world_actual_launch_report_v0` として Bara runtime、input identity、
  `unsupported_loader_feature` blocker を保存する。targeted tests と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:03 JST: B8 の 3 つ目の小ステップとして、GUI Hello World
  fixture を Rosetta black-box oracle で実行し、`expected.json` と launch
  metadata の初期 schema を固定した。`tests/expected/b8_gui_hello_world.json`
  は stdout 上の deterministic `gui_window_created` event、exit status 0、
  stderr 空を保存する。`tests/expected/b8_gui_hello_world.launch.metadata.json`
  は `b8_gui_hello_world_launch_metadata_v0` として oracle identity、fixture
  identity、observed lifecycle event を保存する。検証は snapshot の targeted
  tests と commit 前の full verification。

- 2026-06-11 10:48 JST: B8 の 2 つ目の小ステップとして、self-authored
  single-binary GUI Hello World source を追加し、x86_64 Mach-O executable
  としてビルドできる host-specific fixture にした。AppKit source は
  GUI window creation 後に deterministic lifecycle event を stdout へ出し、
  短時間後に終了する。`btbc-cli build-x86_64-gui-hello-world-fixture` は
  `gui_hello_world_mach_o_executable` metadata を返し、public Mach-O probe で
  x86_64 Mach-O として認識できる。検証は snapshot の targeted tests と
  commit 前の full verification。

- 2026-06-11 10:38 JST: B8 の最初の小ステップとして、実 x86_64 macOS
  アプリ起動の初期ターゲットを self-authored single-binary GUI Hello World
  に固定した。`.app` bundle や private dyld integration を初期対象から外し、
  public system framework imports は loader/runtime/helper boundary で扱う。
  成功条件は stdout、stderr、exit status、return value または process-level
  equivalent、launch metadata、blocker classification を含む stable JSON report
  の Rosetta expected / Bara actual 比較とした。検証は snapshot の docs-only
  checks。

- 2026-06-11 10:22 JST: B7 の 19 個目の小ステップとして、IR invariant を
  Rust verifier report に接続した。`validate_program` の validation issue は
  `EmittedFunctionVerificationIssue::IrInvariant` として report に入り、CLI artifact
  では stable `ir_*` issue に変換される。これで B7 の implementation TODO は完了し、
  次は large milestone review gate として PR を開く。検証は snapshot の targeted
  test と commit 前の full verification。

- 2026-06-11 10:10 JST: B7 の 18 個目の小ステップとして、
  stable failure classification kind を追加し、final-state mismatch から具体分類へ
  接続した。`return_value_mismatch` は `WrongRegisterValue`、`stdout_mismatch` は
  `WrongExternalCall`、`exit_status_mismatch` は `WrongCallReturn` になる。
  `UnsupportedReason::DecodeUnsupportedOpcode` などの未対応命令系 emit error は
  `UnsupportedInstruction` に分類する。検証は snapshot の targeted test と
  commit 前の full verification。

- 2026-06-11 10:10 JST: B7 の 17 個目の小ステップとして、
  verification lane scripts を分離した。`verify-quick` は format / security /
  domain / check / clippy / library unit tests、`verify-native` は workspace tests、
  `verify-oracle` は blackbox oracle、`verify-nightly` は small-case shrink tests と
  nightly output directory への failure package 保存を担当する。検証は snapshot の
  lane scripts と commit 前の full verification。

- 2026-06-11 10:04 JST: B7 の 16 個目の小ステップとして、
  Rust deterministic 小ケース生成と shrink candidate plan を追加した。
  `bara_oracle::small_case` は no-args/u64 の小ケース集合と expected final state、
  非ゼロ immediate return の `return 0` shrink 候補を pure に返す。検証は
  snapshot の targeted test と commit 前の full verification。

- 2026-06-11 09:59 JST: B7 の 15 個目の小ステップとして、
  expected / actual final state comparator report を failure package に接続した。
  comparison mismatch 時の `failure.json` は `final_state` field に
  `ComparisonReport` を保存する。検証は snapshot の targeted test と commit 前の
  full verification。

- 2026-06-11 09:52 JST: B7 の 14 個目の小ステップとして、
  Rust verifier report が branch fixup consistency を検査できるようにした。
  fixup target は PC map source に解決できる必要があり、fixup offset / source は
  生成 code 内の 4-byte instruction slot を指す必要がある。検証は snapshot の
  targeted tests と commit 前の full verification。

- 2026-06-11 09:45 JST: B7 の 13 個目の小ステップとして、
  Rust verifier report を追加し、PC map が全 IR block start の source PC を
  保持していることを検査できるようにした。`bara-arm64::verify` は I/O を持たない
  pure report を返し、`emit-fixture-artifacts` / `check-corpus --out` /
  `check-blackbox --out` は `verifier.report.json` を保存する。検証は snapshot の
  targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-11 09:32 JST: B7 の 12 個目の小ステップとして、
  Haskell verifier package / schema reader / small x86 semantics interpreter の
  導入可否を判断した。B7 では Haskell を追加せず、既存 Rust workspace 内で
  verifier report を先に整える。Haskell は schema が安定し、QuickCheck /
  Hedgehog generator / shrinker または独立仕様モデルが必要になった時点で
  `spec/` と Nix toolchain を同じ change で追加する。検証は snapshot の
  documentation-only checks と最終 `nix develop -c ./scripts/verify`。

- 2026-06-11 09:27 JST: B7 の 11 個目の小ステップとして、
  fixture shrink / failure classification / corpus update の初期運用 package を
  追加した。`check-corpus --out` / `check-blackbox --out` は失敗 fixture ごとに
  `failures/<case_id>/failure.json` を保存し、raw testcase の comparison mismatch
  では `testcase.json`、`expected.json`、`actual.json` も保存する。
  `failure.json` には failure kind、message、shrink `not_attempted`、corpus update
  action を含める。検証は snapshot の targeted test と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-11 09:18 JST: B7 の 10 個目の小ステップとして、
  Rosetta black-box oracle 経路を clean-room ルール内で再検討した。
  `x86_64_mach_o_fixture` は `RosettaOracleObservation` を介して runner
  subprocess の status / stdout / stderr だけを扱い、`expected.json` の
  testcase behavior は runner stdout の `ObservedResult` JSON だけから作る。
  `docs/clean-room.md` と `docs/test-oracle.md` に同じ境界を記録した。
  検証は snapshot の targeted test と最終 `nix develop -c ./scripts/verify`。

- 2026-06-11 09:09 JST: B7 の 9 つ目の小ステップとして、
  `check-corpus --out` / `check-blackbox --out` が raw testcase fixture の
  compile artifact metadata を `compiled/<case_id>/` に保存するようにした。
  `actual/<case_id>.json` は外部観測結果の stdout、stderr、exit status、
  return value を保持し、artifact metadata は sidecar として同じ regression
  output bundle に含める。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 23:01 JST: B7 の 8 つ目の小ステップとして、
  generated executable smoke を `ObservedResult` regression gate に昇格した。
  `check-blackbox --out` は `return_42_native_executable_smoke` と
  `mach_o_return_42_native_executable_smoke` の process execution result を
  `actual/*.json` に保存する。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 22:51 JST: B7 の 7 つ目の小ステップとして、
  `emit-fixture-artifacts` が `artifact.report.json` を保存するようにした。
  report は function-level v0 state layout、fixture function v0 cache validation
  identity、helper requirements を含む。stdout host trap fixture では
  `write_stdout(ptr_len_to_unit)` requirement が記録される。検証は snapshot の
  targeted test と最終 `nix develop -c ./scripts/verify`。

- 2026-06-10 22:35 JST: B7 の 6 つ目の小ステップとして、
  `emit-fixture-artifacts` CLI を追加した。testcase を Bara の decode / lift /
  ARM64 emit pipeline に通し、`compiled.ir.json`、`pcmap.json`、`fixups.json`、
  `helpers.json` を指定 directory に保存する。ARM64 emitter は branch lowering で
  適用した fixup の offset / source / target / kind を `EmittedFunction` に保持し、
  CLI 側の stable JSON DTO へ写す。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 21:36 JST: B7 の 5 つ目の小ステップとして、
  `compare-expected-actual` CLI を追加した。保存済みの `expected.json` と
  `actual.json` を `ObservedResult` として parse し、M1 の比較対象フィールドを
  `ComparisonReport` で比較する。一致時は空 issue report を stdout に出し、
  不一致時は `ComparisonMismatch` として非ゼロ終了する。検証は snapshot の
  targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-10 21:19 JST: B7 の 4 つ目の小ステップとして、
  `generate-arm64-actual` CLI を追加した。testcase を Bara の decode / lift /
  ARM64 emit / native runner pipeline に通し、`ObservedResult` JSON を
  `actual.json` として保存する。比較は次ステップに残した。検証は snapshot の
  targeted test と最終 `nix develop -c ./scripts/verify`。

- 2026-06-10 21:00 JST: B7 の 3 つ目の小ステップとして、
  `generate-x86_64-expected` CLI を追加した。一時 x86_64 oracle runner を
  build して Rosetta 上で実行し、stdout の `ObservedResult` JSON を
  `expected.json` として保存する。Rosetta host 非対応時は `RunError` として
  分類する。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 20:40 JST: B7 の 2 つ目の小ステップとして、
  `build-x86_64-oracle-runner` CLI を追加した。runner は testcase bytes を
  executable memory に配置して no-args / `u64` function として呼び出し、
  `ObservedResult` 互換 JSON を stdout に出す x86_64 Mach-O executable として
  build される。Rosetta 実行と `expected.json` 保存は次ステップに残した。検証は
  snapshot の targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 22:22 JST: B7 の先頭小ステップとして、
  `build-x86_64-macho-fixture` CLI を追加した。`return_42` testcase は
  x86_64 Mach-O `_main` として assemble / link され、生成 binary の Mach-O
  magic と public header 上の x86_64 cputype を regression で確認する。引数 ABI
  と host trap fixture は後続 runner harness へ分離し、現時点では classified
  unsupported とした。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-09 21:55 JST: B6 の最後の小ステップとして、pure writer の
  serialized output Mach-O を既存の public Mach-O probe に通す regression を
  追加した。`mach_o_hello_world_stdout.bin` 由来の compile 経路で作った writer
  output が、writer layout の entry offset、load command size、segment file
  size と一致する `LC_MAIN` / `LC_SEGMENT_64` metadata として probe される。
  これにより B6 の実装 TODO は完了し、次は large milestone review gate として
  branch の PR を開く。検証は snapshot の targeted test と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-09 21:42 JST: B6 の 9 つ目の小ステップとして、`bara-mach-o`
  の pure writer に offset / size / byte serialization 境界を追加した。
  writer は minimal ARM64 Mach-O の header、`LC_SEGMENT_64`、section table、
  `LC_MAIN`、text / const payload bytes を型付き layout と serialized bytes
  として返す。`btbc-cli` の実 Mach-O stdout fixture 入力経路から compile した
  ARM64 main bytes と binary metadata 由来 stdout const bytes が、この writer
  serialization plan の text / const range に配置されることを検証した。検証は
  snapshot の targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 21:28 JST: B6 の 8 つ目の小ステップとして、Mach-O entry
  pipeline の Program image metadata を entry-aware にした。code section は
  selected segment 全体ではなく entry offset 以降の range として保持する。
  Embedded stdout metadata がある場合は、entry 前の self-authored
  `BARA_STDOUT\0` payload を `ConstData` section として保持し、同じ binary
  metadata から stdout host trap request を作る。検証は snapshot の targeted
  tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 21:18 JST: B6 の 7 つ目の小ステップとして、
  `ProgramImageMetadata` を `bara-ir` に追加した。metadata は code sections、
  symbols、relocations、imports、unwind entries を typed collection として
  持つ。`Program::new` は空 metadata の互換 API として残し、
  `Program::with_image_metadata` と
  `lift_decoded_function_with_image_metadata` が metadata 付き Program を作る。
  Mach-O entry pipeline は selected code segment range を code section として
  `MachOEntryFunctionInput` に添付し、Mach-O native artifact compile 経路は
  その metadata を IR へ渡す。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-09 20:46 JST: B6 の 6 つ目の小ステップとして、
  `MachOEntryFunctionInput` を追加し、Mach-O executable image 全体と
  entry-derived `TestCase` を同じ pipeline 出力として保持するようにした。
  既存の `mach_o_entry_function_test_case*` API は互換 wrapper として残し、
  `btbc-cli` の Mach-O native artifact link 経路は `TestCase` 単体ではなく
  `MachOEntryFunctionInput` を受ける。回帰テストでは entry bytes だけでなく、
  selected code segment 全体と entry offset が保持されることを確認した。検証は
  snapshot の targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 20:08 JST: B6 の 5 つ目の小ステップとして、malformed /
  unsupported Mach-O の artifact 生成時 blocker classification を回帰テストで
  固定した。`link-mach-o-arm64-main` と `link-mach-o-arm64-stdout-main` は
  short Mach-O input を `MachOEntryFunctionTestCaseError::Probe(InputTooShort)`
  として、entry point はあるが segment がない input を
  `MachOEntryFunctionTestCaseError::Plan(NotConvertible { blocker:
  MissingSegment })` として返し、native artifact output を作らない。検証は
  targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 22:28 JST: B6 の 4 つ目の小ステップとして、fixture 専用
  host trap JSON への依存を減らした。`mach_o_hello_world_stdout.bin` は
  selected segment の entry 前に self-authored `BARA_STDOUT\0` payload を
  持ち、`mach_o_entry_function_test_case_with_embedded_host_traps` はその
  payload から `TestCaseHostTrapPlan::stdout` を作る。`check-blackbox` と
  native stdout artifact のデフォルト経路は host-traps JSON を読まず、
  明示 JSON 経路は互換テストとして残す。検証は snapshot の targeted tests
  と `nix develop -c ./scripts/verify`。

- 2026-06-08 21:47 JST: B6 の 3 つ目の小ステップとして、input Mach-O の
  entry / segment / stack metadata を native output packaging に渡す境界を
  追加した。`NativeArtifactMetadata` は raw fixture では既存 JSON を維持し、
  Mach-O artifact 経路では optional `source_image` として `entryoff`、
  `stacksize`、selected segment の `name` / `vmaddr` / `fileoff` / `filesize`
  を保持する。Mach-O artifact CLI は既存 entry function testcase 変換を先に
  通すため、malformed / unsupported Mach-O の既存分類を優先する。検証は
  snapshot の targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 21:33 JST: B6 の 2 つ目の小ステップとして、Mach-O backed
  `hello world` 入力を native executable artifact へ変換する CLI /
  blackbox 経路を追加した。`link-mach-o-arm64-stdout-main` は Mach-O 入力と
  host trap plan を既存の `mach_o_entry_function_test_case_with_host_traps`
  経由で `TestCase` に変換し、stdout helper-aware compile と
  `link_arm64_stdout_main_executable` に委譲する。
  `mach_o_hello_world_stdout_native_executable` を blackbox report に追加し、
  生成 artifact を実行して stdout `hello world\n` と exit status 0 を確認する。
  検証は snapshot の targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 21:12 JST: B6 の先頭小ステップとして、Mach-O backed
  `return_42` 入力を native executable artifact へ変換する CLI / blackbox
  経路を追加した。`link-mach-o-arm64-main` は Mach-O 入力を既存の
  `mach_o_entry_function_test_case` 経由で `TestCase` に変換し、
  standalone artifact compile と `link_arm64_main_executable` に委譲する。
  `mach_o_return_42_native_executable_smoke` を blackbox report に追加し、
  生成 artifact を実行して exit status 42 を確認する。検証は snapshot の
  targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 20:18 JST: B5 large milestone completion の最終小ステップとして、
  IR validation に missing branch/fallthrough/call target report を追加し、
  decoder / lifter は short / near `jcc` 全条件を `CondJump` へ接続した。
  ARM64 emitter は parity 以外の条件を `b.cond` へ lower し、parity 条件を
  explicit unsupported として維持する。`jl_rel32_return_42` を repository
  fixture に追加し、signed-less rel32 branch を decode / lift / emit / runtime
  regression にした。検証は snapshot の targeted tests と
  `nix develop -c ./scripts/verify`。

- 2026-06-08 20:05 JST: B5 large milestone completion の小ステップとして、
  `Push` / `Pop` IR、internal-target `DirectCall` terminator、ARM64 の
  16-byte aligned stack slot lowering、direct call `bl` fixup、link-register
  save/restore、block 間 `rax` live-in propagation を追加した。
  `push_pop_return_42`、`loop_countdown_return_0`、`nested_call_return_42` を
  repository fixture に追加し、nested call は linked native executable artifact
  としても実行した。検証は snapshot の targeted tests と
  `nix develop -c ./scripts/verify`。

- 2026-06-08 19:53 JST: B5 large milestone completion の小ステップとして、
  short `jmp rel8` を `DecodedInstructionKind::JmpRel8` と
  `Terminator::DirectJump` へ接続した。`direct_jmp_return_42` を repository
  fixture に追加し、decode / lift / emit と native runtime 実行の regression
  とした。検証は `nix develop -c cargo test -p bara-isa-x86
  decodes_jmp_rel8_and_continues_with_target_block`、`nix develop -c cargo test
  -p bara-isa-x86 lifts_jmp_rel8_to_direct_jump_terminator`、`nix develop -c
  cargo test -p bara-runtime direct_jmp_return_42`、`nix develop -c cargo test
  -p btbc-cli check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures`、
  および `nix develop -c ./scripts/verify`。

- 2026-06-08 19:45 JST: B5 large milestone completion の小ステップとして、
  ARM64 emitter に `cmp x0,#imm12`、`tst x0,x0`、`b.eq` / `b.ne`、
  unconditional `b` の branch fixup を追加した。`branch_eq_return_42` を
  repository fixture に追加し、decode / lift / emit と native runtime 実行の
  regression とした。検証は `nix develop -c cargo test -p bara-arm64
  emits_conditional_branch_fixups_for_equal`、`nix develop -c cargo test -p
  bara-arm64 emits_cmp_x0_immediate_for_rax_compare_immediate`、`nix develop -c
  cargo test -p bara-arm64 emits_tst_x0_x0_for_rax_test_rax`、
  `nix develop -c cargo test -p bara-runtime branch_eq_return_42`、
  `nix develop -c cargo test -p btbc-cli
  check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures`、および
  `nix develop -c ./scripts/verify`。

- 2026-06-08 19:36 JST: B5 large milestone completion の準備として、decoder が
  `ret` と direct `call rel32` の後続 block bytes を保持できるようにした。
  explicit terminator で EOF に到達した場合は missing return sentinel を追加しない。
- 検証: `nix develop -c cargo test -p bara-isa-x86 trailing_block_bytes`、
  `nix develop -c cargo test -p bara-isa-x86 call_rel32_then_fallthrough_instruction`、
  および `nix develop -c cargo test -p bara-isa-x86
  missing_ret_becomes_unsupported_instruction` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 19:26 JST: B5 の 9 つ目の小ステップとして、
  short `jne/jnz rel8` を decode / lift し、`X86Cond::NotEqual` の
  `Terminator::CondJump` として IR に追加した。負 displacement の target
  計算も regression で確認した。
- 検証: `nix develop -c cargo test -p bara-isa-x86 jne` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 19:07 JST: B5 の 8 つ目の小ステップとして、
  short `je/jz rel8` を decode / lift し、`X86Cond::Equal` の
  `Terminator::CondJump` として IR に追加した。fallthrough 側の後続命令は
  次 block として保持する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 je` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:59 JST: B5 の 7 つ目の小ステップとして、
  `test eax,eax` を decode / lift し、`IrOp::Test` として
  flags-producing IR に追加した。ARM64 emit は flag lowering 実装前の
  explicit unsupported として分類する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 test_eax`、
  `nix develop -c cargo test -p bara-ir test_op`、および
  `nix develop -c cargo test -p bara-arm64 test_ops_are_not_emitted_before_flag_lowering`
  が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:48 JST: B5 の 6 つ目の小ステップとして、
  `cmp eax, imm8/imm32` を decode / lift し、`IrOp::Cmp` として
  flags-producing IR に追加した。ARM64 emit は flag lowering 実装前の
  explicit unsupported として分類する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 cmp`、
  `nix develop -c cargo test -p bara-ir cmp`、および
  `nix develop -c cargo test -p bara-arm64 cmp_ops_are_not_emitted_before_flag_lowering`
  が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:41 JST: B5 の 5 つ目の小ステップとして、
  `FlagValue::{Known, Unknown}` と CF/PF/AF/ZF/SF/OF を持つ `Flags`
  domain model を `bara-ir` に追加した。`cmp` / `test` / `jcc` の
  decode / lift / emit は後続小ステップに分離した。
- 検証: `nix develop -c cargo test -p bara-ir flags` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:36 JST: B5 の 4 つ目の小ステップとして、
  `Fallthrough`、`DirectJump`、`CondJump`、`X86Cond` を typed IR
  terminator として追加した。branch lowering / fixup はまだ実装せず、
  ARM64 emit は explicit unsupported として分類する。
- 検証: `nix develop -c cargo test -p bara-ir` と
  `nix develop -c cargo test -p bara-arm64 emit::tests` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 17:15 JST: B5 の 3 つ目の小ステップとして、terminator
  がない decoded stream 末尾を暗黙 fallthrough とせず、
  `MissingReturnTerminator` の typed unsupported terminator を持つ
  `BasicBlock` として lift するようにした。
- 検証: `nix develop -c cargo test -p bara-isa-x86 lift::tests` と
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 16:45 JST: B5 の 2 つ目の小ステップとして、lifter に
  basic block 分割境界を導入した。`ret` などの terminator instruction で
  block を確定し、後続 instruction があれば次の `BlockId` と source range
  を持つ `BasicBlock` として lift する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 lift::tests` と
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 16:32 JST: B5 の最初の小ステップとして、`add` / `sub`
  fixture coverage が control-flow 前段の regression corpus に含まれている
  状態を TODO と進行履歴へ反映した。既存の `tests/cases`、`tests/expected`、
  `crates/bara-runtime` regression、`tests/expected-reports/blackbox.json` が
  `add` / `sub` 単独および複合 fixture を保持している。
- 検証: `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 15:59 JST: B4 の最後の小ステップとして、unsupported
  syscall / external call の分類と report schema を安定させた。
  function-level の emit unsupported boundary は corpus failure
  `message` に stable JSON として出力される。syscall は ABI と
  address range、external call は symbol id、unresolved/public symbol
  import target、call site / return address を記録する。
- 検証: `nix develop -c cargo test -p btbc-cli
  function_run::tests::unsupported`、`nix develop -c ./scripts/check-domain-types`、
  `nix develop -c ./scripts/check-no-invisible-chars`、`git diff --check`、
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 15:51 JST: B4 の 7 つ目の小ステップとして、libc / dyld /
  import 呼び出しを直接模倣せず、public symbol/import identity として扱う
  model を追加した。`ExternalCallRequest` は `ExternalSymbolImport` を保持し、
  `libc::puts`、`libc::write`、`dyld_stub_binder` を public symbol identity
  として表現できる。
- 検証: `nix develop -c cargo test -p bara-ir` が通過した。続く final B4
  step で full `nix develop -c ./scripts/verify` を実行する。
- 2026-06-08 15:40 JST: B4 の 6 つ目の小ステップとして、macOS / Linux /
  Windows の OS ABI 差分を stdout helper emission strategy 境界で分離した。
  `arm64-apple-macos` は public `_write` prologue strategy に解決され、
  `aarch64-unknown-linux-gnu` と `aarch64-pc-windows-msvc` は
  `write_stdout` helper emission の explicit unsupported target として分類される。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact`、
  `nix develop -c ./scripts/check-domain-types`、`nix develop -c ./scripts/check-no-invisible-chars`、
  `git diff --check`、および `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 14:34 JST: B4 の 5 つ目の小ステップとして、stdout 相当を
  Bara host helper から native stdout emission へ変換する境界を文書化した。
  `write_stdout(ptr_len_to_unit)` は Bara host effect capability であり、
  macOS ARM64 standalone artifact では output packaging 境界が public
  `_write` prologue に変換する。decode / lift / core IR / ARM64 emit /
  manifest parsing / oracle comparison へ native emission の責務を漏らさない。
- 検証: documentation-only 変更として
  `nix develop -c ./scripts/check-no-invisible-chars` と `git diff --check` が
  通過した。code/script/config 変更がないため full `./scripts/verify` は省略した。
- 2026-06-08 14:26 JST: B4 の 4 つ目の小ステップとして、
  `puts` / `write` 相当の stdout effect を Bara host helper
  `write_stdout(ptr_len_to_unit)` の typed request として扱えるようにした。
  `HostTrapKind::Stdout` は `HostHelperRequest::WriteStdout` へ写像され、
  executable manifest preflight は resolved manifest helper を IR 側の
  `HostHelperAbi` と照合してから実行へ進む。
- 検証: `nix develop -c cargo test -p bara-ir`、
  `nix develop -c cargo test -p btbc-cli executable_run`、および
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 13:13 JST: B4 の 3 つ目の小ステップとして、
  `helper_call_external`、`helper_unimplemented`、`helper_exit` の最小 ABI を
  typed domain value として定義した。helper ABI は名前と signature の pure
  value であり、runtime 実行や host syscall 呼び出しはまだ行わない。
- 検証: `nix develop -c cargo test -p bara-ir` と
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 13:00 JST: B4 の 2 つ目の小ステップとして、external
  symbol / import call を `BoundaryRequest::Helper(HelperRequest::CallExternal(...))`
  として IR に残す境界を追加した。ARM64 emit は実行コードを出さず
  `ExternalCallUnsupported { request }` を返す。
- 検証: `nix develop -c cargo test -p bara-ir`、
  `nix develop -c cargo test -p bara-arm64`、`nix develop -c ./scripts/verify`
  が通過した。
- 2026-06-08 12:50 JST: B4 の先頭小ステップとして、x86_64 `syscall` を
  typed public ABI request として IR に残す境界を追加した。`syscall` は
  `BoundaryRequest::Syscall(SyscallRequest { abi: X86_64, at, return_to })`
  として lift され、ARM64 emit は実行コードを出さず
  `SyscallUnsupported { request }` を返す。
- 検証: `nix develop -c cargo test -p bara-ir`、
  `nix develop -c cargo test -p bara-isa-x86`、
  `nix develop -c cargo test -p bara-arm64`、`nix develop -c ./scripts/verify`
  が通過した。
- 2026-06-08 12:06 JST: 旧 M 系マイルストーンと `当面の最短 TODO` の
  具体情報を、削除ではなく [TODO.md](../TODO.md) の B1-B8 へ吸収した。
  `add/sub`、`cmp/test/jcc`、`push/pop/call`、Rosetta oracle、Haskell
  verifier、fallback、metadata 出力などの項目を線形ロードマップ内に残し、
  実行順は B1-B10 のまま維持した。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:59 JST: 実装順を [TODO.md](../TODO.md) の
  `線形実装ロードマップ` に一本化した。README も独立した実装順を持たず
  TODO の線形ロードマップへ案内する形にした。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:51 JST: B8 と PE / Wine 接続前段の間に、B9:
  実 x86 32-bit アプリ対応を挿入した。B9 は互換性上の論点を Wine 接続前に
  発見するための推奨ステップとし、blocker が大きい場合は記録したうえで
  飛ばして B10: PE / Wine 接続前段へ進んでよいことを明記した。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:42 JST: TODO 本流の長期目標を再整理した。PE / Wine
  接続前に B8: 実 x86_64 macOS アプリ起動へ到達することを明記し、旧 B9
  の source ISA mode / x86_32 guardrail と旧 B10 の user-space runtime
  architecture を B8 の設計制約へ統合した。旧 B11/B12 の wasm2c /
  platform adapter / LLVM IR / Wasm 副出力は
  [将来構想メモ](future-research-concepts.md) へ移し、本流 TODO から外した。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:21 JST: B3 の最後の小ステップとして、`clang` packaging 経路と pure writer 経路の差分検証を `bara-mach-o` の公開仕様ベース model 比較として追加した。`MachOArm64ClangPackagingModel`、comparison report、classified mismatch issue を定義し、`_main` / `__TEXT` / `__text` / optional `__const` / minimal load commands の parity を検証できるようにした。B3 は review gate に到達した。
- 検証: `nix develop -c cargo test -p bara-mach-o` は未実装 comparison API の compile error で期待どおり失敗し、実装後に通過した。変更全体の `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 11:08 JST: B3 の 3 つ目の小ステップとして、`bara-mach-o` の writer plan に public Mach-O model を追加した。`_main` entry、`__TEXT` segment、mandatory `__text` section、const payload がある場合の `__const` section、最小 `LC_SEGMENT_64` / `LC_MAIN` 相当の load command model を domain type として定義した。
- 検証: `nix develop -c cargo test -p bara-mach-o` は未実装 model API の compile error で期待どおり失敗し、実装後に通過した。変更全体の `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:46 JST: B3 の 2 つ目の小ステップとして、`bara-mach-o` crate を追加し、ARM64 Mach-O executable writer の pure planning 境界を設計した。`MachOArm64MainCode`、`MachOArm64ConstData`、writer request、payload、plan、target を domain type として定義し、empty payload parts は classified input error にする。
- 検証: `nix develop -c cargo test -p bara-mach-o` は未実装 API の compile error で期待どおり失敗し、実装後に通過した。変更全体の `nix develop -c ./scripts/verify-supply-chain` と `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:26 JST: B3 の最初の小ステップとして、Mach-O input parser と output artifact planning / materialization の責務を module 境界で分離した。`binary_format::input` が public format probe / Mach-O metadata / load command parsing を扱い、`binary_format::output` が executable image plan / materialization を扱う。外部公開 API は `binary_format` と crate root の re-export で維持した。
- 検証: 移動前後で `nix develop -c cargo test -p bara-oracle binary_format::tests` が通過した。変更全体の `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:09 JST: B2 の最後の小ステップとして、unsupported host を classified stable output にした。`NativeArtifactError::UnsupportedHost` は `EmitError` の分類を保ちつつ、artifact kind、target triple、host os/arch を含む JSON message を返す。
- 検証: `nix develop -c cargo test -p btbc-cli unsupported_host_error_serializes_as_stable_json_message` は既存 text message との差分で期待どおり失敗し、実装後に同テスト、`nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:04 JST: B2 の 4 つ目の小ステップとして、外部 `clang` packaging を `NativeArtifactPackager` trait 境界へ分離した。`ClangNativeArtifactPackager` が現行 process 実行を担当し、test fake packager は同じ request から linked executable metadata を返せる。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_packaging_boundary_accepts_different_packagers` は未実装 trait / request の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:00 JST: B2 の 3 つ目の小ステップとして、generated code、stdout data、toolchain command、output path の責務を分離した。`NativeGeneratedCode`、`NativeStdoutData`、`NativeToolchainCommand`、`NativeArtifactOutputPath` を導入し、`link_assembly_source` は typed output path と toolchain command を組み立ててから外部 process を呼ぶようにした。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_request_types_separate_code_stdout_command_and_output_path` は未実装型 / method の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 09:48 JST: B2 の 2 つ目の小ステップとして、native linked executable artifact の metadata JSON 出力を追加した。`link-fixture-arm64-main` は text ではなく artifact metadata JSON を返し、metadata は execution result と別の domain value として保持される。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_metadata_serializes_as_stable_json` は未実装 serializer / accessor の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify-supply-chain`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 09:35 JST: merge 済み B1 branch を local cleanup し、B2 branch `task/b2-artifact-domain-types` を開始した。B2 の最初の小ステップとして、raw ARM64 bytes、native assembly source、linked executable を `native_artifact` module 内の別 domain type として分離した。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_types_separate_raw_source_and_linked_executable` は未実装型の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact::tests`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 09:23 JST: B1 の最後の小ステップとして、`docs/hello-world-roadmap.md` を完了済みロードマップに整理し、B1 安定化成果から B2 の実行可能成果物モデルへ接続した。
- 検証: `nix develop -c ./scripts/check-no-invisible-chars`、`git diff --check`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 21:47 JST: B1 の先頭小ステップとして、生成 executable の smoke test を blackbox report に追加した。`return_42_native_executable_smoke` は `return_42` fixture を native executable として link し、実プロセス exit status 42 と空 stdout/stderr を確認する。
- 検証: 期待 fixture 更新後に `nix develop -c cargo test -p btbc-cli check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures` が期待どおり失敗し、実装後に同テスト、`nix develop -c cargo test -p btbc-cli check_blackbox_writes_report_and_schema_specific_actual_outputs`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 21:54 JST: B1 の 2 つ目の小ステップとして、`link-fixture-arm64-stdout-main` の出力を stable `ObservedResult` JSON report にした。生成 artifact は command 内で実行され、stdout `hello world\n`、exit status 0、stderr 空が JSON に含まれる。
- 検証: 期待 fixture 更新後に `nix develop -c cargo test -p btbc-cli link_fixture_arm64_stdout_main_writes_hello_world_executable` が期待どおり失敗し、実装後に同テストと `nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 22:10 JST: B1 の 3 つ目の小ステップとして、native artifact packaging / toolchain / execution の failure classification を整理した。temporary assembly と `clang` 呼び出し、linked executable 欠落は `EmitError`、artifact 実行失敗は `RunError` に分類する。
- 検証: 期待分類テスト追加後に `nix develop -c cargo test -p btbc-cli packaging_and_toolchain_failures_are_emit_errors` が期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact::tests` と `nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 23:46 JST: B1 の 4 つ目の小ステップとして、native artifact 関連の CLI behavior tests を `main.rs` から `crates/btbc-cli/src/native_artifact_cli_tests.rs` へ分割した。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_cli_tests` と `nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 14:48 JST: Bara の agent action commands を VSCode / Codex IDE から選べるように、repo-scoped skill として `.agents/skills/bara-*` を追加した。
- 検証: `nix develop -c ./scripts/verify` は `verify-cves` の pipe 処理で停止したため中断。代わりに同等 gate を分解して実行し、`cargo fmt --all -- --check`、`./scripts/check-no-invisible-chars`、`./scripts/check-domain-types`、`cargo metadata --locked --format-version 1`、manual `cargo audit` baseline check、`cargo deny check`、`./scripts/verify-nix-package`、`cargo check --workspace --all-targets`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`./scripts/verify-blackbox` が通過した。

## 進行記録の更新規律

この文書は「履歴」だけでなく、コンテキストなしで現在何が進行中かを把握する入口でもある。

今後、TODO-backed implementation、refactoring、architecture work、milestone branch work、
または大きな documentation / policy change を開始、停止、完了、保留、方針転換するときは、
この文書の `現在の作業スナップショット` を同じ変更で更新する。

各進行記録には、最低限以下を含める:

- 更新時刻。形式は `YYYY-MM-DD HH:MM JST` とする。
- 状態。`planned`、`in_progress`、`paused`、`blocked`、`completed`、`superseded` のどれかを明記する。
- 対応する `TODO.md`、`docs/design-todo.md`、または focused roadmap entry。
- 作業 branch と、commit 済みなら最新 commit。
- 何が終わり、何が未完了で、次に何をするべきか。
- 実行した検証、または検証を狭めた理由。

進行中の項目を放置しない。作業が完了、保留、または別方針に置き換わった場合は、
古い `in_progress` 状態を必ず更新する。

## 現在地

最小 `hello world` milestone は完了済み。

到達済み:

- raw x86 function fixture を decode / lift / ARM64 emit できる。
- ARM64 machine code artifact をファイルへ出力できる。
- macOS ARM64 executable artifact として package できる。
- 生成 executable を OS 上で起動し、実 OS stdout に `hello world\n` を出せる。

現在の主な次フェーズ:

- fixture 専用の成功経路を実バイナリ対応へ広げる。
- B4-B7 で syscall / libc 境界、control flow、Mach-O 入力、oracle /
  regression 基盤を広げる。
- B8 で実 x86_64 macOS アプリを user-space runtime から起動できる状態を
  目指す。
- B9 で実 x86 32-bit アプリ対応を扱う。blocker が大きい場合は記録して
  飛ばせるが、B10 の PE / Wine 接続前に先に処理するのが望ましい。
- PE / Wine 接続前段は B10 として扱う。
- wasm2c、NDA target adapter、LLVM IR / Wasm 副出力は
  [将来構想メモ](future-research-concepts.md) に分離し、本流 TODO から外す。

## 完了済みマイルストーン

### Hello World milestone

状態:

- 完了。

到達点:

- raw x86_64 function bytes から ARM64 native runner で `rax` return value を比較できる。
- stdout host trap を fixture として扱い、expected / actual JSON で比較できる。
- Bara executable manifest v0 から raw function pipeline へ変換できる。
- public Mach-O probe、Mach-O backed raw function 実行、Mach-O backed stdout fixture 実行を扱える。
- raw testcase から ARM64 machine code artifact を出力できる。
- raw testcase から macOS ARM64 executable artifact を生成できる。
- stdout host trap fixture を standalone macOS ARM64 executable artifact として package し、実 OS stdout へ `hello world\n` を出せる。

検証:

- `nix develop -c ./scripts/verify` が Hello World milestone 完了時点で通過済み。
- 詳細な段階履歴は [docs/hello-world-roadmap.md](hello-world-roadmap.md) に保存済み。

## 進行上の決定

### TODO と設計 TODO の分離

状態:

- 完了。

決定:

- [TODO.md](../TODO.md) は線形実装ロードマップを管理する。
- [docs/design-todo.md](design-todo.md) は詳細設計、分割方針、リファクタリング、単一責任監査の TODO を管理する。

理由:

- 実装作業とリファクタリング作業が同じ TODO に混ざると、差分の目的が曖昧になる。
- 今後は feature work と refactoring work をできるだけ分けて進行できるようにする。

### エージェント進行規律の固定

状態:

- 運用ルールとして追加済み。

決定:

- エージェントは実装前に関連する `TODO.md` と `docs/design-todo.md` を参照する。
- TODO にない作業は、先に milestone または focused roadmap entry として記録してから実装する。
- 実装状況と TODO の状態を一致させる。
- 完了済みマイルストーンや大きな方向転換は、この文書に記録する。

理由:

- セッションごとにコンテキストを再説明しなくても、ドキュメントから次に進むべき作業を判断できるようにする。
- プロジェクトがどのように進行したかを、コミット履歴に依存せず把握できるようにする。

### タイムスタンプ付き進行スナップショット

状態:

- completed: 2026-06-07 20:00 JST。

決定:

- [docs/progress.md](progress.md) の先頭付近に `現在の作業スナップショット` を置く。
- 作業の開始、停止、完了、保留、方針転換時に、時刻、状態、対応 TODO、branch/commit、検証、次の作業を記録する。
- `in_progress` 状態は放置せず、完了、保留、または置き換え時に必ず更新する。

理由:

- コミット履歴や会話ログがなくても、次に読むべき TODO、現在の作業状態、直近の完了作業、必要な検証を把握できるようにする。
- エージェントが別セッションで再開しても、古いコンテキストに依存せず同じ運用判断をできるようにする。

## 次に進む場所

現在の実装ロードマップは [TODO.md](../TODO.md) の `線形実装ロードマップ`
だけを参照する。上から順に読み、最初の未完了項目を次の作業候補にする。
現時点の次候補は B4: x86 syscall / libc 境界。

優先度の高い設計監査は [docs/design-todo.md](design-todo.md) の D1 と D2:

- D1: CLI と command 境界
- D2: Artifact domain model
