# 詳細設計 TODO / 設計メモ

この文書は、実装大項目とは分けて、設計上の判断、分割方針、
肥大化を防ぐための監査観点を残す場所とする。

実装 TODO は [TODO.md](../TODO.md) の B1-B10 に置き、ここには
「どの境界をどう切るべきか」「いつ設計を固定しすぎないようにするか」
を記録する。
本流から外した未確立な派生研究は
[将来構想メモ](future-research-concepts.md) に分離する。

## D1: CLI と command 境界

- [ ] `btbc-cli/src/main.rs` から command dispatch、command implementation、file I/O、test を分割する。
- [ ] CLI は domain logic を持たず、typed input を作って application service に渡す境界にする。
- [ ] native artifact、blackbox run、binary probe、Mach-O run をそれぞれ責務別 module に整理する。
- [ ] CLI test は command behavior test と domain conversion test を混ぜない。

メモ:

- 現在の `btbc-cli/src/main.rs` は肥大化しており、B1/B2 の前に優先して分割する。
- CLI は今後 AOT / JIT / loader / oracle / artifact packaging を束ねるため、早めに薄くしておく。

## D2: Artifact domain model

- [ ] raw ARM64 code、assembly source、object file、linked executable、execution report を別の domain type として扱う。
- [x] artifact metadata は実行結果とは分け、生成条件、target triple、toolchain、helper requirements を含める。
- [x] 外部 toolchain 経路と pure writer 経路を同じ interface から選べるようにする。
- [x] host unsupported、toolchain missing、link failure、execution failure を分類する error/report model を設計する。

メモ:

- Hello World では `clang` packaging で十分だが、将来の Mach-O writer や
  ELF/PE packaging を考えると artifact model を先に固める。
- artifact は「ファイル」ではなく「生成物とその説明」として扱う。

## D3: Source ISA mode と x86 bit-width

- [x] source ISA mode を `x86_64` / `x86_32` として明示できる domain type を追加する。
- [x] address size、operand size、stack width を source mode から決める。
- [ ] calling convention を source mode から決める。
- [x] register model は 64-bit register だけでなく、partial register を表現できる形にする。
- [ ] decoder / lifter / metadata schema の public 名称を `x86_64` 固定にしすぎない。

メモ:

- 現状の IR と lifter は x86_64 最小 subset として問題ない。
- B8 の実 x86_64 macOS アプリ起動では x86_64 を対象にするが、source
  mode を型として入れ、B9 の x86 32-bit アプリ対応を public API から
  閉じ出さない。
- B9 は B10 の PE / Wine 接続前に先に処理するのが望ましいが、blocker が
  大きい場合は記録したうえで飛ばしてよい推奨ステップとする。
- 2026-06-11 の B8 小ステップとして、`bara-runtime::UserSpaceLaunchPlan` に
  `source_isa_profile` を追加した。現在は x86_64 long mode、address size
  64-bit、default operand size 32-bit、stack width 64-bit を typed profile
  として保持し、B8 actual launch report に projection する。profile model は
  x86_32 protected mode、address size 32-bit、default operand size 32-bit、
  stack width 32-bit も表現できる。これは B9 の x86_32 decode / lift 実装ではなく、
  launch/report 境界から x86_32 を閉じ出さないための guardrail である。
- 2026-06-11 の B8 小ステップとして、`bara-ir::X86Reg` に accumulator
  family の `rax` / `eax` / `ax` / `al` と destination-index family の
  `rdi` / `edi` / `di` / `dil` を追加し、`family`、`width`、`full_width`、
  `is_partial_view` で register view を判定できるようにした。これは既存の
  `eax` 命令 lift を即座に partial-register semantics へ変えるものではなく、
  B9 と後続 decode / lift 拡張で partial register を public IR から表現できる
  guardrail である。

## D4: Bara IR の責務

- [ ] Bara IR は binary translation 固有の semantic IR として維持する。
- [ ] CFG、terminator、flags、stack、call、memory access、helper request を段階的に表現する。
- [ ] backend や副出力で失われやすい情報は metadata または helper boundary として保持する。
- [ ] IR validation は I/O を持たない pure report として返す。

メモ:

- 未確立な副出力研究は本流 TODO ではなく、
  [将来構想メモ](future-research-concepts.md) の構想として扱う。
- 2026-06-11 の B7 判断として、Haskell verifier はまだ導入しない。
  まず Rust 側で IR invariant、PC map invariant、fixup consistency、
  final state comparator を stable report として整える。Haskell は
  `spec/` 配下の独立仕様モデルと property/shrink が必要になり、schema と
  Nix toolchain 追加の必要性がテストで示された時点で導入する。

## D5: Host helper / OS boundary

- [ ] stdout、file I/O、time、memory allocation、process exit を capability として分ける。
- [ ] Bara host helper ABI が syscall / OS API request と runtime helper を区別できる最小 interface を設計する。
- [ ] helper request は core IR / emit に OS 固有処理を混ぜず、runtime boundary で解決する。
- [ ] unsupported helper / OS API request を stable blocker classification として返す。

メモ:

- `hello world` の stdout helper は初期成功経路として妥当。
- `write_stdout(ptr_len_to_unit)` は `HostHelperRequest` / `HostHelperAbi`
  として IR に保持し、`RuntimeHelper` とは分ける。これにより stdout
  effect を syscall / libc / OS API の直接実装として扱わず、manifest
  解決と runtime 境界で扱う capability に留める。
- native stdout emission は output artifact packaging 境界の責務とする。
  現在の macOS ARM64 `_write` prologue は packaging strategy であり、
  decode / lift / IR / ARM64 emit へ OS 固有処理を混ぜない。
- stdout helper emission は target OS ABI ごとの strategy で選ぶ。現状は
  `arm64-apple-macos` の `_write` strategy だけを実装し、Linux / Windows
  は明示的な unsupported emission target として分類する。
- libc / dyld / import call は `ExternalSymbolImport` の public symbol
  identity として保持する。`puts` / `write` / `dyld_stub_binder` は
  import identity であり、libc ABI や dyld loader behavior を直接模倣しない。
- function-level の unsupported syscall / external call は
  `btbc-cli` の report I/O 境界で `unsupported_boundary` JSON message
  として分類する。これは停止理由の安定化であり、syscall 実行、
  libc 呼び出し、dyld import 解決を意味しない。
- 今後は B8 の x86_64 macOS アプリ起動、B9 の x86 32-bit アプリ対応、
  B10 の Wine bridge が同じ helper boundary を使えるようにする。
- B8 の最初の GUI Hello World は AppKit-based single-binary fixture とするが、
  AppKit や Objective-C runtime の内部構造を core IR / emit へ混ぜない。
  public import identity、helper capability、または unsupported boundary として
  runtime 境界で扱う。
