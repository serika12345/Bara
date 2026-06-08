# プロジェクト進行履歴

この文書は、コミット履歴を読まなくても Bara がどのように進行してきたかを
把握できるようにするための進行記録である。

詳細な実装 TODO は [TODO.md](../TODO.md)、詳細設計とリファクタリング TODO は
[docs/design-todo.md](design-todo.md)、`hello world` までの詳細な段階履歴は
[docs/hello-world-roadmap.md](hello-world-roadmap.md) に置く。

## 現在の作業スナップショット

最終更新: 2026-06-08 15:51 JST

状態:

- project_state: completed。B4 の 7 つ目の小ステップとして、
  libc / dyld / import 呼び出しを直接模倣せず、public symbol/import
  identity として扱う model を追加した。
- active_milestone: in_progress。[TODO.md](../TODO.md) の B4:
  x86 syscall / libc 境界。先頭 7 項目は完了し、次は unsupported
  syscall / external call の分類と report schema を安定させる小ステップ。
- active_design_focus: in_progress。[docs/design-todo.md](design-todo.md) の D4:
  Bara IR の責務と D5: Host helper / OS boundary に沿って、external
  imports を `ExternalSymbolImport` の public symbol identity として表現した。
- active_branch: `task/b4-syscall-ir-request`。base commit は `19eeedb`
  (`Make roadmap linear without losing milestones`)。latest commit は
  review package で確認する。
- related_todo: [TODO.md](../TODO.md) B4 の
  libc / dyld / import 呼び出しを直接模倣せず、public symbol/import model として扱う。
- completed_work: `bara-ir` に `ExternalSymbolImport`、`ExternalImportTarget`、
  `PublicSymbolImport`、`PublicLibcSymbol`、`PublicDyldSymbol` を追加した。
  `ExternalCallRequest` は unresolved symbol id だけでなく public symbol import
  identity を保持できる。`puts` / `write` / `dyld_stub_binder` は identity
  として残り、libc ABI や dyld loader behavior はまだ実行しない。
- remaining_work: B4 の unsupported boundary report schema を具体化する。
- next_action: B4 の最後の小ステップとして unsupported syscall / external call
  の分類と report schema を安定させる。
- verification: `nix develop -c cargo test -p bara-ir` が通過した。
  続く final B4 step で full `nix develop -c ./scripts/verify` を実行する。

直近で完了した作業:

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
