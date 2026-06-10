# Rosetta Oracle 検証ワークフロー

## 目的

Rosetta をブラックボックス oracle として使い、自前 BTB の外部挙動を検証する。

Rosetta の内部情報は使わない。比較するのはテスト harness が出力した結果 JSON のみ。

## 初期フロー

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
  +-- btbc compile raw x86_64 bytes
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

## testcase

最初は raw x86_64 bytes を直接持つ。

```json
{
  "case_id": "return_42",
  "entry": 0,
  "bytes": "b82a000000c3",
  "abi": {
    "args": [],
    "return": "u64"
  }
}
```

## expected.json

Rosetta で x86_64 harness を実行して得る。

```json
{
  "case_id": "return_42",
  "exit_status": 0,
  "return_value": 42,
  "stdout": "",
  "stderr": ""
}
```

## actual.json

自前 BTB 出力を ARM64 native runner で実行して得る。

```json
{
  "case_id": "return_42",
  "exit_status": 0,
  "return_value": 42,
  "stdout": "",
  "stderr": ""
}
```

## 比較

M1 では以下だけを比較する。

- `case_id`
- `exit_status`
- `return_value`
- `stdout`
- `stderr`

後で追加するもの:

- final `CpuState`
- touched memory
- fault kind
- signal/exception information
- PC map consistency

## x86_64 oracle runner

役割:

- testcase の bytes を x86_64 実行可能メモリに置く。
- x86_64 プロセスとして Rosetta 上で実行する。
- 関数として呼び出す。
- 戻り値を JSON で出す。

注意:

- oracle runner 自体は x86_64 Mach-O としてビルドする。
- arm64 プロセス内で x86_64 コードを直接実行しない。
- 初期は引数なし関数だけを対象にする。

## ARM64 BTB runner

役割:

- Rust BTB が生成した ARM64 machine code を executable buffer に置く。
- ARM64 native 関数として呼び出す。
- 戻り値を JSON で出す。

## コマンド案

```text
btbc-cli build-x86_64-macho-fixture tests/cases/return_42.json target/bara-oracle/x86_64/return_42
btbc-cli build-x86_64-oracle-runner tests/cases/return_42.json target/bara-oracle/x86_64/return_42-oracle
btbc-cli generate-x86_64-expected tests/cases/return_42.json tests/expected/return_42.json
btbc-cli check-fixture tests/cases/return_42.json tests/expected/return_42.json
btbc-cli check-corpus tests/cases tests/expected --out target/bara-blackbox
./scripts/verify-blackbox
```

`build-x86_64-macho-fixture` は、初期 B7 では no-args / `u64` return かつ
host trap なしの testcase bytes を x86_64 Mach-O `_main` として assemble /
link する。引数 ABI、stdout host trap、JSON を出す oracle harness は後続の
x86_64 oracle runner で扱う。

`build-x86_64-oracle-runner` は、同じ初期 scope の testcase bytes を
x86_64 Mach-O runner に埋め込む。runner は x86_64 プロセス内で executable
memory を確保して testcase function を呼び出し、`ObservedResult` 互換 JSON を
stdout に出す。

`generate-x86_64-expected` は、一時 x86_64 oracle runner をビルドし、
arm64 macOS 上で Rosetta 経由の x86_64 プロセスとして実行する。runner stdout
だけを `ObservedResult` として parse し、正規化した JSON を指定された
`expected.json` path に保存する。Rosetta は testcase の外部観測結果を得る
black-box oracle としてだけ使い、runner の構造や内部情報は実装根拠にしない。

`check-corpus` は全 testcase を走査し、成功時は case 単位の JSON report
を出す。失敗がある場合も最後まで走査し、同じ JSON report を出して非ゼロ終了
する。

`--out` を指定すると、エージェントが後続ターンで読める成果物を保存する。

```text
target/bara-blackbox/
  report.json
  actual/<case_id>.json
  compiled/
  ir/
  pcmap/
```

将来の分割案:

```text
btbc-oracle-x64 testcase.json > expected.json
btbc compile testcase.json --emit-ir compiled.ir.json --emit-pcmap pcmap.json --out compiled.bin
btbc-run-arm64 compiled.bin > actual.json
btbc-compare expected.json actual.json
```

## 失敗時に保存するもの

- `testcase.json`
- `expected.json`
- `actual.json`
- `compiled.ir.json`
- `pcmap.json`
- `fixups.json`
- ARM64 disassembly
- failure classification

## failure classification

```text
InvalidTestCase
MissingExpected
InvalidExpected
DecodeError
LiftError
EmitError
RunError
ComparisonMismatch
UnsupportedInstruction
WrongReturnValue
WrongRegisterValue
WrongFlags
WrongMemory
WrongBranchTarget
WrongCallReturn
WrongExternalCall
RunnerCrash
OracleCrash
```
