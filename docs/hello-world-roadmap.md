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
- `rax` return value
- `mov rax, rdi`
- `mov eax, imm32`
- `add eax, imm8` / `add eax, imm32`
- `sub eax, imm8` / `sub eax, imm32`
- `xor eax, eax`
- `ret`
- ARM64 native runner による `u64` 戻り値比較
- file-based corpus fixture と `actual.json` / `report.json` 出力

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

### HW3: stdout host trap

目的:

- translated function から runtime 境界へ明示的に出力要求を渡し、stdout を
  `actual.json` に保存できるようにする。

方針:

- OS syscall を直接再現しない。
- 最初は clean-room な Bara 専用 helper/trap ABI を定義する。
- x86 側命令列は helper call か sentinel instruction sequence によって
  runtime に `write_stdout(ptr, len)` 相当を要求する。

成功条件:

- fixture が stdout に任意の短い ASCII 文字列を出せる。
- stdout / stderr / return_value の比較が通る。

### HW4: raw function hello world

目的:

- raw function fixture から `hello world\n` を stdout に出す。

必要なもの:

- HW2 の memory read または fixture data pointer。
- HW3 の stdout host trap。
- stdout を含む expected / actual comparison。

成功条件:

```json
{
  "stdout": "hello world\n",
  "stderr": "",
  "return_value": 0
}
```

### HW5: loader 付き hello world

目的:

- ELF / Mach-O / PE などの実行ファイルを入力として扱う検討を始める。

注意:

- これは raw function fixture の hello world とは別段階。
- loader、relocation、imports、process memory、OS ABI が必要になるため、
  現在の初期スコープとは分けて扱う。

## 判断基準

- 先に raw function で外部観測を増やす。
- syscall / libc / loader は、host trap と memory model が安定するまで扱わない。
- flags、stack、call は、hello world に必要になった時点で最小対応する。
- ファイルが肥大化し始めたら、次の命令追加前に責務別に分割する。