- 2026-06-11 の B8 小ステップとして、`UserSpaceHelperBoundaryPlan` に
  public AppKit framework import、import resolution、Objective-C runtime、
  OS API request、次 blocker、status を追加した。B8 feedback report は
  `helper_boundary_plan` を保存し、次 action を
  `connect_appkit_import_objc_runtime_helper_boundary`、次 blocker を
  `unsupported_import` とする。これは AppKit / Objective-C runtime の内部構造や
  bridge 実行を実装するものではなく、public import identity を helper capability
  required と explicit blocker へ落とす境界固定である。
- 2026-06-11 の B8 小ステップとして、`helper_boundary_plan.next_blocker` の
  `unsupported_import` を B8 actual result と current blocker に接続した。
  これにより loader blocker は次の実装対象から外れ、current blocker は public
  AppKit import boundary になる。これは AppKit import 解決や Objective-C runtime
  bridge の実行ではなく、helper boundary の explicit blocker promotion である。
- 2026-06-11 の B8 小ステップとして、`helper_boundary_plan.next_blocker` を
  `unsupported_objc_runtime_boundary` に進め、B8 actual result と current blocker に
  接続した。これにより public AppKit import blocker は次の実装対象から外れ、
  current blocker は Objective-C runtime helper boundary になる。これは
  Objective-C runtime bridge の実行ではなく、helper boundary の explicit blocker
  promotion である。
- 2026-06-11 の B8 小ステップとして、Objective-C runtime / AppKit lifecycle
  helper capability contract を `bara-runtime::UserSpaceHelperCapabilityPlan` と
  B8 actual / feedback report に追加した。contract は self-authored fixture の
  deterministic GUI lifecycle event を stdout observation として扱うための
  helper boundary model であり、Objective-C runtime や AppKit の内部構造を
  core IR / emit へ混ぜない。実 host execution、GUI helper process、current blocker
  解除は次 step に残す。
- 2026-06-11 の B8-H1 helper execution slice として、Bara actual path が x86_64 Mach-O GUI
  fixture を入力として probe したうえで、self-authored AppKit source を host AppKit
  helper capability として build/run し、deterministic lifecycle event を
  actual observation にする経路を追加した。これは public AppKit API と自作 fixture
  に基づく helper boundary execution であり、Objective-C runtime / AppKit の内部再実装、
  private dyld behavior、任意 GUI app の full process translation ではない。
  current blocker は `none`、feedback report は `matched` となり、B8-H1 は review gate に
  到達した。
- 2026-06-11 の milestone 再定義として、上記 helper execution を B8 全体の完了ではなく
  B8-H1 reviewable slice として扱う。次の B8-G1 は、x86_64 entry path が Bara の
  decode / lift / emit / runtime execution を通ったうえで AppKit lifecycle helper
  capability を呼び、GUI window 上に `hello world` label のフォント描画を
  developer-visible mode で確認できることを目標にする。host AppKit helper は
  public AppKit boundary として残してよいが、helper 単独実行だけを
  変換レイヤー通過とは扱わない。
- 2026-06-11 の B8-G1 first step として、automated oracle 用の短時間終了 GUI
  fixture と、Rosetta 手動可視確認用の manual-visible x86_64 GUI fixture を
  同じ self-authored AppKit source から build できるように分けた。
  manual-visible binary は public AppKit API で window と `hello world` label を描画し、
  auto-close timer を無効化して window close / `Command-Q` まで event loop を維持する。
  これは変換レイヤー接続前の test binary 固定であり、次 step では translated
  x86_64 entry path から AppKit lifecycle helper capability を呼ぶ境界を設計する。
- 2026-06-11 の B8-G1 completion step として、Bara-defined
  `appkit_gui_hello_world` host trap contract を追加した。専用 x86_64 entry
  `0f0b4238473131c0c3` は decode / lift / emit / runtime execution を通り、
  emitted host trap request として AppKit lifecycle helper capability を要求する。
  CLI は Rosetta 確認済み x86_64 GUI binary を public Mach-O probe したうえで、
  translated entry request によって automated helper または manual-visible helper を
  起動し、launch report に translated path、helper request、runtime result、
  helper capability invocation を保存する。これは B8-G1 専用 host trap 経由の
  最小 GUI launch であり、Objective-C runtime / AppKit call translation や
  任意 GUI app 実行を意味しない。
- B8-G2 以降は、B8-G1 専用 host trap を肥大化させるのではなく、実 Mach-O entry
  から進んだ結果として helper boundary が必要になる箇所を特定し、public import
  identity、call site、argument / return marshaling、helper capability request を
  分けて model 化する。Objective-C runtime / AppKit helper bridge は
  lifecycle event 専用から class lookup、selector lookup、message send、
  autorelease pool、run loop lifecycle へ広げるが、core IR / ARM64 emit に
  AppKit 固有処理を混ぜない。
- B8-D0 として、一般アプリ化の前に debug bundle foundation を置く。debug bundle は
  input probe、entry extraction、decode report、lift IR、emit report、PC map、
  fixups、helper requests、loader plan、runtime attempt、blocker、repro command を
  1 directory に集める failure analysis 用 sidecar とする。これは通常の
  actual / launch / feedback report を置き換えず、次の unsupported boundary を
  修正するための作業材料を保存する。core decode / lift / emit / validation は I/O を
  持たず、debug 情報は戻り値の report value または明示 collector から CLI が保存する。
- 2026-06-11 の B8-D0 completion step として、`generate-b8-debug-bundle` CLI を
  追加した。debug bundle は input Mach-O probe と、B8-G1 の translated host trap
  entry に対する entry bytes、decode report、lift IR、emit report、PC map、fixups、
  helper request、runtime attempt、loader plan、blocker、repro command を
  `target/b8-debug/<case_id>/` 相当の directory に保存する。これは sidecar
  foundation であり、実 `LC_MAIN` first-block translation attempt は B8-G2 に残す。
- 2026-06-11 の B8-G2 completion step として、debug bundle の entry source を
  B8-G1 専用 host-trap sentinel から public `LC_MAIN` `entryoff` 由来の実 entry
  bytes に切り替えた。bundle は decode / lift / emit / runtime attempt を段階別に
  `available` / `failed` / `skipped` として保存し、`launch.report.json` には
  `public_lc_main_entryoff`、処理した source PC range、B8-G1 host-trap path
  `not_used`、同じ blocker report を保存する。現在の source of truth は
  `blocker.json` の `unsupported_instruction` /
  `DecodeUnsupportedOpcode { opcode: 85 }` であり、B8-G3 は x86_64 `push rbp`
  prologue slice から進める。
