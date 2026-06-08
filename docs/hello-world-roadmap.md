# hello world までの完了済みマイルストーン

状態:

- 完了。最小 `hello world` milestone と、その後の B1: Hello World 成果物の
  安定化は完了済み。

## 目的

この文書は、raw x86_64 function bytes 実行から、最小の `hello world`
相当を外部観測できるところまでに通った道筋を記録する。

ここでの `hello world` は、最初はプロセス全体や loader を扱わない。
raw function fixture が runtime 境界を通じて stdout に
`hello world\n` を出し、`actual.json` / `expected.json` で比較できる状態を
最初の目標にした。

B1 では、その成果を standalone macOS ARM64 executable artifact として
安定化し、blackbox report、stable JSON report、failure classification、
責務別 CLI tests へ接続した。

## 完了時点の到達点

完了時点では以下を扱える。

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
- executable image の code segment と entry offset validation
- executable manifest の `write_stdout` host helper import declaration / validation
- `check-executable <manifest.json> <expected.json>`
- public binary format の最小 probe
- Mach-O 64-bit little-endian magic の recognized-but-unsupported 分類
- `probe-binary <path>` CLI による public binary probe
- `probe-binary <path>` の安定 JSON report 出力
- Mach-O 64-bit little-endian header の typed `filetype` metadata
- Mach-O 64-bit little-endian header の typed `ncmds` / `sizeofcmds` metadata
- Mach-O 64-bit little-endian load command table bounds validation
- Mach-O 64-bit little-endian unsupported load command summary
- Mach-O probe fixture / expected JSON と `check-binary-probe`
- Mach-O probe report 上の executable image 変換可否 metadata
- Mach-O 64-bit little-endian `LC_SEGMENT_64` metadata
- Mach-O 64-bit little-endian `LC_MAIN` entry point metadata
- Mach-O executable image conversion blocker classification
- Mach-O materialized executable image から no-args u64 raw function testcase への
  pure conversion
- `check-mach-o <binary> <expected.json>` による Mach-O backed raw function 実行
- `check-mach-o-host-traps <binary> <host-traps.json> <expected.json>` による
  Mach-O backed stdout host trap 実行
- x86 `call rel32` の typed decode と classified unsupported lift
- `TestCase` 上の typed stack state metadata
- Mach-O `LC_MAIN.stacksize` から testcase stack metadata への pure conversion
- public ABI / import / host helper / syscall 相当境界の clean-room 計画文書
- x86 `syscall` の typed decode と classified unsupported lift
- raw testcase から ARM64 machine code artifact への実行なし変換境界
- raw testcase から macOS ARM64 executable artifact への toolchain packaging 境界
- standalone macOS ARM64 executable artifact による実 OS stdout emission
- generated executable smoke test を含む stable blackbox report
- `link-fixture-arm64-stdout-main` の stable `ObservedResult` JSON report
- native artifact packaging / toolchain / execution の failure classification
- native artifact command behavior の責務別 CLI test module

## 次フェーズとの接続

B1 完了時点で、fixture 専用の successful path は以下まで安定した。

- raw ARM64 bytes を保存できる。
- host-trap-free fixture を macOS ARM64 executable として link できる。
- stdout host trap fixture を native `_write` prologue 付き executable として
  link し、実 OS stdout / exit status を JSON report にできる。
- `check-blackbox --out` は生成 executable smoke を含む stable report を返す。

次の実装入口は [TODO.md](../TODO.md) の B2: 実行可能成果物モデル。
B2 では、この文書で到達した CLI I/O 境界をそのまま広げるのではなく、
raw ARM64 bytes、assembly source、linked executable、execution report を
別の domain type として扱い、artifact metadata と toolchain boundary を
明確にする。

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
- HW6: public binary probe の I/O 境界

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

状態:

- 完了。

### HW5c: entry point と process-like run result

目的:

- function-level runner と process-like runner の境界を分ける。

必要なもの:

- executable entry を起動する API。
- exit status / return value / stdout / stderr の扱いを明確化する型。
- raw function runner との重複を避ける委譲。

成功条件:

- manifest executable の実行結果を `actual.json` として保存できる。

状態:

- 完了。

### HW5d: host helper import table

目的:

- sentinel だけでなく、manifest が利用する host helper を明示する。

必要なもの:

- `write_stdout` 相当の Bara helper import。
- helper id / name / signature の typed representation。
- 未宣言 helper を使った場合の validation error。

方針:

- executable manifest の `imports` に `write_stdout` host helper を宣言する。
- `host_traps` で stdout を要求する manifest は、`write_stdout` import を必須にする。
- import table は manifest parser 境界で検証し、runtime には既存の trap plan だけを渡す。

成功条件:

- stdout helper が manifest に宣言され、実行時 trap plan と対応づく。

状態:

- 完了。

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

状態:

- 完了。

### HW6: public binary probe の I/O 境界

目的:

- core の binary format probe を、CLI / filesystem 境界から使えるようにする。
- 実行や loader 変換へ進む前に、未知形式や未対応形式を安定して報告する。

分割:

- HW6a: `probe-binary <path>` CLI
- HW6b: probe report JSON
- HW6c: Mach-O header metadata の最小 typed field
- HW6d: probe fixture corpus
- HW7: Mach-O load command envelope

### HW6a: `probe-binary <path>` CLI

目的:

- ファイルから public binary bytes を読み込み、既存の binary format probe に渡す
  I/O 境界を作る。

方針:

- filesystem access は `btbc-cli` に閉じる。
- `bara-oracle::binary_format` は純粋な bytes probe のままにする。
- CLI は実行や loader 変換を行わず、recognized-but-unsupported / error を
  user-visible に返すだけにする。

成功条件:

- Mach-O magic を持つ fixture file に対して `probe-binary` が
  recognized-but-unsupported を報告する。
- 短すぎる入力、unknown magic は分類された CLI error として扱う。

状態:

- 完了。

### HW6b: probe report JSON

目的:

- `probe-binary <path>` の成功出力を、機械的に比較しやすい安定 JSON にする。

方針:

- JSON serialization は `bara-oracle::json` の純粋関数へ寄せる。
- `btbc-cli` は filesystem I/O と probe 呼び出しだけを担当し、JSON 文字列を
  ad hoc に組み立てない。
- 失敗時は既存の分類 error を維持する。

成功条件:

- Mach-O magic を持つ fixture file に対して `probe-binary` が
  `{"format":"mach_o_64_little_endian","status":"recognized_but_unsupported"}`
  を返す。
- probe report serializer の単体テストで JSON field と enum 名が固定される。

状態:

