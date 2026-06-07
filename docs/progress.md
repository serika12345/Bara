# プロジェクト進行履歴

この文書は、コミット履歴を読まなくても Bara がどのように進行してきたかを
把握できるようにするための進行記録である。

詳細な実装 TODO は [TODO.md](../TODO.md)、詳細設計とリファクタリング TODO は
[docs/design-todo.md](design-todo.md)、`hello world` までの詳細な段階履歴は
[docs/hello-world-roadmap.md](hello-world-roadmap.md) に置く。

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

## 次に進む場所

現在の実装ロードマップは [TODO.md](../TODO.md) の B1 から進める。

優先度の高い設計監査は [docs/design-todo.md](design-todo.md) の D1 と D2:

- D1: CLI と command 境界
- D2: Artifact domain model