- 2026-06-11 の B8-G3 completion step として、x86_64 `push rbp` (`0x55`) だけを
  first ISA blocker slice として追加した。IR register model は base pointer family
  を持ち、decode は `PushRbp`、lift は `IrOp::Push { src: Rbp }`、ARM64 emit は
  `str x29, [sp, #-16]!` として扱う。これは prologue 全体や一般 stack frame
  lowering の実装ではなく、debug bundle が最初の blocker を越えるための最小 slice
  である。現在の source of truth は `blocker.json` の
  `DecodeUnsupportedOpcode { opcode: 72 }` (`48 89 e5`, `mov rbp,rsp`) であり、次の
  PR Gate はこの REX.W register move を扱う。
- 2026-06-11 の B8-G3b completion step として、`48 89 e5` (`mov rbp,rsp`) だけを
  prologue slice として追加した。IR register model は stack pointer family を持ち、
  decode は `MovRbpRsp`、lift は `IrOp::Mov { dst: Rbp, src: Rsp }`、ARM64 emit は
  `mov x29, sp` として扱う。これは一般 register move や stack frame lowering の
  実装ではない。現在の source of truth は `blocker.json` の
  `DecodeUnsupportedOpcode { opcode: 65 }` (`41 57`, `push r15`) であり、次の
  PR Gate はこの REX.B extended-register push を扱う。
- 2026-06-11 の B8-G3c completion step として、`41 57` (`push r15`) だけを
  prologue slice として追加した。IR register model は R15 family を持ち、
  decode は `PushR15`、lift は `IrOp::Push { src: R15 }`、ARM64 emit は
  `str x15, [sp, #-16]!` として扱う。これは extended register push の一般化や
  prologue 全体の lowering ではない。現在の source of truth は `blocker.json` の
  `DecodeUnsupportedOpcode { opcode: 65 }` (`41 56`, `push r14`) であり、次の
  PR Gate は R14 の REX.B extended-register push を扱う。
- 2026-06-11 の B8-G3d completion step として、`41 56` (`push r14`) だけを
  prologue slice として追加した。IR register model は R14 family を持ち、
  decode は `PushR14`、lift は `IrOp::Push { src: R14 }`、ARM64 emit は
  `str x14, [sp, #-16]!` として扱う。これは REX.B push 全体や callee-saved
  prologue 全体の一般実装ではない。現在の source of truth は `blocker.json` の
  `DecodeUnsupportedOpcode { opcode: 83 }` (`53`, `push rbx`) であり、次の
  PR Gate は RBX push を扱う。
- 2026-06-11 の B8-G3e opcode-only batch として、`53` (`push rbx`) と
  `48 89 c3` (`mov rbx,rax`) を追加した。RBX は IR register family として追加し、
  ARM64 emit では `x19` に対応させる。batch は debug bundle の次 blocker が
  `48 8b 05 disp32` の RIP-relative memory load になった時点で停止する。これは
  単なる opcode 追加を超えて image-relative address calculation、read width、
  mapped bytes / loader metadata 境界を要求するため、次の focused PR Gate として扱う。
- 2026-06-11 の B8-G3f として、`48 8b 05 disp32`
  (`mov rax, qword ptr [rip+disp32]`) を RIP-relative 64-bit load slice として追加した。
  IR は source address と read width を `MemRipRelative` として保持し、Mach-O entry
  pipeline は materialized executable segment bytes を `ProgramImageMetadata` の mapped
  bytes として渡す。ARM64 emit はこの slice では rebase / bind 適用後の runtime memory
  ではなく、public Mach-O から得た mapped bytes の qword を AOT immediate として
  materialize する。次 blocker は `48 8b 10`
  (`mov rdx, qword ptr [rax]`) であり、register-indirect memory、`rdx` register、
  loader / mapped runtime memory boundary を含むため opcode-only batch では扱わない。
- 2026-06-11 の B8-G3g として、`48 8b 10`
  (`mov rdx, qword ptr [rax]`) を register-indirect 64-bit load boundary として追加した。
  IR は `rdx` register family と `MemRegIndirect { base: Rax, width: Bits64 }` を持つ。
  ARM64 emit はこの slice では RAX が静的に既知で、かつその address が
  `ProgramImageMetadata` の mapped bytes にある場合だけ `x2` immediate として
  materialize する。RAX が runtime value の場合や mapped bytes から読めない場合は、
  loader / mapped runtime memory の不足として typed unsupported reason を返す。次 blocker
  は `48 8d 3d b3 10 00 00` (`lea rdi, [rip+disp32]`) であり、memory read ではない
  RIP-relative effective address materialization として次の focused PR Gate で扱う。
- 2026-06-11 の B8-G3h として、`48 8d 3d disp32`
  (`lea rdi, [rip+disp32]`) を RIP-relative address materialization slice として追加した。
  IR は memory read と区別するため、source address を `AddressRipRelative` operand として
  保持する。ARM64 emit は現状の ABI-focused register mapping に合わせて `rdi` を `x0` に
  materialize し、その後は `rax` value を available とみなさない。次 blocker は
  `48 8d 35 b6 10 00 00` (`lea rsi, [rip+disp32]`) であり、`rsi` register family と
  2 番目の argument register materialization を次の focused PR Gate で扱う。
- 2026-06-11 の B8-G3i として、`48 8d 35 disp32`
  (`lea rsi, [rip+disp32]`) を RIP-relative address materialization slice として追加した。
  IR は B8-G3h と同じ `AddressRipRelative` operand を使い、destination は新規
  source-index register family の `rsi` として保持する。ARM64 emit は ABI-focused mapping
  に合わせて `rsi` を `x1` に materialize し、`rax` value availability は維持する。
  次 blocker は `48 8b 3d 22 3b 00 00` (`mov rdi, qword ptr [rip+disp32]`) であり、
  address materialization ではなく RIP-relative 64-bit memory load into argument register
  として次の focused PR Gate で扱う。
- 2026-06-11 の B8-G3j として、`48 8b 3d disp32`
  (`mov rdi, qword ptr [rip+disp32]`) を RIP-relative 64-bit memory load slice として追加した。
  IR は B8-G3f と同じ `MemRipRelative { width: Bits64 }` operand を使い、destination を
  `rdi` として保持する。ARM64 emit は現在の mapped bytes から qword を読み、ABI-focused
  mapping に合わせて `x0` に materialize する。ただし destination は `rdi` なので、
  `rax` value availability は無効化する。次 blocker は
  `48 8b 35 eb 3a 00 00` (`mov rsi, qword ptr [rip+disp32]`) であり、同じ memory load を
  2 番目の argument register destination として次の focused PR Gate で扱う。