- 完了。

### HW6c: Mach-O header metadata の最小 typed field

目的:

- magic だけでなく、公開 Mach-O header の最小 metadata を typed value として
  probe report に含める。

方針:

- 実行や loader 変換はしない。
- まず Mach-O 64-bit little-endian header の `filetype` だけを扱う。
- `filetype` の primitive 値は parser 境界で enum / classified unsupported に変換する。
- header bytes の不足は magic 不足とは別の分類 error とする。

成功条件:

- Mach-O 64-bit little-endian executable header を probe すると、report JSON に
  file type metadata が含まれる。
- 未対応 filetype は分類 error または unsupported metadata として扱い、panic しない。

状態:

- 完了。

### HW6d: probe fixture corpus

目的:

- public binary probe を file-based fixture / expected JSON で回帰確認できるようにする。

方針:

- fixture binary は self-authored な最小 header bytes とする。
- expected は `probe-binary` と同じ stable JSON report にする。
- 比較 I/O は CLI / scripts 境界に閉じ、`bara-oracle::binary_format` は純粋 probe のままにする。
- 実行、loader 変換、load commands parse はしない。

成功条件:

- Mach-O executable header fixture と expected probe JSON が repository にある。
- binary probe fixture を検証する CLI または script があり、`verify-blackbox` から通る。

状態:

- 完了。

### HW7: Mach-O load command envelope

目的:

- Mach-O header の load command table を、実 loader に入る前の typed metadata として
  検証できるようにする。

分割:

- HW7a: `ncmds` / `sizeofcmds` typed metadata
- HW7b: load command table bounds validation
- HW7c: unsupported load command summary
- HW8: Mach-O segment command metadata

### HW7a: `ncmds` / `sizeofcmds` typed metadata

目的:

- Mach-O 64-bit little-endian header から load command count と command byte size を
  typed value として取り出し、probe report に含める。

方針:

- load command 本体はまだ parse しない。
- `ncmds` / `sizeofcmds` の primitive 値は parser 境界で newtype に変換する。
- 0 commands / 0 command bytes の扱いは、分類 error ではなく typed metadata とする。
- 実行、loader 変換、segment extraction はしない。

成功条件:

- Mach-O executable header fixture の probe JSON に load command count / size が含まれる。
- public primitive API を増やさず、domain type と serializer で表現する。

状態:

- 完了。

### HW7b: load command table bounds validation

目的:

- Mach-O header が宣言する load command table の byte range が、入力 binary 内に
  収まることを検証する。

方針:

- load command 本体の種類や内容はまだ parse しない。
- table start は 64-bit Mach-O header 直後とし、`sizeofcmds` で終端を決める。
- range overflow / input 不足は分類 error とする。
- `sizeofcmds == 0` は valid な empty table として扱う。

成功条件:

- `sizeofcmds == 0` の既存 fixture は valid のまま通る。
- `sizeofcmds` が入力長を超える fixture は分類 error になる。

状態:

- 完了。

### HW7c: unsupported load command summary

目的:

- Mach-O load command table の各 command envelope を読み、未対応 command として
  summary metadata に残す。

方針:

- command の意味解釈や segment extraction はまだしない。
- 各 command は `cmd` と `cmdsize` だけを typed value として読む。
- command range が table 内に収まらない場合は分類 error とする。
- `ncmds == 0` は empty summary として扱う。
- 実行、loader 変換、relocation、imports はしない。

成功条件:

- unknown load command を含む Mach-O fixture を probe すると、report JSON に
  unsupported command summary が含まれる。
- malformed command size / range は分類 error になる。

状態:

- 完了。

### HW8: Mach-O segment command metadata

目的:

- unsupported summary だけでなく、公開 Mach-O `LC_SEGMENT_64` command の
  envelope-level metadata を typed value として読めるようにする。

分割:

- HW8a: `LC_SEGMENT_64` command kind recognition
- HW8b: segment name / vmaddr / fileoff / filesize metadata
- HW8c: executable image への変換可否 report

### HW8a: `LC_SEGMENT_64` command kind recognition

目的:

- Mach-O load command の `cmd` が `LC_SEGMENT_64` の場合、unsupported command ではなく
  recognized segment command として summary に分類する。

方針:

- segment contents、sections、VM mapping、entry point 変換はまだ扱わない。
- まず command kind と byte size だけを typed summary に含める。
- `cmd` の primitive 値は parser 境界で enum / newtype に変換する。

成功条件:

- `LC_SEGMENT_64` command を 1 つ含む fixture を probe すると、
  report JSON に recognized segment command summary が含まれる。
- unknown command はこれまで通り unsupported summary に残る。

状態:

- 完了。`LC_SEGMENT_64` は command kind と byte size のみを
  `recognized_segments` summary に分類する。

### HW8b: segment name / vmaddr / fileoff / filesize metadata

目的:

- 公開 Mach-O `segment_command_64` header から segment name、`vmaddr`、
  `fileoff`、`filesize` を typed metadata として probe report に含める。

方針:

- sections、protection flags、`nsects`、flags、VM mapping、entry point 変換は
  まだ扱わない。
- segment name は 16 byte fixed field として読み、最初の NUL までを UTF-8 として
  JSON 文字列にする。
- UTF-8 として不正な segment name は silent replacement せず、分類 error にする。

成功条件:

- `LC_SEGMENT_64` command を 1 つ含む fixture bytes を probe すると、
  `recognized_segments` に `name`、`vmaddr`、`fileoff`、`filesize` が出る。
- `LC_SEGMENT_64` の `cmdsize` が public `segment_command_64` header の 72 bytes
  未満なら `LoadCommandTooSmall` として reject する。

状態:

- 完了。`LC_SEGMENT_64` の command header metadata を typed value として読み、
  stable JSON report に含める。

### HW8c: executable image への変換可否 report

目的:

- Mach-O probe の結果が、Bara の executable image model へ変換可能かどうかを
  loader 実装前の typed metadata として報告する。

方針:

- 実際の executable image 変換、entry point 抽出、VM mapping はまだ行わない。
- 現時点では Mach-O entry point load command を扱っていないため、
  `not_convertible` / `missing_entry_point` として分類する。
- 変換可否は probe report の metadata に含め、CLI は JSON をそのまま安定出力する。

成功条件:

- Mach-O executable header fixture を probe すると、
  `executable_image_conversion` に変換不可理由が出る。
- 判定は domain type と serializer で表現し、ad hoc JSON 文字列生成を増やさない。

状態:

- 完了。entry point metadata 未対応の Mach-O probe は
  `not_convertible` / `missing_entry_point` として stable JSON report に含める。

