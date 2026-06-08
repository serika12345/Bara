# プロジェクト進行履歴

この文書は、コミット履歴を読まなくても Bara がどのように進行してきたかを
把握できるようにするための進行記録である。

詳細な実装 TODO は [TODO.md](../TODO.md)、詳細設計とリファクタリング TODO は
[docs/design-todo.md](design-todo.md)、`hello world` までの詳細な段階履歴は
[docs/hello-world-roadmap.md](hello-world-roadmap.md) に置く。

## 現在の作業スナップショット

最終更新: 2026-06-08 09:25 JST

状態:

- project_state: completed。B1: Hello World 成果物の安定化が完了し、review gate 待ち。
- active_milestone: completed。[TODO.md](../TODO.md) の B1 は全項目完了。
- active_design_focus: planned。次の設計監査入口は [docs/design-todo.md](design-todo.md) の D1 と D2。
- active_branch: `task/b1-executable-smoke-blackbox-report`。B1 完了 branch。
- related_todo: B1 `docs/hello-world-roadmap.md` 完了済みロードマップ整理 completed。
- completed_work: `docs/hello-world-roadmap.md` を完了済み履歴として整理し、B1 安定化成果と B2: 実行可能成果物モデルへの接続を明記した。
- remaining_work: B1 なし。レビュー後の次フェーズは B2。
- next_action: `/review-gate` で B1 branch をレビューし、承認後に `/merge-reviewed` で `main` へ取り込む。次に進める場合は B2 の最小 TODO-backed step から始める。
- verification: `nix develop -c ./scripts/check-no-invisible-chars`、`git diff --check`、`nix develop -c ./scripts/verify` が通過した。

直近で完了した作業:

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
- native artifact の stable report と failure classification を整える。
- CLI 肥大化を抑え、artifact domain model を明確にする。
- x86 32-bit、user-space runtime、platform abstraction、LLVM/Wasm 副出力を見越して設計を固定しすぎない。

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

- [TODO.md](../TODO.md) は実装マイルストーンと大項目 TODO を管理する。
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

現在の実装ロードマップは [TODO.md](../TODO.md) の B1 から進める。

優先度の高い設計監査は [docs/design-todo.md](design-todo.md) の D1 と D2:

- D1: CLI と command 境界
- D2: Artifact domain model