- 2026-06-11 の B8-G3k として、連続する RIP-relative MOV load batch を次の non-load
  blocker まで進めた。`48 8b 35 disp32` (`mov rsi, qword ptr [rip+disp32]`) は
  `rsi` / `x1` へ、続く `4c 8b 35 disp32` (`mov r14, qword ptr [rip+disp32]`) は
  `r14` / `x14` へ mapped qword を materialize する。どちらも `rax` destination ではないため、
  `rax` value availability は維持する。次 blocker は `41 ff d6` (`call r14`) であり、
  load ではなく unknown indirect control-flow boundary として次の focused PR Gate で扱う。
- 2026-06-11 の B8-G3l として、`41 ff d6` (`call r14`) を direct `call rel32` や
  RIP-relative load と混ぜず、register-indirect call boundary として model 化した。
  decode は `call_r14` で停止し、lift は
  `RegisterIndirectCallUnsupported { target: R14, call_site, return_to }` を持つ unsupported
  terminator に変換する。B8 debug bundle は lifted IR の frontier unsupported terminator を
  stable `register_indirect_call` boundary として report する。arbitrary indirect target
  execution、translation cache、fallback JIT/interpreter はまだ導入しない。
- 2026-06-11 の B8-G4a として、Mach-O entry image materialization を segment-relative
  PC から public `LC_SEGMENT_64.vmaddr` ベースの VM address space へ切り替えた。
  `MachOExecutableImagePlan` は selected segment の file range、segment `vmaddr`、
  entry segment offset、entry virtual address を分けて持つ。`ExecutableImage` は
  code segment base と entry PC の差分から entry bytes を切り出し、
  `ProgramImageMetadata` の mapped bytes / code / const-data range と B8 debug bundle
  は同じ Mach-O VM address space を report する。rebase / bind / import 解決は
  `loader.plan.json` の deferred step として残し、private dyld behavior には踏み込まない。
- 2026-06-11 の B8-G4b として、`call r14` boundary を public Mach-O import metadata
  boundary に接続した。debug bundle は `call_site=4294972996` / `return_to=4294972999`
  の register-indirect call と、直前の
  `mov r14, qword ptr [rip+disp32]` が読む `target_pointer_load.address=4294979672` を
  `loader.plan.json` に保存する。現 fixture は public load command として
  `LC_DYLD_CHAINED_FIXUPS dataoff=24576 datasize=584` を持つため、import symbol identity
  はまだ解決せず、helper boundary request は
  `import_symbol_identity_unresolved` の stable blocker とする。次の design slice は
  private dyld behavior ではなく public chained fixups payload decoder の最小実装である。
- 2026-06-12 の B8-G4c として、public `LC_DYLD_CHAINED_FIXUPS` payload の
  header、starts-in-image / starts-in-segment、`DYLD_CHAINED_IMPORT` table、
  uncompressed symbol strings、現 fixture に必要な `DYLD_CHAINED_PTR_64_OFFSET`
  bind chain entry を typed report として decode した。`call r14` の
  `target_pointer_load.address=4294979672` は `__DATA_CONST` chain の import ordinal
  11 へ解決され、`/usr/lib/libobjc.A.dylib` の `_objc_msgSend` import identity として
  `loader.plan.json` に保存される。これは import helper execution や Objective-C /
  AppKit bridge の実装ではなく、次の B8-G5 で helper boundary request と marshaling
  blocker へ接続するための public metadata decode boundary である。
- 2026-06-12 の B8-G5 として、decoded `_objc_msgSend` import identity を
  `import_helper_call` request の planning input に接続した。`loader.plan.json` と
  launch report は import identity、`target_register=r14`、`call_site=4294972996`、
  `return_to=4294972999`、`source_isa=x86_64` を保存し、helper execution ではなく
  `x86_64_argument_marshaling_unimplemented` /
  `helper_return_marshaling_unimplemented` の stable blocker で停止する。次の focused
  slice は Objective-C / AppKit bridge ではなく、B8-G5a の helper marshaling
  contract である。
- 2026-06-12 の B8-G5a として、`import_helper_call` request の
  `required_marshaling.contract` に `b8_import_helper_marshaling_contract_v0` を追加した。
  contract は x86_64 macOS System V calling convention、`rdi` receiver、`rsi`
  selector、`rax` return destination を stable report として保存する。これは
  `_objc_msgSend` 実行ではなく、次の B8-G5b で receiver / selector / return value
  materialization blocker を扱うための ABI/helper boundary contract である。
- 2026-06-12 の B8-G5b として、B8-G5a の marshaling contract から
  `b8_objc_message_materialization_boundary_v0` を追加した。boundary は `call r14` の
  直前にある `rdi` / `rsi` の materialization source を decode report から探し、
  current fixture ではどちらも RIP-relative qword load として report する。その qword
  value は `ProgramImageMetadata.mapped_bytes` から読むが、現 mapping は必要な data 側
  address をまだ覆っていないため、`receiver_mapped_image_qword_unavailable` /
  `selector_mapped_image_qword_unavailable` で止める。`rax` return destination は
  `write_helper_return_to_x86_64_rax` plan と
  `helper_return_value_materialization_unimplemented` blocker に留める。これは
  Objective-C / AppKit bridge や `_objc_msgSend` host execution ではなく、次の B8-G5c
  で public Mach-O mapped image metadata を広げるための materialization boundary である。
- 2026-06-12 の B8-G5c として、`ProgramImageMetadata.mapped_bytes` を executable entry
  segment だけでなく、public `LC_SEGMENT_64` の file-backed segment 全体から構成する
  ようにした。これにより current fixture の `__DATA.__objc_classrefs` /
  `__DATA.__objc_selrefs` にある receiver / selector qword load 元を stable report に
  保存できる。保存される値は file-backed mapped raw qword であり、private dyld state
  は使わない。current blocker は
  `receiver_mapped_value_fixup_resolution_unimplemented` /
  `selector_mapped_value_fixup_resolution_unimplemented` へ進むため、次の B8-G5d では
  public chained fixups / rebase / bind metadata に基づく raw qword resolution を扱う。
- 2026-06-12 の B8-G5d として、public `LC_DYLD_CHAINED_FIXUPS` metadata に基づき、
  ObjC receiver / selector の mapped raw qword を bind import または rebase VM address
  として解釈する report を追加した。current fixture では receiver が
  `_OBJC_CLASS_$_NSApplication` import identity、selector が Mach-O image-base relative
  rebase VM address `4294975648` として解決される。これは `_objc_msgSend` execution や
  Objective-C / AppKit bridge ではなく、argument materialization blocker を return value
  materialization blocker へ進めるための public loader/fixup boundary である。