### HW8d: Mach-O entry point command metadata

目的:

- 公開 Mach-O `LC_MAIN` load command を unsupported command ではなく、
  executable entry point metadata として probe report に含める。

方針:

- `entry_point_command` の公開 layout に基づき、`entryoff` と `stacksize` のみを
  typed metadata として読む。
- 実際の executable image 変換、VM mapping、section parsing、loader execution、
  syscall、import はまだ扱わない。
- `cmdsize` が公開 `entry_point_command` の 24 bytes 未満なら
  `LoadCommandTooSmall` として reject する。

成功条件:

- `LC_MAIN` を含む Mach-O fixture を probe すると、
  `recognized_entry_points` に `entryoff` と `stacksize` が出る。
- `LC_MAIN` は `unsupported_commands` には分類されない。
- entry point がある Mach-O probe は、entry point がない場合とは別の
  conversion blocker として分類できる。

状態:

- 完了。`LC_MAIN` の `entryoff` / `stacksize` metadata を typed value として読み、
  stable JSON report に含め、変換可否 blocker を entry point 有無から分類する。
  segment 有無による blocker の細分化は HW8e で扱う。

### HW8e: Mach-O conversion blocker for missing segment metadata

目的:

- Mach-O executable image conversion の blocker を、entry point 有無だけでなく
  recognized `LC_SEGMENT_64` metadata 有無でも分類する。

方針:

- 実際の executable image 変換、file range validation、`entryoff` と segment の
  対応付け、VM mapping、section parsing、loader execution、syscall、import は
  まだ扱わない。
- `LC_MAIN` があり、recognized `LC_SEGMENT_64` がない場合は
  `not_convertible` / `missing_segment` として報告する。
- `LC_MAIN` と recognized `LC_SEGMENT_64` が両方ある場合は、次に足りない
  capability として `not_convertible` / `unsupported_image_mapping` を報告する。

成功条件:

- entry point がない Mach-O probe は `missing_entry_point` のまま分類される。
- entry point があり recognized segment がない Mach-O probe は `missing_segment`
  として分類される。
- entry point と recognized segment がある Mach-O probe は
  `unsupported_image_mapping` として分類される。

状態:

- 完了。Mach-O executable image conversion blocker は entry point、recognized segment、
  image mapping capability の順に typed metadata として分類する。

### HW8f: Mach-O segment file range validation

目的:

- recognized `LC_SEGMENT_64` metadata の `fileoff` / `filesize` が、入力 binary の
  file byte range として成立することを検証する。

方針:

- `fileoff + filesize` の overflow と input length 超過を classified probe error として
  reject する。
- `filesize == 0` は empty file range として扱い、`fileoff` が input length 以下なら
  valid とする。
- VM range、protection、sections、`entryoff` と segment の対応付け、executable image
  変換はまだ扱わない。

成功条件:

- zero-size range at EOF は valid として recognized segment metadata に残る。
- nonzero range が input 内に収まる場合は valid として recognized segment metadata に残る。
- overflow または input 外の segment file range は
  `SegmentFileRangeOutOfBounds` として reject する。

状態:

- 完了。recognized `LC_SEGMENT_64` の file range は metadata assembly 時に検証され、
  executable image conversion や VM mapping には進まない。

### HW8g: Mach-O LC_MAIN entryoff file offset validation

目的:

- recognized `LC_MAIN` metadata の `entryoff` が、入力 binary 内の byte を指す
  file offset として成立することを検証する。

方針:

- `entryoff < input length` のみを検証し、`entryoff == input length` と input 外の
  offset は classified probe error として reject する。
- `entryoff` と recognized segment の対応付け、VM mapping、section parsing、
  executable image 変換、loader execution、syscall、import はまだ扱わない。

成功条件:

- input 内の `entryoff` は valid として recognized entry point metadata に残る。
- EOF または input 外を指す `entryoff` は
  `EntryPointFileOffsetOutOfBounds` として reject する。

状態:

- 完了。`LC_MAIN.entryoff` は metadata assembly 時に入力 byte を指す file offset として
  検証され、segment 対応付けや executable image conversion には進まない。

### HW8h: Mach-O entry point segment file range blocker

目的:

- recognized `LC_MAIN.entryoff` が recognized `LC_SEGMENT_64` file range に含まれるかを、
  executable image conversion metadata として分類する。

方針:

- entry point がなく recognized segment があるかどうかに関係なく、既存通り
  `missing_entry_point` を報告する。
- entry point があり recognized segment がない場合は、既存通り `missing_segment` を
  報告する。
- entry point と recognized segment があり、entry file offset がどの segment file
  range にも含まれない場合は `entry_point_outside_segment` を報告する。
- entry file offset が recognized segment file range に含まれる場合だけ、次の未実装
  capability として `unsupported_image_mapping` を報告する。
- zero-size segment は entry byte を含まないものとして扱う。
- VM address mapping、section parsing、code extraction、executable image creation、
  loader execution、syscall、import はまだ扱わない。

成功条件:

- entry point outside recognized segment は probe parse error ではなく
  `not_convertible` / `entry_point_outside_segment` として stable JSON に出る。
- entry point inside recognized segment は引き続き
  `not_convertible` / `unsupported_image_mapping` として分類される。

状態:

- 完了。Mach-O executable image conversion blocker は recognized entry point の
  file offset が recognized segment file range に含まれるかを分類する。

### HW8i: Mach-O ambiguous entry point blocker

目的:

- 複数の recognized `LC_MAIN` がある Mach-O executable を parse error ではなく、
  executable image conversion metadata の ambiguity として分類する。

方針:

- recognized entry point が 0 個なら既存通り `missing_entry_point` を報告する。
- recognized entry point が複数なら `ambiguous_entry_point` を報告する。
- recognized entry point が 1 個だけの場合に限り、recognized segment の有無、
  segment file range との対応、image mapping capability を既存順で分類する。
- 実際の executable image 変換、VM mapping、section parsing、loader execution、
  syscall、import はまだ扱わない。

成功条件:

- 複数の recognized `LC_MAIN` は probe parse error ではなく
  `not_convertible` / `ambiguous_entry_point` として stable JSON に出る。
- recognized entry point が 1 個だけの既存 blocker 分類は変わらない。

状態:

- 完了。Mach-O executable image conversion blocker は複数の recognized `LC_MAIN` を
  ambiguous entry point として分類する。

### HW8j: Mach-O ambiguous entry segment blocker

目的:

