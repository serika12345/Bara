# hello world までのマイルストーン

## 目的

この文書は、現在の raw x86_64 function bytes 実行から、最小の
`hello world` 相当を外部観測できるところまでの道筋を定義する。

ここでの `hello world` は、最初はプロセス全体や loader を扱わない。
raw function fixture が runtime 境界を通じて stdout に
`hello world\n` を出し、`actual.json` / `expected.json` で比較できる状態を
目標にする。

## 現在地

現在は以下を扱える。

- raw x86_64 function bytes
- entry offset `0`
- 引数なし
- `u64` 引数 1 個
- pointer 引数 1 個と read-only input memory
- Bara 専用 stdout host trap
- raw x86 function bytes 上の Bara 専用 stdout sentinel
- `rax` return value
- `mov rax, rdi`
- `movzx eax, byte ptr [rdi]`
- `mov eax, imm32`
- `add eax, imm8` / `add eax, imm32`
- `sub eax, imm8` / `sub eax, imm32`
- `xor eax, eax`
- `ret`
- ARM64 native runner による `u64` 戻り値比較
- file-based corpus fixture と `actual.json` / `report.json` 出力
- stdout / stderr / return_value の expected / actual 比較
- Bara executable manifest v0 から raw function pipeline への変換
- `check-executable <manifest.json> <expected.json>`

## マイルストーン

### HW0: no-args integer corpus の安定化

目的:

- 現在の no-args / `rax` return fixture を増やして、decode / lift / emit の
  最小 pipeline を安定させる。

成功条件:

- add/sub/xor の単独 fixture と複合 fixture が blackbox corpus で通る。
- decode / runtime integration tests が肥大化しないよう分割されている。

状態:

- 完了。

### HW1: 整数引数 ABI

目的:

- raw function に `u64` 引数を渡し、`rax` return value として観測できるようにする。

必要なもの:

- testcase ABI で `args: ["u64"]` を表現する。
- expected / actual JSON に引数値を保存するか、少なくとも testcase から runner へ渡す。
- x86_64 側は System V の第 1 引数 `rdi` を扱う。
- ARM64 側は native 第 1 引数 `x0` を使い、x86 `rdi` 相当として lift / emit できる。

最初の fixture:

```text
mov rax, rdi
ret
```

成功条件:

- `identity_u64` fixture が `return_value` で入力引数を返す。
- no-args fixture と one-arg fixture が同じ corpus runner で比較できる。

状態:

- 完了。

### HW2: 最小 memory read

目的:

- x86 function が pointer 引数から byte / qword を読めるようにする。

必要なもの:

- testcase に input memory bytes を表現する。
- runner が read-only input memory を用意し、pointer を x86 引数として渡す。
- IR に typed memory load を追加する。
- ARM64 emit が base pointer + offset の load を出せる。

最初の fixture:

```text
movzx eax, byte ptr [rdi]
ret
```

成功条件:

- input memory の先頭 byte を `return_value` として比較できる。

状態:

- 完了。

### HW3: stdout host trap

目的:

- runtime 境界で clean-room な stdout trap plan を扱い、stdout を
  `actual.json` に保存できるようにする。

方針:

- OS syscall を直接再現しない。
- 最初は clean-room な Bara 専用 helper/trap ABI を定義する。
- この段階では testcase の `host_traps` metadata で stdout 出力を宣言する。
- x86 側命令列からの helper call / sentinel instruction sequence 連携は、
  HW4 の raw function hello world で最小対応する。

成功条件:

- fixture が stdout に任意の短い ASCII 文字列を出せる。
- stdout / stderr / return_value の比較が通る。

状態:

- 完了。

### HW4: raw function hello world

目的:

- raw function fixture から `hello world\n` を stdout に出す。

必要なもの:

- HW2 の memory read または fixture data pointer。
- HW3 の stdout host trap。
- x86 側命令列から host trap を要求する helper call または sentinel sequence。
- stdout を含む expected / actual comparison。

成功条件:

```json
{
  "stdout": "hello world\n",
  "stderr": "",
  "return_value": 0
}
```

状態:

- 完了。

### HW5: loader 付き hello world

目的:

- ELF / Mach-O / PE などの実行ファイルを入力として扱う検討を始める。

注意:

- これは raw function fixture の hello world とは別段階。
- loader、relocation、imports、process memory、OS ABI が必要になるため、
  現在の初期スコープとは分けて扱う。

分割:

- HW5a: Bara executable manifest v0
- HW5b: executable image / segment model
- HW5c: entry point と process-like run result
- HW5d: host helper import table
- HW5e: public binary format の最小 probe

### HW5a: Bara executable manifest v0

目的:

- OS の実行ファイル形式へ入る前に、loader 境界の最小入力形式を定義する。
- raw function fixture と同じ bytes / abi / host_traps を、Bara 専用 executable
  manifest として読み込めるようにする。

方針:

- ELF / Mach-O / PE はまだ parse しない。
- manifest は clean-room な Bara 独自 JSON とする。
- manifest parser は filesystem I/O を持たず、文字列から typed executable
  fixture へ変換する。
- CLI や corpus runner の filesystem I/O は境界層に閉じる。

最初の fixture:

```text
manifest
  -> entry function bytes: ud2; xor eax, eax; ret
  -> host_traps stdout: "hello world\n"
  -> expected stdout / return_value
```

成功条件:

- `hello_world_executable_manifest` が existing raw function pipeline へ変換され、
  stdout `hello world\n`、`return_value` 0 として比較できる。
- manifest parser の失敗理由が分類されている。

状態:

- 完了。

### HW5b: executable image / segment model

目的:

- manifest 内の bytes を単なる function bytes ではなく、entry point を持つ
  executable image として扱う。

必要なもの:

- code segment と entry offset の domain type。
- section / segment の最小 model。
- entry が code segment 範囲内にあることの validation。

成功条件:

- entry offset 付き image から既存 decode/lift/emit pipeline へ渡せる。

### HW5c: entry point と process-like run result

目的:

- function-level runner と process-like runner の境界を分ける。

必要なもの:

- executable entry を起動する API。
- exit status / return value / stdout / stderr の扱いを明確化する型。
- raw function runner との重複を避ける委譲。

成功条件:

- manifest executable の実行結果を `actual.json` として保存できる。

### HW5d: host helper import table

目的:

- sentinel だけでなく、manifest が利用する host helper を明示する。

必要なもの:

- `write_stdout` 相当の Bara helper import。
- helper id / name / signature の typed representation。
- 未宣言 helper を使った場合の validation error。

成功条件:

- stdout helper が manifest に宣言され、実行時 trap plan と対応づく。

### HW5e: public binary format の最小 probe

目的:

- Bara manifest で固めた境界を、公開仕様に基づく実ファイル形式へ接続する
  検討を開始する。

方針:

- 最初は parse probe のみ。実行までは目標にしない。
- public spec に基づく magic / header / entry metadata の読み取りに限定する。
- format-specific parser は executable image model へ変換する境界として扱う。

成功条件:

- ELF / Mach-O / PE のうち 1 形式について、最小 header を分類して
  unsupported-but-recognized として報告できる。

## 判断基準

- 先に raw function で外部観測を増やす。
- syscall / libc / loader は、host trap と memory model が安定するまで扱わない。
- flags、stack、call は、hello world に必要になった時点で最小対応する。
- ファイルが肥大化し始めたら、次の命令追加前に責務別に分割する。