- 2026-06-12 の B8-G5e として、ObjC helper return value を x86_64 `rax` に書き戻す
  `b8_objc_helper_return_writeback_boundary_v0` を stable report に追加した。boundary は
  source を `objc_helper_return_value`、destination を `x86_64_rax`、width を 64-bit、
  ordering を `after_helper_call_returns` として保存する。まだ helper result は生成せず、
  remaining blocker は `objc_helper_execution_unimplemented` に進める。次の B8-G6a では
  Objective-C / AppKit host execution ではなく、ObjC helper execution request boundary を
  stable report として分離する。
- 2026-06-12 の B8-G6a として、B8-G5e の
  `objc_helper_execution_unimplemented` を
  `b8_objc_helper_execution_request_v0` として stable report に分離した。request は
  `_objc_msgSend` の public import identity、receiver の
  `_OBJC_CLASS_$_NSApplication` import identity、selector の resolved VM address、x86_64
  `rax` return write-back boundary、required capability
  `objc_runtime_message_send_helper`、remaining blocker
  `objc_helper_execution_unimplemented` を 1 箇所に集約する。これは Objective-C runtime /
  AppKit API の host execution、arbitrary indirect call target execution、translation
  cache、fallback JIT/interpreter を追加するものではない。次の B8-G6b では、この required
  capability を public Objective-C runtime helper bridge contract として分離する。
- 2026-06-12 の B8-G6b として、`objc_runtime_message_send_helper` capability を
  `b8_objc_runtime_helper_bridge_contract_v0` として stable report に分離した。contract は
  input として `_objc_msgSend` import identity、receiver import identity、selector VM
  address、required capability を持ち、output として `objc_helper_return_value` と
  x86_64 `rax` return write-back boundary を持つ。error contract は
  `objc_runtime_helper_execution_unimplemented` として分類する。これは public Objective-C
  runtime / AppKit helper bridge の実行実装ではなく、次の B8-G6c で self-authored
  fixture に必要な host execution slice を接続するための contract 固定である。
- 2026-06-13 の B8-G6c として、B8-G6b の bridge contract が残した
  `objc_runtime_helper_execution_unimplemented` を self-authored B8 GUI fixture に必要な
  `_objc_msgSend(NSApplication, sharedApplication)` だけの host execution slice として
  扱う。selector identity は public Mach-O mapped bytes の NUL-terminated UTF-8 から
  `sharedApplication` として解決し、host execution は temporary Objective-C helper process
  を public Objective-C runtime / AppKit API で build/run する。戻り値は
  `objc_helper_return_value` / `host_pointer_u64` として report し、既存の
  `b8_objc_helper_return_writeback_boundary_v0` を `available` にして x86_64 `rax`
  write-back value へ接続する。これは arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter、Objective-C / AppKit 内部構造の再実装ではない。
  次の blocker は `objc_helper_return_continuation_unimplemented` であり、B8-G6d では
  `return_to` PC からの continuation boundary を report する。
- 2026-06-13 の B8-G6d として、B8-G6c の helper execution result が残した
  `objc_helper_return_continuation_unimplemented` を
  `b8_objc_helper_return_continuation_boundary_v0` として stable report に分離した。
  boundary は `call r14` の `call_site` / `return_to` / `target_register` を source
  として持ち、input には `objc_helper_return_value` / `host_pointer_u64` と既存の
  `b8_objc_helper_return_writeback_boundary_v0`、x86_64 `rax` へ書き戻した
  `written_value` を保存する。register state は `rax` が
  `objc_helper_return_value` 由来の 64-bit value を持つことを明示し、
  `next_source_pc=return_to` と
  `return_to_continuation_execution_unimplemented` を次 blocker として report する。
  これは `return_to` block の実行、arbitrary indirect call target execution、
  translation cache、fallback JIT/interpreter を追加するものではない。
- 2026-06-13 の B8-G6e として、G6d の
  `return_to_continuation_execution_unimplemented` を
  `b8_return_to_continuation_decode_boundary_v0` として stable report に分離した。
  boundary は `next_source_pc=4294972999` を `return_to_source_pc` として扱い、
  public Mach-O code segment bytes から continuation block 用の `X86Bytes` を作って
  既存 decoder に渡す。input には G6d が保存した x86_64 `rax` register state を保持し、
  `processed_source_pc_range`、`next_instruction`、`unsupported_instruction` を report する。
  現 fixture の次 blocker は `return_to_continuation_unsupported_instruction` であり、
  先頭 instruction は `4c 8b 3d ...` の unsupported REX/MOV である。これは
  `return_to` block の実行、arbitrary indirect call target execution、translation cache、
  fallback JIT/interpreter を追加するものではない。
- 2026-06-13 の B8-G6f として、G6e の
  `return_to_continuation_unsupported_instruction` のうち continuation block 先頭の
  `4c 8b 3d ...` を x86_64 `mov r15, qword ptr [rip+disp32]` として既存
  decode / lift / emit / debug report 境界に追加した。`b8_return_to_continuation_decode_boundary_v0`
  は input の x86_64 `rax` register state を保持したまま、先頭 instruction を
  `mov_r15_qword_ptr_rip_relative` として report し、次の unsupported opcode は
  `0x49` at `4294973006` へ進む。これは continuation block の一般実行や
  translation cache / fallback JIT を追加するものではなく、次の B8-G6g では
  `49 8b 3f` を `mov rdi, qword ptr [r15]` slice として扱う。
- 2026-06-13 の B8-G6g として、G6f の
  `return_to_continuation_unsupported_instruction` を受けて `49 8b 3f` を
  x86_64 `mov rdi, qword ptr [r15]` として decode / lift / debug report 境界に
  追加した。continuation report は input の x86_64 `rax` state と、直前の
  `mov r15, qword ptr [rip+disp32]` で materialize した `r15` raw qword を保持し、
  public `LC_DYLD_CHAINED_FIXUPS` 上で `r15` が AppKit `_NSApp` import に解決されることを
  `fixup_resolution` として保存する。`mov rdi, [r15]` は mapped image qword read ではなく
  imported global pointee load であるため、`rdi` materialization は
  `return_to_continuation_import_global_load_unimplemented` として blocked report に残す。
  これは一般的な dynamic library data symbol memory model、continuation block の一般実行、
  translation cache / fallback JIT を追加するものではない。次の B8-G6h では `_NSApp`
  imported global load を fixture-scoped boundary として扱う。
- 2026-06-13 の B8-G6h として、`_NSApp` imported global pointee load は
  self-authored B8 GUI fixture の `_objc_msgSend(NSApplication, sharedApplication)` host
  helper return value に限って materialize する。`rdi` materialized state は
  `source=imported_global_pointee_load`、`base_register=r15`、`base_fixup_resolution.import.symbol_name=_NSApp`、
  `value_source=objc_shared_application_helper_return_value` を保存する。これは一般的な
  imported global memory model、任意の dynamic library data symbol read、continuation block
  の一般実行、translation cache / fallback JIT を追加するものではない。次の B8-G6i では
  `31 d2` / `xor edx, edx` を focused ISA slice として扱う。