- 1 個の recognized `LC_MAIN.entryoff` が複数の recognized `LC_SEGMENT_64` file
  range に含まれる Mach-O executable を、parse error ではなく executable image
  conversion metadata の ambiguity として分類する。

方針:

- entry point が 0 個なら既存通り `missing_entry_point` を報告する。
- entry point が複数なら既存通り `ambiguous_entry_point` を報告する。
- entry point が 1 個だけで recognized segment がない場合は既存通り
  `missing_segment` を報告する。
- entry point が 1 個だけで containing segment が 0 個なら既存通り
  `entry_point_outside_segment` を報告する。
- entry point が 1 個だけで containing segment が複数なら
  `ambiguous_entry_segment` を報告する。
- entry point が 1 個だけで containing segment が 1 個なら既存通り
  `unsupported_image_mapping` を報告する。
- 実際の executable image 変換、VM mapping、section parsing、loader execution、
  syscall、import はまだ扱わない。

成功条件:

- 1 個の recognized entry point が複数の recognized segment file range に含まれる
  Mach-O probe は `not_convertible` / `ambiguous_entry_segment` として stable JSON
  に出る。
- 複数 containing segment は probe parse error として reject しない。

状態:

- 完了。Mach-O executable image conversion blocker は単一 entry point と複数
  containing segment の組み合わせを ambiguous entry segment として分類する。

### HW8k: Mach-O executable image conversion responsibility split

目的:

- Mach-O header parsing / load command metadata assembly と executable image conversion
  blocker classification の責務を分け、次の loader/image conversion 検討前に
  module boundary を明確にする。

方針:

- parsing logic は `mach_o.rs` に残す。
- executable image conversion metadata type と blocker classification は専用 module に
  移す。
- serialized JSON 名、既存の blocker 分類、public re-export は変えない。
- 実際の executable image 変換、VM mapping、section parsing、loader execution、
  syscall、import はまだ扱わない。

成功条件:

- `MachOMetadata::new` は parsed load command metadata から pure classifier を呼び、
  既存の stable JSON と blocker tests が変わらず通る。
- `mach_o.rs` は Mach-O header parsing と metadata assembly に集中する。

状態:

- 完了。Mach-O executable image conversion metadata と blocker classification を
  専用 module に分離し、既存の JSON behavior と public re-export を維持する。

## 次の大マイルストーン

ここから先は、細かい command metadata の追加ではなく、Mach-O を既存の
raw function / executable manifest pipeline に段階的に接続する。

### HW9: Mach-O executable image materialization

目的:

- recognized `LC_SEGMENT_64` / `LC_MAIN` metadata から、Bara の
  `ExecutableImage` / `ExecutableManifest` 相当へ変換できる最小経路を作る。

成功条件:

- 単一 entry point と単一 containing segment を持つ Mach-O fixture から、
  typed executable image が作れる。
- 変換不能な Mach-O は既存の blocker classification と classified error で止まる。
- section parsing、dynamic loader、imports、syscall、libc はまだ扱わない。

状態:

- 完了。Mach-O conversion metadata から materialization plan を作り、
  `BinaryInput` の segment bytes から既存 `ExecutableImage` を pure に作れる。

#### HW9a: Mach-O convertible image candidate

目的:

- 単一 `LC_MAIN` と、その entry point file offset を含む単一 `LC_SEGMENT_64` を、
  executable image materialization の変換可能候補として typed metadata に残す。

方針:

- classifier は pure のままにし、I/O、raw bytes extraction、`ExecutableImage` /
  `ExecutableManifest` 生成、runtime 実行はまだ行わない。
- blocker がある case は既存の `not_convertible` JSON を維持する。
- convertible case は selected entry point / segment を既存 metadata type で保持する。

成功条件:

- 単一 entry point と単一 containing segment は `convertible` status になる。
- convertible metadata から、選択された entry point と segment を確認できる。
- 既存 blocker tests と stable JSON tests が通る。

状態:

- 完了。Mach-O executable image conversion metadata が、単一 entry point と
  単一 containing segment を変換可能候補として表現できる。

#### HW9b: Mach-O executable image materialization plan

目的:

- 変換可能候補から、raw bytes extraction と executable manifest 生成に必要な
  typed materialization plan を pure に作る。

方針:

- plan は selected segment の file range と、entry point の segment-relative
  offset だけを保持する。
- raw bytes extraction、`ExecutableImage` / `ExecutableManifest` 生成、
  runtime 実行、CLI 追加はまだ行わない。
- 変換不能な候補から plan を要求した場合は、blocker とは別の plan 専用
  classified error で止める。

成功条件:

- convertible metadata から、segment file range と entry point relative offset を
  domain type で確認できる。
- not-convertible metadata から plan を作ろうとすると classified error になる。
- 既存 blocker tests と stable JSON tests が通る。

状態:

- 完了。Mach-O executable image materialization に必要な最小 plan を
  conversion metadata から pure に作れる。

#### HW9c: binary format test responsibility split

目的:

- 次の executable image materialization 実装前に、`binary_format` の巨大な
  inline tests を責務別 test module に分ける。

方針:

- production behavior、public API、JSON shape は変えない。
- probe、conversion、plan の tests を `binary_format/mod.rs` から外す。
- probe tests は header/input、load command、segment、entry point に分ける。

成功条件:

- `binary_format/mod.rs` は module declarations / public re-export /
  test module declaration に戻る。
- 既存 test names と assertions は維持される。
- 次の HW9 materialization 実装で、責務に合う test file を選びやすい。

状態:

- 完了。Mach-O probe / conversion / plan tests を責務別 module に分割し、
  HW9 materialization に進む前の test surface を整理する。

#### HW9d: Mach-O executable image materialization

目的:

- `BinaryInput` と `MachOExecutableImagePlan` から、既存の
  `ExecutableImage` を pure に作る。

方針:

- plan の segment file range で input bytes を切り出し、`CodeSegment` /
  `ExecutableEntry` / `ExecutableImage` に変換する。
- code segment base は既存 manifest と同じ `X86Va::new(0)` とし、entry は
  plan の segment-relative offset を使う。
- CLI、file I/O、runtime 実行、manifest JSON 生成、loader/import/syscall はまだ
  行わない。

成功条件:

- segment bytes containing `mov eax, 42; ret` から `ExecutableImage` を作り、
  entry 以降の function bytes を取り出せる。
- plan の file range が input 外なら materialization 専用 classified error で止まる。
- `binary_format/mod.rs` を肥大化させず、materialization tests は責務別 file に置く。

状態:

- 完了。Mach-O executable image plan から、既存 `ExecutableImage` を pure に
  materialize できる。

### HW10: Mach-O backed raw function execution

