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

- [ ] source ISA mode を `x86_64` / `x86_32` として明示する domain type を追加する。
- [ ] address size、operand size、stack width、calling convention を source mode から決める。
- [ ] register model は 64-bit register だけでなく、partial register を表現できる形にする。
- [ ] decoder / lifter / metadata schema の public 名称を `x86_64` 固定にしすぎない。

メモ:

- 現状の IR と lifter は x86_64 最小 subset として問題ない。
- B8 の実 x86_64 macOS アプリ起動では x86_64 を対象にするが、source
  mode を型として入れ、B9 の x86 32-bit アプリ対応を public API から
  閉じ出さない。
- B9 は B10 の PE / Wine 接続前に先に処理するのが望ましいが、blocker が
  大きい場合は記録したうえで飛ばしてよい推奨ステップとする。

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
- wasm2c platform adapter / NDA target adapter は本流 TODO ではなく、
  [将来構想メモ](future-research-concepts.md) の未確立構想として扱う。

## D6: User-space runtime

- [ ] AOT、JIT、fallback interpreter、translation cache、artifact cache を同じ user-space runtime 境界から扱う。
- [ ] executable memory、signal、exception、thread、TLS、memory protection を public OS API の範囲で整理する。
- [ ] kernel extension、private dyld behavior、private OS hook を前提にしない。
- [ ] Rosetta 2 型の OS 統合ではなく、Bara は user-space binary translation runtime として設計する。

メモ:

- ユーザー空間完結は Bara の重要な差別化点。
- B8 の実 x86_64 macOS アプリ起動では、process-wide 互換性が必要な箇所も、
  まず loader/runtime metadata と helper boundary で表現する。

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