- 2026-06-13 の B8-G6i として、`31 d2` を x86_64 `xor edx, edx` 専用 slice として
  decode / lift し、32-bit register zeroing semantics により `rdx` が 64-bit zero へ
  materialize されることを `source=xor_edx_edx_zero`、`value=0`、`width=bits64` として
  continuation report に保存する。これは `xor r32, r32` 全体の一般化、continuation block
  の一般実行、translation cache / fallback JIT を追加するものではない。次の B8-G6j では
  到達済みの `call r14` at `4294973018` / `return_to=4294973021` を focused boundary として扱う。
- 2026-06-13 の B8-G6j として、continuation block 内の `call r14` を
  `b8_return_to_continuation_call_boundary_v0` として保存する。target は初回 helper call の
  `_objc_msgSend` import identity を `preserved_import_helper_call_target` /
  `x86_64_macos_system_v_callee_saved_register` として扱い、arguments は `rdi` の `_NSApp`
  value、`rsi` の `setActivationPolicy:` selector rebase、`rdx=0` を available state として
  report する。これは continuation block の一般実行、arbitrary indirect call target
  execution、translation cache / fallback JIT を追加するものではない。次の B8-G6k では
  `_objc_msgSend(NSApp, setActivationPolicy:, 0)` を focused helper boundary として扱う。
- 2026-06-13 の B8-G6k として、上記 continuation call boundary から
  `b8_return_to_continuation_objc_helper_boundary_v0` を派生し、helper request、
  bridge contract、available-or-blocked state を stable report に保存する。target
  `_objc_msgSend`、receiver `_NSApp` value、selector `setActivationPolicy:`、
  argument `rdx=0` は available だが、host execution はまだ行わず
  `return_to_continuation_objc_helper_execution_unimplemented` を次 blocker とする。
  これは `setActivationPolicy:` 以外の arbitrary Objective-C message send、
  continuation block の一般実行、arbitrary indirect call target execution、
  translation cache / fallback JIT を追加するものではない。次の B8-G6l では
  `_objc_msgSend(NSApp, setActivationPolicy:, 0)` だけの focused host execution slice を扱う。
- 2026-06-13 の B8-G6l として、`_objc_msgSend(NSApp, setActivationPolicy:, 0)` を
  public Objective-C runtime / AppKit API helper process で実行し、
  `b8_return_to_continuation_objc_helper_host_execution_v0` に helper output
  `bool_as_u64`、`next_source_pc=4294973021`、次の continuation decode boundary、
  次 blocker `return_to_continuation_unsupported_instruction` を保存する。これは
  `setActivationPolicy:` 以外の arbitrary Objective-C message send、return-to
  continuation の一般実行、arbitrary indirect call target execution、translation cache /
  fallback JIT を追加するものではない。次の B8-G6m では `4294973043` の
  `48 89 c2` / `mov rdx, rax` を扱うが、source `rax` は直前の `_objc_alloc_init`
  `call rel32` return value なので、単なる register-copy decode だけでなく return
  materialization blocker として扱う。
- 2026-06-13 の B8-G6m として、`48 89 c2` / `mov rdx, rax` を focused x86_64
  register-copy slice として decode / lift / emit / debug report に追加した。debug
  bundle では直前の `call_rel32` at `4294973028` / target `4294973108` /
  return_to `4294973033` を `source_call_return` として `rdx` materialization blocker に
  保存し、next blocker を
  `return_to_continuation_call_rel32_return_value_materialization_unimplemented` に進める。
  同じ continuation block は `call r14` at `4294973046` / return_to `4294973049` と
  selector `setDelegate:` まで decode / report するが、`objc_alloc_init` 全般、
  arbitrary register-copy execution、general call-rel32 helper execution、arbitrary
  Objective-C message send、translation cache / fallback JIT は追加しない。次の B8-G6n
  では public Mach-O stub / symbol / import metadata を使って、この `call_rel32`
  return value の helper boundary または stable unresolved-stub blocker を扱う。
- 2026-06-13 の B8-G6n として、public Mach-O `section_64.reserved1/reserved2`、
  `LC_DYSYMTAB` indirect symbol table、`LC_SYMTAB` / string table から `__stubs`
  target `4294973108` を `_objc_alloc_init` に解決する focused resolver を
  `bara-oracle` に追加した。B8 debug bundle は `call_rel32` at `4294973028` /
  return_to `4294973033` を
  `b8_return_to_continuation_call_rel32_helper_boundary_v0` として保存し、`rax` return
  value が `mov rdx, rax` により `setDelegate:` argument へ渡る dataflow を
  `b8_return_to_continuation_call_rel32_return_value_dataflow_v0` として記録する。次
  blocker は `return_to_continuation_call_rel32_helper_execution_unimplemented` である。
  これは arbitrary call-rel32 execution、general dyld stub binding、arbitrary Objective-C
  allocation / initialization、translation cache / fallback JIT を追加するものではない。
  次の B8-G6o では `_objc_alloc_init` helper execution request と class argument
  materialization / bridge blocker を focused slice として扱う。
- 2026-06-13 の B8-G6o として、`_objc_alloc_init` call-rel32 helper boundary に
  `b8_return_to_continuation_call_rel32_helper_execution_request_v0` を追加した。直前の
  `mov rdi, qword ptr [rip+disp32]` を mapped bytes と public chained fixups から
  materialize し、class argument は `address=4294988128`、`resolved_rebase=4294988184`
  として available state になる。helper execution request は `_objc_alloc_init`、
  `objc_alloc_init_helper` capability、`x86_64_rax` writeback boundary、class bridge
  `b8_return_to_continuation_objc_alloc_init_class_bridge_v0` を保存し、next blocker を
  `return_to_continuation_objc_alloc_init_class_bridge_unimplemented` に進める。これは
  arbitrary Objective-C class allocation / initialization bridge、general call-rel32
  execution、general dynamic symbol resolver、translation cache / fallback JIT を追加する
  ものではない。次の B8-G6p では `BaraGuiHelloWorldDelegate` に限る self-authored fixture
  class bridge / identity blocker を扱う。