目的:

- Mach-O から取り出した code segment と entry point を、既存の raw function
  decode / lift / emit / runtime pipeline へ渡して実行する。

成功条件:

- Mach-O fixture 内の最小 x86 function が `return_value` として比較できる。
- 既存の `check-executable` と同等の expected / actual JSON 比較が Mach-O 入力でも
  できる。
- VM address と file offset の対応は domain type で表現され、primitive boundary が
  増えすぎない。

状態:

- 完了。`check-mach-o <binary> <expected.json>` で Mach-O fixture 内の raw
  x86 function を既存 decode / lift / emit / runtime pipeline へ渡し、stable
  expected / actual JSON で比較できる。

#### HW10a: Mach-O executable image entry function testcase

目的:

- materialized `ExecutableImage` の entry point 以降を、既存 raw function runner 用の
  no-args u64 `TestCase` に pure に変換する。

方針:

- `ExecutableImage::entry_function_bytes()` を使い、`CaseId` と
  `TestCaseAbi::NoArgsU64` を持つ `TestCase` を作る。
- `BinaryInput` slicing、file I/O、CLI、expected comparison、runtime 実行はまだ
  行わない。
- image error は Mach-O executable image entry function 専用 classified error に
  包む。

成功条件:

- entry offset を持つ `ExecutableImage` から、case id、ABI、entry 以降の x86 bytes が
  期待通りの `TestCase` を作れる。
- `binary_format/mod.rs` を肥大化させず、entry function 変換 test は責務別 file に置く。

状態:

- 完了。materialized Mach-O executable image を no-args u64 raw function
  `TestCase` へ pure に変換できる。

#### HW10b: Mach-O binary input entry function testcase pipeline

目的:

- Mach-O `BinaryInput` から no-args u64 raw function `TestCase` までを、pure な
  pipeline API で作れるようにする。

方針:

- `probe_public_binary_format`、executable image conversion、plan、
  materialization、entry function 変換を pipeline 専用 module で orchestration する。
- probe / Mach-O parser / materialization module には orchestration を戻さない。
- file I/O、CLI、expected comparison、runtime 実行はまだ行わない。
- pipeline 専用 classified error で Probe / Plan / Materialization /
  EntryFunction を区別する。

成功条件:

- Mach-O-like `BinaryInput` から、case id、ABI、entry 以降の x86 bytes が期待通りの
  `TestCase` を作れる。
- not-convertible な Mach-O は pipeline 専用 error の Plan 分類で止まる。
- `binary_format/mod.rs` を肥大化させず、pipeline test は責務別 file に置く。

状態:

- 完了。Mach-O `BinaryInput` から no-args u64 raw function `TestCase` までを pure に
  生成できる。

#### HW10c: Mach-O binary CLI execution

目的:

- Mach-O binary file から no-args u64 raw function `TestCase` を作り、既存 raw
  function runner と expected / actual JSON 比較へ接続する。

方針:

- CLI 境界で binary file を読み、`BinaryInput` に変換する。
- case id は binary path stem から `CaseId` を作る。
- `mach_o_entry_function_test_case` で `TestCase` を作り、既存 `run_test_case` を使う。
- file I/O、runtime 実行、JSON 比較は `btbc-cli` に留め、`bara-oracle` に戻さない。
- host trap、imports、syscalls、loader 拡張はまだ行わない。

成功条件:

- `check-mach-o <binary> <expected.json>` で Mach-O fixture 内の
  `mov eax, 42; ret` が `return_value: 42` として比較できる。
- blackbox verification に Mach-O backed raw function fixture が含まれる。
- CLI implementation は小さな command handler と helper に留まる。

状態:

- 完了。Mach-O binary file を既存 raw function runner へ渡し、stable expected /
  actual JSON で比較できる。

### HW11: Mach-O hello world via Bara host trap

目的:

- Mach-O fixture 内の x86 function から、Bara 専用 stdout host trap を通じて
  `hello world\n` を外部観測する。

成功条件:

- Mach-O backed execution の `stdout` / `stderr` / `return_value` が stable JSON で
  比較できる。
- host helper import declaration / validation は既存 manifest model と矛盾しない。
- 実 OS syscall や libc call にはまだ踏み込まない。

状態:

- 完了。Mach-O entry function `TestCase` に明示的な host trap plan を付け、
  `check-mach-o-host-traps <binary> <host-traps.json> <expected.json>` で
  Bara 専用 stdout sentinel から `hello world\n` を stable expected / actual JSON で
  比較できる。

#### HW11a: Mach-O entry host trap plan preservation

目的:

- Mach-O から materialize した entry function `TestCase` が、呼び出し側から渡された
  `TestCaseHostTrapPlan` を保持できるようにする。

方針:

- 既存の no-trap API は `TestCaseHostTrapPlan::none()` に委譲し、挙動を変えない。
- `BinaryInput` / `ExecutableImage` から `TestCase` への変換は pure のままにする。
- CLI、file I/O、runtime 実行、expected comparison はまだ行わない。

成功条件:

- Mach-O entry function `TestCase` に stdout host trap plan を保持できる。
- 既存の `mach_o_entry_function_test_case` は no-trap のまま維持される。

状態:

- 完了。Mach-O entry function 変換と pipeline に `*_with_host_traps` API を追加し、
  既存 API は no-trap plan へ委譲している。

#### HW11b: Mach-O host trap CLI execution

目的:

- Mach-O binary、明示的な host trap plan JSON、expected JSON を CLI 境界で読み、
  既存 raw function runner と comparison へ接続する。

方針:

- stdout plan は expected JSON から逆算せず、別の `host-traps.json` として明示する。
- `bara-oracle` は host trap plan JSON を pure に `TestCaseHostTrapPlan` へ変換する。
- `btbc-cli` は file I/O、runtime 実行、JSON 比較の境界に留める。
- OS syscall、libc call、loader import、relocation は扱わない。

成功条件:

- `check-mach-o-host-traps <binary> <host-traps.json> <expected.json>` で
  Mach-O fixture 内の `0f 0b; xor eax, eax; ret` が stdout `hello world\n`、
  `return_value: 0` として比較できる。
- blackbox verification に Mach-O backed hello world fixture が含まれる。

状態:

- 完了。Mach-O backed hello world fixture が blackbox verification に入り、
  explicit host trap plan 経由で stdout / stderr / return_value を比較できる。

### HW12: Minimal stack / call boundary

目的:

- loader 付き hello world に必要な範囲で、stack と call / return の境界を最小対応する。

成功条件:

- function-level execution の責務を壊さず、必要な stack state を typed input として
  表現できる。
