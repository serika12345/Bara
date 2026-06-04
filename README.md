# Bara

<p align="center">
  <img src="logo.png" alt="Bara logo" width="160">
</p>

Bara は、x86_64 などの既存バイナリを ARM64 などへ変換する binary-to-binary compiler の研究プロジェクトです。

Rosetta の再実装ではなく、変換コア、runtime、検証系、OS/ABI/ローダー固有部分を分離し、Wine のような互換レイヤーへ接続しやすい構成を探ることを目的とします。

## 方針

- 初期実装はパフォーマンスより検証しやすさを優先する。
- 最初は raw x86_64 function bytes を関数単位で扱う。
- 最初の成功条件は `mov eax, 42; ret` 相当が 42 を返すこと。
- Rust を主実装にする。
- Haskell は仕様モデル、property test、検証器として使う。
- Rosetta はブラックボックス oracle としてのみ使い、内部実装を参考にしない。
- Linux user-space の既存変換レイヤーとして FEX-Emu、Box64、QEMU user-mode を研究対象にする。ただし、実装根拠は公開仕様と外部挙動に限定する。

## 初期スコープ

最初に扱うもの:

- x86_64 の小さな関数片
- 引数なし
- `rax` 戻り値
- 最小限の decode / IR / ARM64 emit / runner
- `expected.json` と `actual.json` の比較

最初に扱わないもの:

- PE/Wine 統合
- syscall
- libc import
- SSE / AVX / x87
- exception / unwind
- self-modifying code
- JIT/fallback
- 最適化

## 検証方針

arm64 macOS 上で x86_64 Mach-O を Rosetta 実行し、期待結果を JSON として取得します。

自前 BTB で同じ入力を ARM64 に変換し、native runner で実行した結果と比較します。

```text
testcase
  |
  +-- x86_64 Mach-O -> Rosetta -> expected.json
  |
  +-- BTB compile -> ARM64 native runner -> actual.json
                                  |
                                  v
                               compare
```

Rosetta から使う情報は、テスト harness が出力する外部挙動だけです。Rosetta の逆アセンブルや内部 symbol は実装根拠にしません。

## ドキュメント

- [TODO](TODO.md): 全体 TODO とマイルストーン
- [初期スコープ](docs/scope.md): M1 の対象、初期 ABI、扱わないもの
- [クリーンルーム運用](docs/clean-room.md): Rosetta をブラックボックス oracle として扱うルール
- [コーディングルール](docs/coding-rules.md): シグネチャで仕様を表す方針と `unsafe` 境界
- [初期 IR 設計](docs/ir.md): 初期 IR、型境界、invariant
- [Rosetta Oracle 検証ワークフロー](docs/test-oracle.md): expected/actual 比較の流れ
- [Public ABI / import boundary](docs/public-abi-import-boundary.md): public ABI、imports、host helpers、syscall 相当境界の clean-room 設計

## 当面のゴール

最初のマイルストーンは以下です。

```text
x86_64 bytes:
  b8 2a 00 00 00 c3

meaning:
  mov eax, 42
  ret

result:
  return_value == 42
```

これを Rust 実装で decode、IR 化、ARM64 emit、実行し、Rosetta oracle の結果と比較できる状態を作ります。

## Rust workspace

初期 workspace は M1 の関心ごとに分けています。

```text
crates/
  bara-isa-x86/   x86_64 raw bytes の decode / lift
  bara-ir/        typed IR と invariant validation
  bara-arm64/     ARM64 emit と PC map
  bara-runtime/   executable memory と generated function runner
  btbc-cli/       開発用の実行チェック入口
```

現在の最小テストは `b8 2a 00 00 00 c3` を decode、IR 化、ARM64 machine code 化し、対応 host では no-args `u64` 関数として実行します。

M1 の実行チェックだけを行う場合:

```sh
./scripts/check-m1
```

成功すると `actual.json` 相当の最小 JSON を標準出力へ出します。

## 開発メモ

必要なツールは Nix 経由で使い、グローバルインストールを前提にしません。

開発環境は Nix Flake で再現可能に定義します。Rust toolchain、formatter、linter、test runner、LLVM 系ツールは dev shell から使います。

```sh
nix develop
```

単発でコマンドを実行する場合:

```sh
nix develop -c cargo --version
nix develop -c ./scripts/verify
nix develop -c cargo fmt --all -- --check
nix develop -c ./scripts/check-no-invisible-chars
nix develop -c ./scripts/check-domain-types
nix develop -c ./scripts/verify-security
nix develop -c ./scripts/verify-supply-chain
nix develop -c ./scripts/verify-nix-package
nix develop -c cargo check --workspace --all-targets
nix develop -c cargo test --workspace
nix develop -c cargo run -p btbc-cli -- check-m1
```

通常のコード、script、設定、repository policy の変更では
`nix develop -c ./scripts/verify` を完了前に通します。

依存関係、`Cargo.lock`、`deny.toml`、`flake.nix` を変更した場合は
`./scripts/verify-supply-chain` を必ず実行します。詳細は
[Supply Chain Policy](docs/supply-chain.md) を参照してください。
セキュリティ関連の変更では `./scripts/verify-security` も実行します。

任意で pre-commit hook を入れる場合:

```sh
nix develop -c ./scripts/install-pre-commit-hook
```

direnv を使う場合は `.envrc` の `use flake` を有効にします。

エディタ間の基本整形は `.editorconfig` で揃えます。文字コード、改行、末尾改行、空白、インデントの最低限のルールは repository 側で管理し、言語固有の整形は `rustfmt` などに委譲します。

初期の実装では、`unsafe` は executable memory と function pointer 呼び出し境界に閉じ込めます。命令変換や IR 操作は、できるだけ型で仕様が読める構成にします。
