# クリーンルーム運用

## 目的

Rosetta はブラックボックス oracle としてのみ使う。実装は公開仕様と自前テストに基づいて育てる。

このプロジェクトでは、Rosetta の内部実装を再現することではなく、binary-to-binary compiler の汎用的な構成を研究する。

FEX-Emu、Box64、QEMU user-mode などの Linux user-space 変換レイヤーは、比較対象・調査対象として扱う。調査は公開ドキュメント、公開 README、外部から観測できる挙動に限定し、内部実装やコード構造を Bara の実装根拠にしない。

## 許可する情報

実装根拠として使ってよいもの:

- Intel/AMD ISA manual
- ARM Architecture Reference Manual
- System V ABI
- Windows x64 ABI
- Mach-O / PE / ELF の公開仕様
- 公開されている OS API 仕様
- 自前で作ったテストケース
- Rosetta に x86_64 バイナリを実行させて得た入出力結果
- FEX-Emu / Box64 / QEMU user-mode の公開ドキュメント
- 既存変換レイヤーを外部から実行して得た入出力結果

Rosetta から取得してよいもの:

- exit status
- stdout / stderr
- return value
- テスト harness が明示的に出力した JSON
- クラッシュしたかどうか

## Rosetta oracle 実行境界

B7 の x86_64 oracle runner は、arm64 macOS 上で x86_64 Mach-O プロセスとして
実行し、Bara 側は subprocess の public process observation だけを読む。
許可される入力は runner の process status、stdout、stderr に限定する。

`expected.json` へ正規化してよい値は、runner stdout に明示的に出力された
`ObservedResult` JSON の `case_id`、`exit_status`、`return_value`、`stdout`、
`stderr` だけである。runner stderr は runner 自体の診断として failure report に
保持してよいが、期待される testcase behavior には混ぜない。

Rosetta 実行によって得た結果は、公開 ISA / ABI 仕様に基づく実装の回帰 oracle として
使う。Rosetta の内部 layout、translation strategy、metadata、symbol、disassembly、
実行時 helper 構造は、設計判断や実装根拠として使わない。

## 禁止する情報

実装根拠として使わないもの:

- Rosetta バイナリの逆アセンブル結果
- Rosetta 内部 symbol 名
- Rosetta 内部 metadata format
- Apple 固有の非公開 ABI
- Rosetta の関数配置や制御フロー
- Rosetta バイナリ由来のコード構造
- FEX-Emu / Box64 / QEMU user-mode の内部実装を模倣すること
- 既存変換レイヤーのコード構造、内部 helper、内部 metadata を実装根拠にすること

## エージェントに渡す情報

コーディングエージェントへ渡してよいもの:

- 入力 x86_64 bytes
- 入力 x86_64 assembly
- Rust 実装が生成した IR
- Rust 実装が生成した ARM64 disassembly
- `expected.json`
- `actual.json`
- 差分
- 公開仕様に基づく命令仕様メモ
- 既存テスト

渡さないもの:

- Rosetta の逆アセンブル
- Rosetta の内部 symbol に基づく分析
- Rosetta 内部構造の推測を実装指示にしたもの

## 失敗ケース駆動の開発

許可される流れ:

```text
1. x86_64 testcase を作る
2. Rosetta で expected.json を作る
3. 自前 BTB で actual.json を作る
4. 差分を調べる
5. 公開 ISA/ABI 仕様に基づいて Rust 実装を直す
6. regression corpus に追加する
```

Rosetta の役割は「期待される外部挙動を返す実行環境」に限定する。