- call target が未対応の場合は classified unsupported として止まる。
- decode / lift / emit / runtime の各責務が混ざらない。

状態:

- 完了。`call rel32` は typed target / return address を持つ decoded instruction として
  読み、lift では `DirectCallUnsupported` として classified unsupported に止める。
  stack state は `TestCase` の typed metadata として保持し、Mach-O `LC_MAIN.stacksize`
  から pure に変換できる。runtime の stack allocation や call execution はまだ
  行わない。

#### HW12a: Direct call unsupported boundary

目的:

- x86 `call rel32` を public ISA semantics に基づいて decode し、まだ実行せず
  classified unsupported として止める。

方針:

- `call rel32` の target は次命令 address と signed displacement から計算する。
- decoded instruction は `target` と `return_to` を `X86Va` で保持する。
- lift は direct call を `Terminator::Unsupported` に変換する。
- emit / runtime / stack mutation は変更しない。

成功条件:

- positive / negative displacement の `call rel32` target が typed address として decode される。
- direct call は `DirectCallUnsupported { target, return_to }` として lift される。
- existing raw function 実行経路は変わらない。

状態:

- 完了。direct call は decode / lift 境界で classified unsupported として扱われ、
  runtime execution には入らない。

#### HW12b: Testcase stack state metadata

目的:

- loader 付き hello world に必要になる stack requirement を、runtime 実行へ混ぜる前に
  testcase metadata として typed に表現する。

方針:

- general testcase stack 型を `bara-oracle` の testcase 領域に置く。
- `MachOEntryPointStackSize` は Mach-O 境界専用型のままにし、
  `TestCaseStackState` へ変換する。
- `TestCase::new` / `with_host_traps` は empty stack state を既定にし、既存挙動を保つ。
- runtime は stack metadata をまだ消費しない。

成功条件:

- `TestCase` は empty / explicit stack size を typed metadata として保持できる。
- explicit zero stack size は validation error として拒否される。
- Mach-O entry function pipeline は `LC_MAIN.stacksize` を testcase stack state として保持する。

状態:

- 完了。`TestCaseStackState` / `TestCaseStackSize` を追加し、Mach-O backed testcase が
  `LC_MAIN.stacksize` を保持できる。

### HW13: Public ABI / import boundary planning

目的:

- Wine-style compatibility layer との接続を見据え、public ABI、imports、host helper、
  syscall 相当の境界を clean-room に設計する。

成功条件:

- Rosetta や既存 translation layer の内部構造に依存しない boundary document がある。
- imports / syscalls / host helpers の責務が、runtime と oracle に混ざらず分かれている。
- 実装前に、許可された public spec と externally observable behavior の範囲が明記される。

分割:

- HW13a: `docs/public-abi-import-boundary.md` に clean-room な public ABI /
  import boundary planning を置く。

状態:

- 完了。`docs/public-abi-import-boundary.md` に、許可された実装根拠、
  forbidden sources、crate / layer ごとの責務、domain vocabulary、最初の実装順序、
  non-goals を明記した。
- imports / syscalls / host helpers は、runtime 実行前に typed declaration、
  validation、resolution、unsupported classification として扱う方針にした。

### HW14: Corpus expansion and regression gate

目的:

- Mach-O backed execution で増えた fixture を、長期的に壊れにくい corpus として管理する。

成功条件:

- raw function、manifest、Mach-O の fixture が同じ report model で比較できる。
- expected / actual JSON の stable schema が維持される。
- `scripts/verify` が新しい regression gate を含む。
- `check-blackbox` の `CorpusReport` JSON が expected report fixture として固定され、
  `scripts/verify-blackbox` で構造的に比較される。

#### HW14a: Existing fixture regression gate

目的:

- 既存の raw function corpus、executable manifest fixture、Mach-O backed execution
  fixture、Mach-O probe fixture を、通常の repository verification gate に載せる。

方針:

- CLI に新しい実行 orchestration を足さず、既存の `check-corpus`、
  `check-executable`、`check-mach-o`、`check-mach-o-host-traps`、
  `check-binary-probe` を script 境界で組み合わせる。
- regression output は `target/bara-blackbox` に閉じ、actual JSON を安定した場所に
  書き出す。
- loader、syscall、libc、Rosetta 内部観測は扱わない。

成功条件:

- `scripts/verify-blackbox` が既存 fixture をまとめて検証する。
- `scripts/verify` が `scripts/verify-blackbox` を含む。

状態:

- 完了。`scripts/verify-blackbox` は出力先を初期化してから既存 fixture を実行し、
  `scripts/verify` の通常 gate から呼ばれる。

#### HW14b: Unified blackbox fixture report

目的:

- 既存の raw function corpus、executable manifest、Mach-O backed execution、
  Mach-O probe fixture を、同じ `CorpusReport` / `FixtureReport` outcome model で
  まとめて比較できるようにする。

方針:

- `btbc-cli check-blackbox [--out <dir>]` を追加し、既存の
  `check-corpus`、`check-executable`、`check-mach-o`、
  `check-mach-o-host-traps`、`check-binary-probe` の責務を維持したまま
  orchestration だけを担当させる。
- observed result と probe report は異なる actual JSON schema のまま保存し、
  同一化する対象は fixture の pass/fail outcome report に限定する。
- regression output は引き続き `target/bara-blackbox` 配下に閉じる。

成功条件:

- `check-blackbox` が既存 fixture 全体の `CorpusReport` JSON を返す。
- `scripts/verify-blackbox` が `check-blackbox --out target/bara-blackbox` を使う。
- CLI tests が stable report JSON と schema 別 actual output を検証する。

状態:

- 完了。`check-blackbox` は raw corpus、manifest 2 件、Mach-O execution 2 件、
  probe 1 件を `CorpusReport` に集約し、actual output は observed/probe の
  schema を分けたまま保存する。

#### HW14c: Stable blackbox report fixture

目的:

- `check-blackbox` の `CorpusReport` JSON を expected fixture として固定し、
  expected / actual JSON schema の回帰を通常 gate で検出できるようにする。

方針:

- `btbc-cli` には追加機能を足さず、既存の
  `check-blackbox --out target/bara-blackbox` が書く `report.json` を使う。
- expected fixture は `tests/expected-reports/blackbox.json` に置く。
- 比較は `scripts/verify-blackbox` の I/O 境界に閉じ、`jq -S` で JSON を
  正規化してから比較する。

成功条件:

- `scripts/verify-blackbox` が `target/bara-blackbox/report.json` と
  `tests/expected-reports/blackbox.json` を比較する。