- 2026-06-13 の B8-G6p として、`class_rebase.resolved_vm_address=4294988184` を
  public `LC_SYMTAB` / `nlist_64.n_value` で
  `_OBJC_CLASS_$_BaraGuiHelloWorldDelegate` に解決する focused resolver を追加した。
  B8 debug bundle は
  `b8_return_to_continuation_objc_alloc_init_class_identity_v0` と
  `b8_return_to_continuation_mach_o_symbol_address_resolution_v0` を保存し、
  `bridge_state=fixture_delegate_bridge_unimplemented`、next blocker
  `return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented` に進める。
  これは arbitrary Objective-C class / instance bridge、Objective-C object layout、
  private runtime metadata、general `_objc_alloc_init` execution、translation cache /
  fallback JIT を追加するものではない。次の B8-G6q では
  `BaraGuiHelloWorldDelegate` に限る host-side substitute bridge contract を扱う。
- 2026-06-13 の B8-G6q として、`_objc_alloc_init` の
  `BaraGuiHelloWorldDelegate` fixture 専用 bridge contract を
  `b8_return_to_continuation_objc_alloc_init_fixture_delegate_bridge_contract_v0` として
  保存した。contract は `public_mach_o_symtab_nlist64` class identity、self-authored
  fixture scope、`objc_alloc_init_fixture_delegate_host_substitute` capability、
  `host_pointer_u64` output、x86_64 `rax` return writeback、後続 `mov rdx, rax` /
  `setDelegate:` dataflow を明示し、next blocker を
  `return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_unimplemented` に
  進める。これは x86_64 binary 内の Objective-C object layout / method table / isa
  pointer 解釈、任意 class allocation、general `_objc_alloc_init` execution、
  delegate callback into translated code、translation cache / fallback JIT を追加するもの
  ではない。次の B8-G6r では public Objective-C / AppKit API helper による
  self-authored fixture delegate substitute の host execution を扱う。
- 2026-06-13 の planning update として、B8-HWGUI を self-authored Hello World GUI
  completion の大目標として明文化した。大目標の対象は、実 `LC_MAIN` entry から
  GUI lifecycle helper boundary までを通し、automated expected / actual と manual visible
  mode の両方で Hello World GUI fixture を完遂することに限定する。`/advance-large` を
  使う場合でも、debug bundle blocker 由来の focused slice を coherent commit として積み、
  arbitrary Objective-C message send、general continuation execution、arbitrary indirect
  target execution、translation cache / JIT、`.app` bundle / resource が必要になったら
  先に TODO 上の focused gate または sub-target へ分割する。
- 2026-06-13 の B8-G6v として、`NSApp run` は arbitrary AppKit lifecycle ではなく
  self-authored B8 GUI fixture 専用の public AppKit helper observation boundary とする。
  helper は fixture delegate の `applicationDidFinishLaunching:` 相当で
  `gui_window_created` event を stdout observation として保存し、
  `timer_after_gui_window_created` / `delay_millis=100` /
  `termination_request=ns_app_terminate_nil` で bounded に戻る。translated-code delegate
  callback execution、window lifecycle 一般化、`.app` bundle / nib / storyboard、
  translation cache / fallback JIT はこの boundary に含めない。
- 2026-06-13 の B8-G6w として、post-run continuation の `48 89 df` /
  `mov rdi, rbx` は `_objc_autoreleasePoolPop` へ渡す saved-register token handoff
  として扱う。`rbx` の値はこの命令で推測せず、
  `return_to_continuation_saved_register_value_materialization_unimplemented` として
  initial `_objc_autoreleasePoolPush` return value から別 gate で接続する。
- 2026-06-13 の B8-G6x として、initial `_objc_autoreleasePoolPush` の
  `call_rel32` return value が `mov rbx, rax` で preserved `rbx` に保存され、
  post-run `mov rdi, rbx` で `_objc_autoreleasePoolPop` token argument へ渡る
  handoff を `b8_return_to_continuation_saved_register_value_v0` として report する。
  `objc_autoreleasePoolPush` token value は public Objective-C runtime helper で
  push/pop した観測値として保存するが、raw helper-process pointer は
  `not_reused_across_helper_processes` と明示し、実 `_objc_autoreleasePoolPop`
  helper execution は次 gate へ分離する。
- 2026-06-13 の B8-G6y として、post-run `_objc_autoreleasePoolPop` は
  focused call-rel32 helper boundary として扱う。fixture の raw token pointer は
  helper process 間で再利用せず、public Objective-C runtime helper は fresh
  helper-process token で push/pop capability を観測する。boundary が executed の場合、
  continuation blocker は `_objc_autoreleasePoolPop` ではなく post-run epilogue の
  `48 83 c4 08` / `add rsp, 8` に進める。
- 2026-06-13 の B8-G6z として、post-run `48 83 c4 08` / `add rsp, 8` は
  arbitrary stack arithmetic ではなく `_objc_autoreleasePoolPop` 後の epilogue
  stack restore として扱う。`b8_return_to_continuation_epilogue_stack_adjustment_v0`
  は `instruction=add_rsp_imm8`、`stack_pointer_register=rsp`、
  `stack_pointer_delta=X86Imm8(8)`、次 blocker `DecodeUnsupportedOpcode { opcode: 91 }`
  at `4294973076` を保存し、次 gate は preserved `rbx` restore に分離する。
- B8-HWGUI 完遂後の OSS app cycle は、任意の downloaded binary ではなく、まず public
  source から x86_64 macOS binary を reproducible に build できる小さい OSS GUI app を
  source-built fixture として扱う。候補選定、license / redistribution、supply-chain、
  clean-room checklist、expected/actual/debug bundle 保存場所を scope 化するまでは
  実装へ進まない。
- B8 の一般アプリ化でぶつかりそうな壁の初期順序は、debug bundle、実 Mach-O entry、
  x86_64 ISA coverage、Mach-O loader execution、dynamic library / import boundary、
  ABI / helper marshaling、Objective-C runtime / AppKit lifecycle、process state、
  indirect control flow / translation cache、macOS constraints / bundle resource とする。
  この順序は実 fixture と compiler output によって入れ替わり得るため、各 step では
  debug bundle の blocker report を source of truth にして次の作業を選ぶ。
  現状は AOT 的 pipeline を主軸にし、JIT / on-demand translation は unknown indirect
  target、callback、lazy binding、runtime-generated target が stable blocker として
  頻出し始めた段階で、translation cache、PC map、runtime helper boundary とセットで
  導入する。
- wasm2c platform adapter / NDA target adapter は本流 TODO ではなく、
  [将来構想メモ](future-research-concepts.md) の未確立構想として扱う。

## D6: User-space runtime

- [x] AOT、JIT、fallback interpreter、translation cache、artifact cache を同じ user-space runtime 境界から扱う。
- [x] executable memory、signal、exception、thread、TLS、memory protection を public OS API の範囲で整理する。
- [x] kernel extension、private dyld behavior、private OS hook を前提にしない。
- [x] Rosetta 2 型の OS 統合ではなく、Bara は user-space binary translation runtime として設計する。

