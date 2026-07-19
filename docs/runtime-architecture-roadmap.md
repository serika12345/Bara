# Runtime Architecture Roadmap

この文書は、B8-HWGUI 完遂後に Bara をどの方向へ一般化するかを記録する。
目的は、Rosetta 2 と同等の利用者体験に近い user-space binary translation
runtime を clean-room で実装しつつ、macOS 専用ではない差し替え可能な構造を
保つことである。

## Goal

Bara の実質的な最終目標は、同 OS / 異アーキテクチャ実行を主対象にした
decomposed binary translation runtime である。

代表的な対象は次の形である。

```text
macOS x86_64 app      -> macOS arm64 host
Linux x86_64 app      -> Linux arm64 host
Windows x86_64 app    -> Wine on arm64 host
```

異 OS / 異アーキテクチャを Bara 単体で扱うことは主目標ではない。Windows API、
PE loader、registry、filesystem、windowing などの Windows 互換性は Wine の
責務とし、Bara は x86_64 guest code を host ARM64 上で実行する CPU/runtime
backend として接続する。

Rosetta は引き続き black-box oracle としてのみ扱う。Rosetta の disassembly、
内部 symbol、内部 artifact layout、private ABI、private dyld / kernel integration
を実装根拠にしない。

## Product Model

Bara は「x86_64 binary を arm64 binary file に変換してユーザー visible な
実行ファイルとして保存する tool」を主経路にしない。

主経路は次の runtime model とする。

```text
guest binary
  -> guest loader / image model
  -> decode / lift / validate
  -> target backend emit
  -> translation artifact / cache
  -> dispatcher / executable memory
  -> OS / ABI / helper personality
```

変換済み file export は debug / review / regression 用の補助機能として扱う。
Rosetta 2 の公開文書でも、JIT は process 内で変換し、AOT artifact は system service
が内部 cache として管理する special Mach object であり、通常の変換済み app として
ユーザーが扱うものではない。Bara も同様に、内部 translation artifact と runtime
cache を本流に置き、必要な範囲だけ debug export する。

## Layer Boundaries

### Guest ISA

Guest ISA layer は x86_64 / x86_32 などの decode、register、flags、memory operand、
control-flow instruction を扱う。ここは OS を知らない。

責務:

- instruction bytes から typed decoded instruction を作る
- flags / partial register / memory operand semantics を IR に渡す
- unsupported instruction を classified error として返す

非責務:

- Mach-O / PE / ELF loader
- Objective-C / Win32 / libc API
- executable memory allocation
- process state mutation

### IR And Validation

IR は guest ISA と target ISA から独立した中間表現である。helper call、trap、
fallthrough、return、indirect branch、exception boundary を型で表す。

責務:

- guest observable semantics を target-independent に表す
- verifier が helper boundary、state layout、control-flow shape を検査できるようにする
- logic は外部から見て pure に保つ

### Target Backend

Target backend は IR から host ISA の code bytes と metadata を作る。
最初の主要 backend は ARM64 である。

責務:

- ARM64 code bytes
- pcmap
- fixups
- helper requirements
- debug exportable artifact report

非責務:

- Mach-O / PE / ELF 入力解析
- host OS service 呼び出し
- Wine / AppKit などの API semantics

## Instruction Coverage Strategy

一般アプリ実行へ広げるとき、Bara は x86 opcode を平たい checklist として
1 つずつ実装していく設計にはしない。実装単位は opcode ではなく、decode された
命令を構成する semantic bucket とする。

基本 pipeline は次の形にする。

```text
x86 bytes
  -> decoder
  -> canonical instruction
  -> operand semantics
  -> guest semantic IR
  -> direct ARM64 lowering / helper call / fallback / stable blocker
```

抽象化してまとめて扱う対象:

- prefix、operand width、address width、REX、ModRM、SIB、immediate の decode
- register / memory / immediate operand の read / write
- RIP-relative、register-indirect、base+index*scale+disp の address calculation
- 8 / 16 / 32 / 64 bit 幅、sign extension、zero extension
- `MOV` / load / store family
- `ADD` / `SUB` / `AND` / `OR` / `XOR` / `CMP` / `TEST` などの integer ALU family
- common flags builder と condition-code evaluation
- `PUSH` / `POP` / `CALL` / `RET` などの stack and control-flow family
- helper request、import call、host service、fallback を表す共通 IR boundary

個別に頑張る必要がある対象:

- x86 flags の細かい差分、特に `CF` / `OF` / `AF` / `PF` と lazy materialization policy
- partial register aliasing、特に `AH` など high 8-bit register と 32-bit write zero-extension
- implicit operands を持つ命令、例えば `MUL` / `DIV` / string instructions / `RFLAGS`
- `REP MOVS` / `CMPS` / `SCAS` などの string instruction family
- `LOCK` prefix、atomic read-modify-write、host memory ordering
- SSE / AVX / x87 / MXCSR / floating-point exception and rounding
- `CPUID` / `RDTSC` など environment-dependent instruction
- self-modifying code、JIT-generated guest code、code cache invalidation
- indirect branch、callback、exception、signal、thread、TLS など opcode 外の runtime state

coverage policy:

- decoder は既存の permissive license decoder を採用する余地を残すが、lift / IR /
  runtime semantics は Bara の clean-room domain model として保持する。
- hot path かつ共通性が高い semantic bucket は direct ARM64 lowering へ進める。
- rare または複雑な命令は、最初は helper または interpreter fallback へ逃がしてよい。
  ただし silent fallback は禁止し、fallback の条件、input state、observable result を
  stable report に残す。
- unsupported instruction は opcode だけで分類せず、可能な限り
  `unsupported_semantic_bucket`、operand shape、required runtime service を含む
  blocker にする。
- OSS app cycle では「次の opcode」ではなく「次の semantic bucket」を TODO 化する。

最初に安定させる semantic bucket の順序:

1. operand decode / register alias / width semantics
2. memory addressing and mapped-image read / write boundary
3. integer ALU and flags
4. stack and direct control flow
5. import/helper call and ABI marshaling
6. indirect control flow and dispatcher/cache
7. process state, TLS, thread, signal, exception
8. SIMD / FP / atomics / string instructions

この順序は固定ではない。実 fixture と debug bundle の blocker report が source of truth
であり、一般アプリ化では observed blocker に応じて順序を入れ替える。

## Reference Materials And Permissive Candidates

Bara の ISA semantics は public specification、自作 fixture、observable behavior に基づく。
既存実装を参照する場合は、license が permissive でも内部構造、helper layout、
metadata format、translation strategy をコピーしない。依存として採用する場合は、
先に supply-chain review、license file / notice 確認、最小 regression test を追加する。

authoritative reference:

| 対象 | URL | 用途 |
| --- | --- | --- |
| Intel 64 and IA-32 SDM | <https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html> | x86_64 decode、instruction semantics、flags、exception behavior の primary source |
| Arm A64 ISA docs | <https://developer.arm.com/documentation/ddi0602/latest/> | ARM64 lowering、condition flags、memory ordering の primary source |
| System V AMD64 ABI | <https://gitlab.com/x86-psABIs/x86-64-ABI> | macOS / Linux 系 x86_64 calling convention の比較基準 |
| Apple Mach-O Runtime Architecture | <https://developer.apple.com/library/archive/documentation/DeveloperTools/Conceptual/MachORuntime/> | Mach-O loader / segment / symbol model の public reference |

dependency candidates:

| 候補 | License | 用途 | Bara での扱い |
| --- | --- | --- | --- |
| Intel XED | Apache-2.0, <https://github.com/intelxed/xed/blob/main/LICENSE> | x86 encoder / decoder | decoder 候補。lift / IR semantics は Bara 側に保持する |
| iced-x86 | MIT, <https://github.com/icedland/iced> | Rust x86/x64 decoder / disassembler / assembler | Rust decoder 候補。supply-chain review 後に比較する |
| Zydis | MIT, <https://github.com/zyantific/zydis> | x86/x86-64 decoder / disassembler | C decoder 候補。FFI cost と build integration を要評価 |
| Capstone | BSD, <https://www.capstone-engine.org/> | multi-architecture disassembly | debug / disassembly export 候補。semantic source にはしない |
| object | MIT OR Apache-2.0, <https://github.com/gimli-rs/object> | object file read/write | Mach-O / ELF / PE parsing or debug comparison 候補 |
| goblin | Apache-2.0, <https://github.com/davebshow/goblin> | ELF / Mach-O / PE parsing | lightweight binary parser 候補 |
| LIEF | Apache-2.0, <https://github.com/lief-project/LIEF> | ELF / Mach-O / PE parse / modify | broad binary format tooling 候補。dependency size と C++ binding cost を要評価 |
| AsmJit | Zlib, <https://github.com/asmjit/asmjit> | low-latency machine code generation | JIT backend research / optional native component 候補 |
| Cranelift | Apache-2.0 WITH LLVM-exception, <https://docs.rs/crate/cranelift-codegen/latest/source/Cargo.toml.orig> | compiler backend / codegen infrastructure | helper/fallback backend 候補。exact CPU state lowering との相性を要評価 |

permissive prior-art references:

| 候補 | License | 見るポイント | 制約 |
| --- | --- | --- | --- |
| FEX | MIT, <https://github.com/FEX-Emu/FEX> | user-mode x86/x86_64 on ARM64、Wine/Proton integration、cache/runtime surface | public docs / observable behavior だけを比較対象にし、内部構造はコピーしない |
| Box64 | MIT, <https://github.com/ptitSeb/box64> | Linux x86_64 user-space translation、native library wrapping model | wrapping policy の問題分割を参考にし、実装詳細は使わない |
| DynamoRIO | BSD, <https://dynamorio.org/page_license.html> | dynamic binary instrumentation、code cache、dispatcher、client interface | DBI architecture の比較参照。Bara runtime design へ直接移植しない |
| Remill / McSema | Apache-2.0, <https://github.com/lifting-bits/remill> / <https://github.com/lifting-bits/mcsema> | instruction lifting、whole-program lifting の責務分離 | LLVM bitcode lifter の先行研究として読む。semantics 実装はコピーしない |

non-goals:

- GPL / LGPL / proprietary tool は、資料として public docs を読むことはあっても、
  permissive dependency candidate にはしない。
- QEMU user-mode、Valgrind、Ghidra、Binary Ninja、Rosetta は、license または
  clean-room 境界の理由により Bara core の実装依存候補ではない。
- dependency を入れる前に `nix develop -c ./scripts/verify-supply-chain` を通し、
  依存追加の domain-level reason を同じ change に記録する。

### Translation Artifact And Cache

Translation artifact は、変換済み block と実行に必要な metadata をまとめる内部形式である。
これはユーザー visible な app bundle ではない。

最小構成:

- source identity
- guest address range
- target code bytes
- pcmap
- fixups
- helper requirements
- ABI / state layout
- cache validation identity

cache key は source binary identity、guest virtual address、source bytes hash、
translator version、target backend、OS personality version を含む必要がある。

### Runtime Dispatcher

Runtime dispatcher は translated block を executable memory に配置し、guest PC、
register state、stack state、helper return、indirect branch、fallback を制御する。

段階的に扱う対象:

- direct fallthrough
- direct call / return
- helper call / return writeback
- indirect call / jump
- callback
- exception / signal
- fallback interpreter / JIT

### Guest Loader And Image Model

Loader layer は Mach-O / PE / ELF を、それぞれ public format から guest image model に
変換する。

共通 model:

- entry point
- segments / sections
- virtual address space
- imports / exports
- relocations / fixups
- symbol identity
- initial stack / argv / envp / aux vector equivalent
- code signature or source identity metadata

Mach-O、PE、ELF は同じ抽象 interface に載せるが、format 固有の詳細は各 module に閉じる。

### OS / ABI Personality

OS personality は guest OS と host OS の差し替え境界である。

例:

- macOS x86_64-on-macOS arm64 personality
- Linux x86_64-on-Linux arm64 personality
- Windows x64-on-Wine personality

責務:

- guest ABI calling convention
- import / dynamic library boundary
- syscall / libc / Objective-C / Win32 helper boundary
- TLS / thread / signal / exception policy
- process initial state
- host service adapter

core translator は OS personality を知らない。OS personality は decode / IR / backend の
内部構造を知らず、typed artifact と helper contract を通して接続する。

## Wine Connection

Wine 接続では Bara が Windows API を実装しない。

```text
x86_64 Windows app
  -> Wine PE loader / ntdll / Win32 API model
  -> Bara x86_64 translator/runtime backend
  -> host ARM64 code
  -> Wine thunks / host OS services
```

Bara の責務:

- x86_64 guest instruction execution
- Windows x64 ABI state
- helper/thunk call boundary
- callbacks into guest code
- exception / signal handoff
- translation cache

Wine の責務:

- PE loader policy
- Windows DLL / API behavior
- registry / filesystem / process / windowing semantics
- platform integration

Wine bridge は、Bara runtime core に対する OS personality の 1 つとして実装する。

## Current State After B8-HWGUI

B8-HWGUI では self-authored x86_64 Mach-O GUI Hello World fixture について、
実 `LC_MAIN` entry から GUI lifecycle helper boundary までの chain を stable report
できるようになった。

できること:

- public Mach-O metadata から entry、mapped bytes、symbol/import/fixup identity を得る
- focused x86_64 instruction subset を decode / lift / emit または classified boundary にする
- Objective-C / AppKit helper boundary を public API helper process で観測する
- automated expected / actual comparison を `{"issues":[]}` にする
- manual visible mode で Hello World window を確認する
- debug bundle に blocker、loader plan、launch report を保存する

まだできないこと:

- input Mach-O 全体を arm64 Mach-O / `.app` として出力する
- arbitrary app を汎用 loader/runtime で実行する
- general continuation execution
- arbitrary indirect call / arbitrary Objective-C message send
- translation cache / dispatcher
- fallback interpreter / JIT
- process-wide state、thread、TLS、signal、exception
- Wine bridge

## Roadmap

### R0: Post-HWGUI Architecture Record

B8-HWGUI 完遂後の議論、抽象化対象、主経路を documentation と TODO に固定する。

完了条件:

- この文書が追加されている
- `TODO.md` が B8-HWGUI 後の抽象化 milestone を持つ
- `docs/design-todo.md` が architecture direction を記録している
- `docs/progress.md` が現在の次 action を review / merge 後の architecture work として示す

### R1: Responsibility Split Audit

`btbc-cli` と `b8_debug_bundle.rs` に集まった B8-specific logic を棚卸しし、
loader、runtime、helper、report、fixture の責務へ分類する。実装変更は最小限にし、
まず module boundary と extraction order を決める。

完了条件:

- B8-specific logic の分類表が design TODO にある
- 抽出順が TODO-backed PR Gate として定義されている
- behavior は変えず、既存 verification が通る

2026-06-14 の audit result:

- `crates/btbc-cli/src/b8_debug_bundle.rs` は debug bundle file I/O、real-entry attempt、
  stable report DTO、loader/import projection、modeled continuation state、public
  Objective-C/AppKit helper process execution を同じ file に持つ。
- `crates/btbc-cli/src/main.rs` は command dispatch、command implementation、
  fixture/oracle path construction、CLI behavior tests を同じ file に持つ。