- report の fixture case id、outcome、schema が変わった場合に通常 gate が失敗する。

状態:

- 完了。`tests/expected-reports/blackbox.json` を expected report fixture として
  固定し、`scripts/verify-blackbox` が `jq -S` 正規化後の構造的比較で
  `CorpusReport` JSON の安定 schema を検証する。

HW14 全体の状態:

- 完了。raw function、manifest、Mach-O execution、Mach-O probe fixture は
  `check-blackbox` の共通 `CorpusReport` に集約され、actual JSON と report JSON の
  regression gate が `scripts/verify` から実行される。

### HW15: Unsupported syscall boundary classification

目的:

- x86_64 `syscall` 命令を public ISA に基づいて decode し、OS syscall 実行へ進まず
  lift 境界で classified unsupported として扱う。

成功条件:

- `0f 05` は typed decoded instruction として表現される。
- lift は syscall を `Terminator::Unsupported` に変換し、命令 address と
  next-instruction address を stable reason に保持する。
- OS syscall 番号、libc、loader、runtime 実行、ARM64 emit には踏み込まない。

#### HW15a: Minimal syscall unsupported boundary

目的:

- x86_64 `syscall` (`0f 05`) を decode / lift 境界だけで classified unsupported に
  止める。

方針:

- decoded instruction は syscall の typed variant として保持する。
- lift は `SyscallUnsupported { at, return_to }` に分類する。
- runtime、ARM64 emit、oracle、CLI は変更しない。

成功条件:

- `0f 05` が typed instruction として decode される。
- syscall は `SyscallUnsupported { at, return_to }` として lift される。
- syscall は実行経路に入らない。

状態:

- 完了。`syscall` は decode / lift 境界で classified unsupported として扱われ、
  runtime execution には入らない。

HW15 全体の状態:

- 完了。syscall は OS / libc / runtime 実行へ進まず、public ISA decode と lift 境界の
  classified unsupported value として固定された。

### HW16: Host helper import resolution plan

目的:

- manifest の host helper import declaration / validation を、runtime 実行前に確認できる
  pure な resolution plan へ進める。

成功条件:

- stdout host trap を持つ manifest は、`write_stdout` / `ptr_len_to_unit` import が
  resolved された domain value を公開する。
- missing import / duplicate / signature mismatch の既存 validation error を維持する。
- runtime、ARM64 emit、CLI、binary format は変更しない。

#### HW16a: Minimal host helper resolution value

目的:

- 既存 manifest host helper import validation を壊さず、`write_stdout` の解決結果を
  `ExecutableManifest` から確認できる pure domain value として保持する。

方針:

- `crates/bara-oracle/src/executable_manifest/host_helper.rs` に小さな resolution plan
  type を置き、host helper の責務を同ファイル内に閉じる。
- stdout host trap がある場合だけ `write_stdout` import を resolved にする。
- entry function 実行挙動は変えない。

成功条件:

- stdout host trap manifest は `write_stdout` / `ptr_len_to_unit` の resolved import を
  domain value として返す。
- stdout host trap がない manifest では、unused import を resolution plan に入れない。
- duplicate / missing import の既存エラーを保つ。

状態:

- 完了。`HostHelperResolutionPlan` / `ResolvedHostHelperImport` を追加し、
  `ExecutableManifest` が import table と別に resolution plan を保持・公開する。

HW16 全体の状態:

- 完了。stdout host trap を持つ manifest は、runtime 実行前に
  `write_stdout` / `ptr_len_to_unit` の resolved host helper import を pure な
  domain value として公開できる。既存の missing / duplicate / signature mismatch
  validation error と entry function 実行挙動は維持されている。

### HW17: Runtime preflight for resolved host helpers

目的:

- executable manifest の resolved host helper plan を、実行前境界で明示的に消費する。

成功条件:

- `check-executable` / `run_executable_manifest` 経路は、stdout host trap 実行前に
  manifest の resolved `write_stdout` helper を確認する。
- runtime unsafe、real host call、helper ABI 拡張、ARM64 emit、binary format、
  oracle manifest parser は変更しない。
- 正常 fixture の外部観測結果は変えない。

#### HW17a: Minimal executable host helper preflight

目的:

- `ExecutableManifest::host_helper_resolution_plan()` を CLI execution 境界で読み、
  stdout host trap がある場合に resolved `write_stdout` import を小さな preflight
  value として確認する。

方針:

- execution preflight は `crates/btbc-cli/src/executable_run.rs` に閉じる。
- preflight logic は `()` ではなく、将来 runtime host-call execution へ渡せる
  report value または classified error を返す。
- 既存 manifest parser validation と整合させ、正常 fixture の behavior は変えない。

成功条件:

- stdout host trap manifest は、function 実行前に `write_stdout` /
  `ptr_len_to_unit` の resolved helper を preflight で確認する。
- `check-executable` の既存 executable manifest fixture は preflight を通って実行される。
- missing resolved helper は execution preflight error として分類される。

状態:

- 完了。`run_executable_manifest` が entry function 実行前に
  `host_helper_resolution_plan()` を明示的に消費し、stdout host trap に対して
  resolved `write_stdout` / `ptr_len_to_unit` helper を確認する。

HW17 全体の状態:

- 完了。resolved host helper plan は `run_executable_manifest` の実行前境界で
  preflight value として消費される。runtime host-call execution、unsafe runtime
  boundary、helper ABI 拡張はまだ扱わない。

### HW18: Translated ARM64 artifact boundary

目的:

- 既存の in-memory runtime 実行から、変換成果物をファイルとして扱う段階へ進む。
- `x86 function bytes -> decode -> lift -> ARM64 emit` の結果を、実行せずに
  明示的な artifact として取り出せるようにする。

方針:

- まだ OS 実行可能ファイル形式は生成しない。
- `bara-runtime` には入れず、JIT 実行境界と artifact 出力境界を分ける。
- `bara-arm64` は machine code bytes 生成に集中させ、ファイル I/O を持たせない。
- stdout host trap を実 OS stdout と見なさない。standalone 実行可能化は次段階で扱う。

成功条件:

- raw testcase を ARM64 machine code bytes に変換し、CLI からファイルへ保存できる。
- 変換だけの経路は runtime unsafe / executable memory allocation / generated function call
  を行わない。
- host trap を要求する testcase は、standalone artifact としての未対応理由を分類する。

#### HW18a: Raw testcase ARM64 code export

目的:

- `TestCase` から `EmittedFunction` までを、実行なしで返す小さな境界を作る。
- CLI で raw testcase JSON から ARM64 code bytes を出力できるようにする。

方針:

