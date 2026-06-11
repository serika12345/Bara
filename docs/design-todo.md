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