- `crates/bara-oracle/src/binary_format/` は public Mach-O parser / resolver として
  clean-room 境界内にあるが、B8-HWGUI 後は runtime-facing `GuestImage` / `MachOImage`
  model の入力になっているため、oracle-specific expected generation と loader model を
  分ける必要がある。
- 最初の behavior-preserving code split は B8 debug bundle report DTO module split にする。
  その後、bundle I/O、real-entry attempt orchestration、GuestImage model、runtime
  dispatcher、helper/ABI bridge の順で抽出する。

2026-06-14 の B8-ARCH2a split result:

- `btbc-cli/src/b8_debug_bundle/report.rs` に entry / decode / artifact / launch /
  runtime attempt / blocker の stable report DTO と stage / source / memory-width schema
  enum を分離した。
- JSON schema 名、serde tag / rename、field 名は維持し、loader/import projection、
  bundle file I/O、helper process execution、modeled continuation state は後続 R2 / R4 /
  R5 境界に残した。

2026-06-14 の B8-ARCH2b split result:

- `btbc-cli/src/b8_debug_bundle/io.rs` に bundle directory layout、output path JSON、
  JSON/bin/text file read/write helper、repro script generation を分離した。
- output path JSON field、bundle file 名、repro script command string は維持し、
  real-entry attempt orchestration、loader/import projection、helper process execution、
  modeled continuation state は後続 R1 / R2 / R4 / R5 境界に残した。

2026-06-14 の B8-ARCH2c split result:

- `btbc-cli/src/b8_debug_bundle/attempt.rs` に real-entry decode/lift/emit/runtime
  attempt orchestration と unsupported terminator frontier helper を分離した。
- JSON output、blocker classification、runtime attempt behavior は維持し、
  loader/import projection、helper process execution、modeled continuation state は後続 R2 /
  R4 / R5 境界に残した。

2026-06-14 の B8-ARCH2d split result:

- `btbc-cli/src/b8_debug_bundle/loader.rs` に loader plan shell DTO を分離した。
- `loader.plan.json` schema 名、field 名、helper boundary request の launch report 接続は
  維持し、import/fixup projection、helper process execution、modeled continuation state、
  `GuestImage` / `MachOImage` 本体抽出は後続 R2 / R4 / R5 境界に残した。

2026-06-14 の B8-ARCH2e split result:

- `btbc-cli/src/b8_debug_bundle/import_boundary.rs` に import boundary projection と
  public import metadata report DTO を分離した。
- `loader.plan.json` import boundary field 名、JSON output、helper boundary request の
  launch report 接続は維持し、helper request / marshaling、helper process execution、
  modeled continuation state、`GuestImage` / `MachOImage` 本体抽出は後続 R2 / R4 / R5
  境界に残した。

2026-06-14 の B8-ARCH2f split result:

- `btbc-cli/src/b8_debug_bundle/helper_boundary.rs` に helper boundary request と import
  helper marshaling contract shell を分離した。
- `loader.plan.json` と launch report の helper boundary request field 名、schema 名、
  JSON output は維持し、helper process execution、modeled continuation state、
  `GuestImage` / `MachOImage` 本体抽出は後続 R2 / R4 / R5 境界に残した。

2026-06-14 の B8-ARCH2g split result:

- `btbc-cli/src/b8_debug_bundle/guest_image.rs` に loader plan の image mapping summary
  shell を分離した。