メモ:

- ユーザー空間完結は Bara の重要な差別化点。
- B8 の実 x86_64 macOS アプリ起動では、process-wide 互換性が必要な箇所も、
  まず loader/runtime metadata と helper boundary で表現する。
- B8 の single-binary GUI Hello World は `.app` bundle や private dyld integration
  を前提にしない。user-space runtime は input Mach-O executable image、
  public system framework imports、entry trampoline、stack / argv / envp、
  launch report をそれぞれ分けて扱う。
- 2026-06-11 の B8 小ステップとして、`bara-runtime::UserSpaceLaunchPlan` に
  image mapping、entry trampoline、initial stack、helper boundary の準備責務を
  分けて置いた。actual launch report はこの plan を `runtime_preparation` として
  JSON projection する。これは実 loader 実行、private dyld behavior、AppKit /
  Objective-C runtime 内部の再実装を意味しない。
- 2026-06-11 の B8 小ステップとして、`UserSpaceLaunchPlan` に
  `integration_policy` を追加した。B8 GUI Hello World actual launch report は
  process scope を current user-space process とし、kernel extension、
  private kernel hook、private dyld behavior をすべて `not_required` として
  記録する。
- 2026-06-11 の B8 小ステップとして、`UserSpaceLaunchPlan` に
  `process_boundary` を追加した。loader、translation cache、runtime helper、
  artifact cache は current user-space process 内の責務として report する。
  これは cache 実装、AOT/JIT/fallback interpreter 実装、または process 外
  integration の追加ではない。
- 2026-06-11 の B8 小ステップとして、`UserSpaceLaunchPlan` に
  `executable_memory` を追加した。allocation は `mmap` private anonymous
  mapping、protection transition は `mprotect` read-write to read-execute、
  release は `munmap` として report する。これは GUI executable launch の
  実行ではなく、既存 runtime executable memory 境界を B8 launch plan に接続する
  public OS API policy である。
- 2026-06-11 の B8 小ステップとして、`UserSpaceLaunchPlan` に
  `execution_strategy` を追加した。JIT、AOT、fallback interpreter は同じ
  `user_space_runtime` boundary から selectable として report する。これは
  各 strategy の実装、selection policy、fallback engine 接続の追加ではない。
- 2026-06-11 の B8 小ステップとして、`UserSpaceLaunchPlan` に
  `bridge_boundary` を追加した。syscall bridge と OS API bridge は
  helper boundary の責務として report し、bridge 実装は core IR / ARM64 emit に
  埋め込まない。これは syscall 実行、OS API mapping、または helper 実装の追加
  ではない。
- 2026-06-11 の B8 小ステップとして、`UserSpaceLaunchPlan` に
  `platform_model`、`macos_constraints`、`fallback_policy` を追加した。
  signal / exception は user-space loader boundary、thread は initial thread
  only、TLS は deferred、memory protection は public OS virtual memory として
  report する。macOS code signing、W^X、hardened runtime は private bypass なしの
  documented behavior / public API 制約として扱う。fallback は unimplemented
  instruction、unknown indirect target、unsupported loader feature を stable
  blocker classification に落とし、interpreter と外部 fallback engine は候補だが
  未実装 / 未接続として report する。これは Rosetta 比較フィードバックサイクルを
  開始できる直前の境界固定であり、実 signal handler、thread/TLS 実行、
  fallback engine 実装、または expected / actual 差分修正の開始ではない。
- 2026-06-11 の B8 小ステップとして、`UserSpaceLaunchPlan` に
  `loader_execution` を追加した。metadata source は public Mach-O probe、
  entry は `LC_MAIN` entryoff、segment mapping は `LC_SEGMENT_64` file ranges、
  imports は dylib load commands から helper boundary、relocations は link-edit
  rebase / bind metadata、Objective-C runtime は helper boundary として report する。
  これは `unsupported_loader_feature` に対する最初の修正フィードバック対象を
  stable plan にするための model 化であり、実 image mapping、import 解決、
  rebase / bind 適用、Objective-C runtime bridge 実行の追加ではない。
- B8-G2 以降の runtime design は、次の順で現在の plan を実行可能な boundary へ
  変える。まず B8-D0 で debug bundle を整え、次に `LC_MAIN` entry から実 entry
  bytes を切り出し、first-block translation attempt と最初の blocker を保存する。
  次に必要な x86_64 ISA subset を corpus-driven に追加し、image mapping / rebase /
  bind / import helper request を public Mach-O metadata から解決する。その後、
  process state として initial stack、argv / envp、heap、TLS、initial thread、
  signals / exceptions、file descriptors を user-space runtime metadata と
  helper boundary に載せる。`.app` bundle や resource は single executable の限界が
  blocker になるまで scope に入れない。

## D7: Binary format input/output の分離

- [x] Mach-O / PE / ELF の input parser と output writer を別責務にする。
- [ ] input parser は public format から executable image metadata を作る。
- [x] output writer は target artifact を作る pure planning / serialization 境界にする。
- [x] writer が育つ場合は oracle crate から独立した crate へ切り出す。

メモ:

- 入力解析と出力生成は同じ Mach-O でも変更理由が違う。
- `bara-oracle` は比較・fixture・外部観測の責務に留め、artifact writer の置き場にしない。
- B3 の pure writer planning 境界は `bara-mach-o` crate に置く。`bara-oracle`
  には fixture / probe / external observation を残し、Mach-O artifact serialization
  は writer 側で育てる。
- B3 の初期 model は `__TEXT` segment、mandatory `__text` section、optional
  `__const` section、`_main` entry、`LC_SEGMENT_64` / `LC_MAIN` 相当の最小
  load command model に限定する。offset / size / byte serialization は次の
  serialization 境界で扱う。
- B3 の `clang` packaging 経路と pure writer 経路の差分検証は、現時点の
  writer maturity に合わせて `bara-mach-o` の公開仕様ベース model 比較として
  固定する。実 bytes の layout / serialization parity は output writer の
  serialization 境界を実装する後続作業で扱う。

## D8: Clean-room research boundary

- [x] Rosetta は black-box oracle としてのみ扱い、内部構造を設計根拠にしない。
- [ ] FEX-Emu / Box64 / QEMU user-mode は問題領域と外部挙動の比較対象に限定する。
- [ ] 研究メモには、実装根拠、比較対象、禁止情報の区別を明記する。
- [ ] 新しい設計判断を追加するときは public spec、自前 test、外部観測のどれに基づくかを記録する。

メモ:

- Bara は Rosetta clone ではなく、分解可能な user-space binary translation runtime の研究として進める。
- 既存実装の内部構造を模倣せず、公開仕様と自前検証に基づいて進める。
