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
btbc-cli generate-arm64-actual tests/cases/return_42.json target/bara-oracle/actual/return_42.json
btbc-cli compare-expected-actual tests/expected/return_42.json target/bara-oracle/actual/return_42.json
btbc-cli emit-fixture-artifacts tests/cases/return_42.json target/bara-oracle/compiled/return_42
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
CLI 実装では Rosetta runner の observation を process status、stdout、stderr に
限定し、`expected.json` に入る testcase behavior は runner stdout の
`ObservedResult` JSON だけから作る。runner stderr は runner failure の診断として
扱い、expected behavior には含めない。

`generate-arm64-actual` は、同じ testcase を Bara の decode / lift / ARM64 emit
経路に通し、対応 host では ARM64 native runner で実行する。実行結果は
`ObservedResult` として正規化し、指定された `actual.json` path に保存する。
この command は expected との比較は行わず、actual artifact の生成だけを担当する。

`compare-expected-actual` は、保存済みの `expected.json` と `actual.json` を
`ObservedResult` として parse し、M1 の比較対象フィールドだけを比較する。
一致時は空の `ComparisonReport` JSON を stdout に出し、不一致時は
`ComparisonMismatch` として非ゼロ終了する。

`emit-fixture-artifacts` は、testcase を Bara の decode / lift / ARM64 emit
経路に通し、`compiled.ir.json`、`pcmap.json`、`fixups.json`、`helpers.json`、
`artifact.report.json` を指定 directory に保存する。初期 schema は regression
用の stable JSON とし、fixups は ARM64 branch lowering で適用した source /
target / kind を記録する。`artifact.report.json` は state layout description、
cache validation identity、helper requirements をまとめる。

`check-corpus` は全 testcase を走査し、成功時は case 単位の JSON report
を出す。失敗がある場合も最後まで走査し、同じ JSON report を出して非ゼロ終了
する。

`--out` を指定すると、エージェントが後続ターンで読める成果物を保存する。
`check-blackbox --out` は generated executable smoke も実プロセスとして実行し、
その process exit status、stdout、stderr を `ObservedResult` として
`actual/<case_id>.json` に保存する。
raw testcase fixture では、同じ case id の compile artifact metadata を
`compiled/<case_id>/` に保存する。`actual/<case_id>.json` は外部観測結果を保持し、
artifact metadata は sidecar として同じ regression output bundle に含める。

```text
target/bara-blackbox/
  report.json
  actual/<case_id>.json
  native-artifacts/<case_id>
  compiled/<case_id>/compiled.ir.json
  compiled/<case_id>/pcmap.json
  compiled/<case_id>/fixups.json
  compiled/<case_id>/helpers.json
  compiled/<case_id>/artifact.report.json
  compiled/<case_id>/verifier.report.json
  failures/<case_id>/failure.json
  failures/<case_id>/testcase.json
  failures/<case_id>/expected.json
  failures/<case_id>/actual.json
```

将来の分割案:

```text
btbc-oracle-x64 testcase.json > expected.json
btbc compile testcase.json --emit-ir compiled.ir.json --emit-pcmap pcmap.json --out compiled.bin
btbc-run-arm64 compiled.bin > actual.json
btbc-compare expected.json actual.json
```

## 失敗時に保存するもの

`check-corpus --out` / `check-blackbox --out` は、失敗 fixture ごとに
`failures/<case_id>/` を作る。`failure.json` は stable failure classification、
message、final state comparison report、shrink status、corpus update action を持つ。
final state comparison report は expected / actual の外部観測結果を比較できた
失敗だけに保存する。raw testcase fixture では、保存できる範囲で以下も同じ
directory に置く。

- `testcase.json`
- `expected.json`
- `actual.json`

`compiled/<case_id>/` が存在する場合は、同じ case id の `compiled.ir.json`、
`pcmap.json`、`fixups.json`、`helpers.json`、`artifact.report.json` を failure
analysis に使う。現時点の shrink は自動実行せず、`failure.json` に
`not_attempted` として記録し、同じ failure kind を保ったまま人間または後続ツールが
testcase を最小化する。

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

## verifier 導入判断

2026-06-11 時点では、B7 に Haskell package、schema reader、small x86 semantics
interpreter は追加しない。Haskell toolchain を入れるには `flake.nix` と
supply-chain 検証範囲が広がるため、まず既存 Rust workspace 内で verifier を
進める。

直近の verifier は、`compiled/<case_id>/compiled.ir.json`、`pcmap.json`、
`fixups.json`、`artifact.report.json`、`verifier.report.json`、
`actual/<case_id>.json`、必要に応じて
`failures/<case_id>/failure.json` を読み、IR invariant、PC map invariant、
fixup consistency、final state comparison を stable report として返す。

B7 の初期 Rust verifier report は `verifier.report.json` として保存する。
現在の検査は、emit 後の PC map が全 IR block start の source PC を保持している
ことと、branch fixup の target が PC map source に解決でき、offset / source の
ARM64 PC が生成 code 内の命令 slot を指していること、比較失敗時の
`failure.json` が final state comparison report を保持することに限定する。

Haskell は、JSON schema が安定し、QuickCheck / Hedgehog による generator と
shrinker、または Rust 実装から独立した仕様モデルが必要になった時点で `spec/`
配下に追加する。その change では Nix dev shell、package metadata、
supply-chain 検証を同時に扱う。

QuickCheck / Hedgehog 導入前の B7 では、Rust workspace 内の
`bara_oracle::small_case` が deterministic な no-args/u64 小ケース集合と、
`mov eax, imm32; ret` を `return 0` へ縮める最初の shrink candidate plan を
提供する。この導線で JSON schema と failure package の形を先に固める。

CI lane は repo-local scripts として分ける。`scripts/verify-quick` は format /
security / domain type / cargo check / clippy / library unit tests を担当し、
`scripts/verify-native` は host-specific native artifact tests を含む workspace
tests を担当する。`scripts/verify-oracle` は `check-blackbox --out` 経路を通し、
失敗時は `target/bara-blackbox/failures/<case_id>/` に failure package を残す。
`scripts/verify-nightly` は deterministic small-case shrink tests と
`check-blackbox --out` を `target/bara-nightly/` に保存する。