- `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output は
  維持し、import/fixup projection、helper boundary、modeled continuation state、
  `GuestImage` / `MachOImage` 本体抽出は後続 R2 / R4 / R5 境界に残した。

2026-06-20 の B8-ARCH2h split result:

- `bara-runtime/src/guest_image/mod.rs` に parser 非依存の runtime-facing `GuestImage`
  shell を追加し、entry point、code segment range、segment source、address space、
  mapped bytes source を domain object として保持する入口を作った。
- B8 debug bundle の `image_mapping` report は `MachOEntryFunctionInput` から
  `GuestImage::mach_o_executable` へ射影してから existing JSON DTO を作る。
- `MachOImage` 本体、imports/fixups/symbol identity、`bara-oracle` からの loader
  domain 抽出は後続 R2 に残した。

### R1a: ISA Semantic Coverage Plan

一般アプリ化で必要になる x86_64 coverage を opcode list ではなく semantic bucket として
分類する。B8-HWGUI で実装した命令群を材料に、decode-only、lift-ready、direct lowering、
helper-required、fallback-required、stable blocker の状態を分ける。

完了条件:

- x86_64 instruction coverage が semantic bucket 単位で design TODO に分類されている
- canonical instruction、operand semantics、guest semantic IR、backend lowering、
  helper/fallback の責務分担が文書化されている
- OSS app cycle で次に潰す対象を opcode ではなく semantic bucket として選べる
- clean-room source と permissive dependency 候補の扱いが明記されている

2026-06-14 の B8-ARCH1a audit result:

- B8-HWGUI で追加した focused instruction slices は
  `prefix / width / register alias decode`、`register transfer`、
  `RIP-relative data access`、`RIP-relative address materialization`、
  `register-indirect data access`、`stack frame and epilogue`、
  `direct control flow`、`indirect / imported control flow`、`integer ALU and flags`、
  `byte load / zero extension`、`syscall / host service request`、
  `Objective-C / AppKit helper service`、`fallback / runtime state` に再分類した。
- coverage status vocabulary は `decode_only`、`lift_ready`、`direct_lowering_ready`、
  `helper_required`、`fallback_required`、`stable_blocker` とする。
- direct lowering は pure IR と target backend metadata だけで表せる bucket に限定する。
  loader/import/ABI/OS personality が必要な bucket は helper、dispatcher state が必要な
  bucket は fallback-required または stable blocker として扱う。
- unsupported report は、opcode だけでなく semantic bucket、operand shape、
  source PC range、raw bytes、required runtime service、fallback possibility、
  clean-room source、next action を持てる方向へ進める。
- dependency candidate は decoder / disassembler の canonical instruction construction
  までに留める。lift / IR semantics / runtime helper / metadata schema / fallback policy は
  Bara の clean-room domain model として保持する。採用は別 PR Gate とし、
  license / NOTICE / transitive dependency / Nix packaging / supply-chain verification を必須にする。

### R2 / B8-TYPE1: Typed Runtime Execution Foundation

型と責務境界の強化を、アプリ起動機能とは独立した中マイルストーンとして完了させる。
Guest image だけでなく、translation artifact、runtime state、macOS host service request までを
typed boundary で接続する。ただし、それらを使った relocation 適用、dispatcher execution、
host service execution は後続 R3 以降に置く。

完了条件:

- entry point、segments、mapped bytes、executable code bytes、imports、fixups、symbol identity、
  unwind が domain type で表現される
- `TranslationArtifact` と dispatcher state を pure に構築・検証でき、cache identity は
  source hash / translator version / target の最小値に限る
- `GuestCall -> MacOsHostServiceRequest -> GuestReturn` の具体的な最小 contract が型で定義される
- loader / dispatcher / helper の failure と unsupported state が typed error / blocker になる
- production の B8 debug bundle GuestImage path が snapshot 境界を使い、primitive / nested DTO
  依存を増やさない。artifact / state / service contract の実経路接続は R3 以降に残す
- `bara-oracle` の external observation 責務と loader domain construction の依存方向が分離される
- cross-platform personality selection、Wine thunk abstraction、PE / ELF interface を先行実装せず、
  public primitive baseline を増やさない

既存 Guest Image Model Extraction の進捗:

- B8-ARCH2h で runtime-facing `GuestImage` shell を追加した。現時点では entry point、
  code segment range、segment source、address space、mapped bytes source だけを扱い、
  `MachOImage` 本体、imports/fixups/symbol identity は未抽出である。
- B8-ARCH2i で `GuestImage` が `ProgramImageMappedBytes` を保持するようにした。
  mapped bytes はまだ `bara-oracle` の public Mach-O materialization 由来であり、
  imports/fixups/symbol identity と `MachOImage` 本体は未抽出である。
- B8-ARCH2j で `GuestImage` が `ProgramImageImports` を保持するようにした。
  imports collection はまだ `ProgramImageMetadata` 由来であり、fixups/symbol identity と
  `MachOImage` 本体は未抽出である。
- B8-ARCH2k で `GuestImage` が `ProgramImageRelocations` を保持するようにした。
  relocations collection はまだ `ProgramImageMetadata` 由来であり、symbol identity と
  `MachOImage` 本体は未抽出である。
- B8-ARCH2l で `GuestImageMetadata` aggregate を追加し、`GuestImage` が
  `ProgramImageMetadata` 由来の sections / mapped bytes / symbols / relocations / imports /
  unwind を aggregate 経由で保持するようにした。`MachOImage` 本体は未抽出である。
- B8-ARCH2m で `MachOImage` shell を追加し、Mach-O specific image model から valid
  `GuestImage` / `GuestImageMetadata` を read-only に参照できるようにした。
- B8-ARCH2n で `MachOImage::executable_from_code_range` を追加し、Mach-O executable
  code segment の source / address-space 決定を runtime constructor 側に閉じた。
- B8-ARCH2o で `MachOImage::executable_from_program_image_metadata` を追加し、
  `ProgramImageMetadata` からの `GuestImageMetadata` assembly と mapped bytes source 選択を
  runtime constructor 側に閉じた。
- B8-ARCH2p で `MachOExecutableCodeRange` を追加し、`MachOImage` constructor が
  汎用 `ProgramImageRange` ではなく Mach-O specific code range domain type を受け取るようにした。
- B8-ARCH2q で `MachOExecutableCodeRange::from_program_image_metadata` を追加し、
  `ProgramImageMetadata.sections()` の single code section から executable code range を
  選ぶ判断を runtime constructor 側に閉じた。
- B8-ARCH2r で `MachOExecutableEntryPoint` を追加し、`MachOImage` constructor が
  generic `GuestImageEntryPoint` ではなく Mach-O specific entry point domain type を
  受け取るようにした。
- B8-ARCH2s から B8-ARCH2ag で Mach-O code segment、metadata value object 群、
  module split、debug mapping projection、mapping snapshot を段階的に runtime 側へ寄せた。
  B8-ARCH2ah では `GuestImageMetadata` が sections / symbols / relocations / imports /
  unwind を value object として返せるようにし、後続 runtime loader caller が payload
  primitive ではなく runtime-facing value object 境界で metadata collection を扱えるようにした。
- B8-ARCH2ai で `MachOExecutableImageMetadata` を追加し、Mach-O specific executable
  image snapshot から mapped bytes / sections / symbols / relocations / imports / unwind を
  value object として扱えるようにした。
- B8-ARCH2aj で `MachOExecutableImageSnapshot` を追加し、Mach-O specific executable
  image snapshot から mapping snapshot と metadata snapshot を同じ boundary で扱えるようにした。

- B8-ARCH2ak〜B8-ARCH2an で、loader plan が一度作った executable image snapshot を
  image mapping、import、helper projection の共通入口にし、metadata compatibility view の
  assembly と snapshot-level access を runtime domain 側へ寄せた。

B8-TYPE1 completion result:

- executable snapshot から `MachOExecutableCodeBytes` と typed source range を取得できるようにし、
  mapped byte payload を bare byte buffer として新規公開しない。
- `bara-arm64::TranslationArtifact` は emitted ARM64 function と source / minimal cache identity を
  pure にまとめ、cache target を concrete `Arm64MacOs` として区別する。
- `bara-runtime` は guest PC、register、stack、helper suspend / return phase の最小 state、concrete
  macOS host service request / return contract、loader / dispatcher / helper blocker を pure domain
  model として持つ。
- production B8 debug bundle は existing Mach-O snapshot path を維持する。新しい artifact / state /
  service contract は focused construction / validation までで停止し、execution path へ接続しない。
- runtime normal dependency graph は `bara-oracle` を含まない。external Mach-O observation は CLI
  adapter で runtime-owned constructor へ渡し、loader domain construction の依存方向を runtime 側に
  保つ。
- public primitive baseline、dependency、lockfile、JSON schema は増やさず、OS personality selection、
  Wine thunk、PE / ELF interface、dispatcher / host execution を実装しない。

停止条件:

- production GuestImage consumer が snapshot 境界を通り、その他の新しい型が focused
  construction / validation test を持った時点で完了する。artifact execution は R3 に残す。
- cache / JIT / fallback、PE / ELF implementation、一般アプリ実行を型マイルストーンへ含めない。
- 完了後の型追加は、R3 以降の concrete blocker を解消する範囲に限定する。

### R3 / B8-LAUNCH1: Translation Artifact Execution Path

typed `TranslationArtifact` を compile、debug export、runtime input の実経路へ接続する。

完了条件:

- ARM64 bytes、PC map、fixups、helper requirements、source/cache identity が artifact 経由で渡る
- existing fixture expected / actual が artifact execution path でも通る
- CLI report DTO を runtime input として使わない

### R4 / B8-LAUNCH2: Executable Image Preparation

public Mach-O metadata から実行直前の mapped image と initial entry state を準備する。

完了条件:

- segment mapping、relocation / rebase / bind、import resolution を fixture 必要分だけ適用する
- W^X、ownership、unresolved import が typed loader result になる
- sentinel-free `LC_MAIN` entry execution の直前状態を debug bundle に保存する

R4 は self-authored fixture の entry block を dispatcher へ渡せる最小 mapped image で停止する。
R5 / R6 で新しい loader blocker が観測された場合は、R4 の focused PR Gate へ戻って解消する。

### R5 / B8-LAUNCH3: Runtime Dispatcher Core

translation artifact と typed runtime state を使って guest control flow を継続実行する。

完了条件:

- entry、fallthrough、direct call、return、helper suspend / return writeback が動く
- indirect target は resolved target または stable blocker になる
- cache / interpreter / JIT は交換可能な future strategy として保つ

### R6 / B8-LAUNCH4: macOS ABI And Service Bridge

x86_64 macOS SysV と public Objective-C / AppKit / libSystem service を generic host service
contract 経由で実行する。

完了条件:

- argument materialization と return writeback が reusable contract になる
- Objective-C / libSystem service が同じ boundary model に載る
- B8-HWGUI 専用 helper path を generic contract 上の fixture case にできる

### R7 / B8-LAUNCH5: Process Environment

initial stack、argv / envp、process metadata を target に必要な範囲で追加する。TLS、thread、
signal、exception、file descriptor、`.app` resource は blocker になった順に扱う。

### R8 / B8-LAUNCH6: Sentinel-Free Self-Authored GUI

self-authored x86_64 Mach-O GUI executable を B8-G1 sentinel なしで `LC_MAIN` から起動し、
loader、dispatcher、service bridge を通って window と label を表示する。

### R9 / B8-LAUNCH7: Source-Built OSS GUI Expansion

license、build inputs、success criteria を固定した OSS GUI app を reproducible build し、first
unsupported boundary を ISA / loader / helper / dispatcher の focused PR Gate へ分解する。

### R10: B10 PE / Wine Planning

R9 / B8-LAUNCH7 の review gate 後に、macOS で実際に検証できた loader / dispatcher / ABI /
service boundary を根拠として PE / Wine / Bara の責務を計画する。それまでは callback、TLS、
thread、exception、thunk や personality selection を先行抽象化しない。
