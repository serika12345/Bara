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

### R2: Guest Image Model

Mach-O parsing / probing から runtime が使える `GuestImage` / `MachOImage` model を
切り出す。将来の PE / ELF を同じ interface に載せられるようにする。

完了条件:

- entry point、segments、mapped bytes、imports、fixups、symbol identity が domain type で表現される
- `bara-oracle` の external observation 責務と loader domain が分離される
- B8 debug bundle は新しい image model を使う

### R3: Translation Artifact And Debug Export

ARM64 block bytes、pcmap、fixups、helper requirements、source identity を
`TranslationArtifact` としてまとめる。debug export は可能にするが、ユーザー visible
app 生成を主経路にしない。

完了条件:

- artifact model が source identity と cache validation identity を持つ
- debug export command が artifact bytes / metadata を保存できる
- existing fixture actual/expected tests が artifact 経由でも通る

### R4: Runtime Dispatcher Foundation

modeled continuation chain を、typed runtime state と dispatcher 境界へ置き換える。

完了条件:

- guest PC / register / stack / helper return state が domain type で表現される
- direct fallthrough、direct call、return、helper return writeback の最小 dispatcher path がある
- unsupported indirect target は stable blocker として残る

### R5: Helper / ABI Bridge Generalization

B8 fixture 専用の Objective-C / AppKit helper を、typed `GuestCall -> HostService -> GuestReturn`
contract に一般化する。

完了条件:

- x86_64 macOS SysV argument materialization と return writeback が reusable contract になる
- Objective-C helper、libSystem helper、future Wine thunk が同じ boundary model に載る
- B8-HWGUI は specialized path ではなく generic helper bridge の fixture case になる

### R6: B8-OSS0 Source-Built GUI App

source-built OSS GUI app を対象に、B8-HWGUI で作った cycle を一般 app に広げる。
downloaded binary ではなく、license / supply-chain / reproducible build を scope 化してから
実装する。

完了条件:

- target app、license、build inputs、success criteria が固定される
- expected / actual / debug bundle 保存場所が決まる
- first unsupported boundary が stable report される

### R7: Wine Bridge Planning

Wine との接続を、Windows x64 OS personality として設計する。実装開始前に Wine 側が
担う責務と Bara 側が担う責務を分ける。

完了条件:

- PE / Wine / Bara の責務分担が文書化される
- callback、exception、TLS、thread、thunk call boundary の最小 plan がある
- 最初の CLI-level Windows x64 fixture target が定義される