- 既存の `run_test_case_function` は挙動を変えず、共通の compile-only helper に委譲する。
- CLI の filesystem I/O は `btbc-cli` に閉じる。
- 出力は raw ARM64 bytes とし、Mach-O / ELF / PE writer はまだ作らない。

成功条件:

- `emit-fixture-arm64 <case.json> <out.bin>` が、host trap を含まない fixture を
  ARM64 machine code bytes として保存する。
- stdout host trap fixture は、standalone artifact unsupported として分類 error になる。
- 既存 `check-*` / blackbox の外部観測結果は変えない。

状態:

- 完了。`emit-fixture-arm64 <case.json> <out.bin>` で raw testcase を
  実行せずに ARM64 machine code bytes へ変換し、CLI I/O 境界で保存できる。
  stdout host trap を要求する testcase は standalone artifact unsupported として
  分類 error になる。

HW18 全体の状態:

- 完了。変換成果物を raw ARM64 machine code artifact としてファイル出力できる
  境界ができた。OS 実行可能ファイル形式、実 OS stdout、process spawn による
  成果物実行は次の大項目で扱う。

### HW19: macOS ARM64 executable packaging smoke

目的:

- HW18 の raw ARM64 machine code artifact を、OS が起動できる成果物へ近づける。
- まずは host trap を含まない no-args `u64` return fixture を、macOS ARM64 の
  `_main` としてリンクし、実プロセスの exit status として観測する。

方針:

- 最初は汎用 Mach-O writer を作らない。
- Nix dev shell 上の public toolchain (`clang`) を CLI I/O 境界から呼び出す。
- `bara-arm64` / `bara-runtime` / input Mach-O probe parser へ toolchain orchestration を
  混ぜない。
- stdout host trap や実 OS stdout はまだ扱わない。`hello world` standalone 実行は
  次段階で、明示的な native stdout emission として切る。

成功条件:

- raw testcase から生成した ARM64 bytes を `_main` body としてリンクした executable
  artifact を作れる。
- 生成 artifact を実プロセスとして起動し、`return_42` が exit status 42 として
  観測できる。
- host trap を要求する testcase は standalone executable packaging unsupported として
  分類 error になる。

#### HW19a: Link host-trap-free fixture as ARM64 main

目的:

- `emit-fixture-arm64` の compile-only 境界を再利用し、host trap を含まない fixture を
  `_main` の機械語 body としてリンクする。

方針:

- CLI filesystem / process spawn boundary に閉じる。
- temporary assembly source は CLI が生成し、core crates へ I/O を入れない。
- raw bytes は `.byte` directive として書き出し、assembler / linker に渡す。
- macOS ARM64 host 以外では classified unsupported として扱う。

成功条件:

- `link-fixture-arm64-main <case.json> <out-exe>` が executable artifact を作る。
- 生成 artifact を起動すると `return_42` fixture が exit status 42 になる。
- stdout host trap fixture は executable packaging unsupported として reject される。

状態:

- 完了。`link-fixture-arm64-main <case.json> <out-exe>` で host trap を含まない
  raw testcase を ARM64 `_main` body として `clang` でリンクし、macOS ARM64 上で
  executable artifact を作れる。`return_42` fixture は生成 artifact の実行時
  exit status 42 として観測できる。stdout host trap fixture は standalone artifact
  unsupported として reject される。

HW19 全体の状態:

- 完了。変換済み ARM64 bytes を OS が起動できる macOS ARM64 executable artifact に
  package し、実プロセスとして exit status を観測する smoke 経路ができた。
  standalone `hello world` stdout emission は次の大項目で扱う。

### HW20: Standalone executable stdout emission

目的:

- Bara 専用 stdout host trap fixture を、standalone macOS ARM64 executable artifact の
  実 OS stdout emission として観測できるようにする。
- 生成した executable を起動すると、コンソールに `hello world\n` が出る状態へ進む。

方針:

- x86 `syscall` や libc / dyld import の汎用変換はまだ扱わない。
- 既存 runtime の `RunResult.stdout` 合成を実 OS stdout と見なさない。
- packaging 境界で、明示的な native stdout prologue を生成する。
- process spawn / temporary assembly / toolchain 呼び出しは `btbc-cli` の
  `native_artifact` 境界に閉じる。
- `bara-arm64` は既存どおり ARM64 function body bytes の emit に集中する。

成功条件:

- `hello_world_stdout_return_0` fixture から standalone executable artifact を作れる。
- 生成 artifact を実プロセスとして起動すると stdout が `hello world\n`、
  exit status が 0 になる。
- stdout host trap metadata だけでなく、emitted function が stdout request を持つことを
  確認してから native stdout prologue を生成する。

#### HW20a: Link stdout host trap fixture with native write prologue

目的:

- stdout host trap を要求する fixture を、native `_write` prologue と既存 ARM64
  function body bytes で構成される `_main` としてリンクする。

方針:

- `compile_test_case_function` の compile-only 結果を使う。
- `TestCaseHostTrapPlan` の stdout text と `EmittedHostTrapRequests` の stdout request が
  揃っている場合だけ native stdout prologue を生成する。
- prologue は file descriptor 1、text pointer、text length を `_write` に渡す。
- generated body は既存 emit 結果の bytes をそのまま置き、return value を `_main`
  の exit status として扱う。

成功条件:

- `link-fixture-arm64-stdout-main <case.json> <out-exe>` が
  `hello_world_stdout_return_0` fixture から executable artifact を作る。
- 生成 artifact を起動すると stdout `hello world\n`、exit status 0 が観測できる。
- stdout request がない fixture は stdout executable packaging unsupported として
  classified error になる。

状態:

- 完了。`link-fixture-arm64-stdout-main <case.json> <out-exe>` で
  `hello_world_stdout_return_0` fixture を native `_write` prologue 付きの
  macOS ARM64 executable artifact としてリンクできる。生成 artifact は実プロセスとして
  stdout `hello world\n`、exit status 0 を観測できる。stdout request がない fixture は
  stdout main unsupported として classified error になる。

HW20 全体の状態:

- 完了。Bara 専用 stdout host trap fixture を、runtime の stdout 合成ではなく
  standalone executable artifact 内の明示的な native stdout emission として観測できる。
  生成 executable を起動すると実OS stdoutに `hello world\n` が出る。

## 判断基準

- 先に raw function で外部観測を増やす。
- syscall / libc / loader は、host trap と memory model が安定するまで扱わない。
- flags、stack、call は、hello world に必要になった時点で最小対応する。
- ファイルが肥大化し始めたら、次の命令追加前に責務別に分割する。
