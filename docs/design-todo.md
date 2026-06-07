# 詳細設計 TODO / 設計メモ

この文書は、実装大項目とは分けて、設計上の判断、分割方針、
肥大化を防ぐための監査観点を残す場所とする。

実装 TODO は [TODO.md](../TODO.md) の B1-B12 に置き、ここには
「どの境界をどう切るべきか」「いつ設計を固定しすぎないようにするか」
を記録する。

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
- [ ] artifact metadata は実行結果とは分け、生成条件、target triple、toolchain、helper requirements を含める。
- [ ] 外部 toolchain 経路と pure writer 経路を同じ interface から選べるようにする。
- [ ] host unsupported、toolchain missing、link failure、execution failure を分類する error/report model を設計する。

メモ:

- Hello World では `clang` packaging で十分だが、将来の Mach-O writer、ELF/PE、NDA target packaging を考えると artifact model を先に固める。
- artifact は「ファイル」ではなく「生成物とその説明」として扱う。

## D3: Source ISA mode と x86 bit-width

- [ ] source ISA mode を `x86_64` / `x86_32` として明示する domain type を追加する。
- [ ] address size、operand size、stack width、calling convention を source mode から決める。
- [ ] register model は 64-bit register だけでなく、partial register を表現できる形にする。
- [ ] decoder / lifter / metadata schema の public 名称を `x86_64` 固定にしすぎない。

メモ:

- 現状の IR と lifter は x86_64 最小 subset として問題ない。
- ただし `x86 -> arm64` も最終目標に含めるなら、B9 の前に source mode を型として入れる。

## D4: Bara IR の責務

- [ ] Bara IR は semantic IR として維持し、LLVM IR / Wasm へ置き換えない。
- [ ] CFG、terminator、flags、stack、call、memory access、helper request を段階的に表現する。
- [ ] LLVM IR / Wasm へ落とすと失われる情報は metadata または helper boundary として保持する。
- [ ] IR validation は I/O を持たない pure report として返す。

メモ:

- LLVM IR / Wasm は出力ターゲットや検証ターゲットとして有用だが、Bara の中心IRにすると binary translation 固有の PC map、partial register、flags、例外境界を失いやすい。
- Wasm は portable verifier / sandbox runner として、LLVM IR は backend 比較として扱う。

## D5: Host helper / platform abstraction

- [ ] stdout、file I/O、time、memory allocation、input、audio、rendering、window/event loop を capability として分ける。
- [ ] Bara host helper ABI と wasm2c platform imports が共有できる最小 interface を設計する。
- [ ] helper request は core IR / emit に OS 固有処理を混ぜず、runtime boundary で解決する。
- [ ] open fake backend で CI と regression を回し、NDA adapter は closed layer に閉じる。

メモ:

- `hello world` の stdout helper は初期成功経路として妥当。
- 今後は Wine bridge、wasm2c platform adapter、NDA target adapter が同じ helper abstraction を使えるようにする。

## D6: User-space runtime

- [ ] AOT、JIT、fallback interpreter、translation cache、artifact cache を同じ user-space runtime 境界から扱う。
- [ ] executable memory、signal、exception、thread、TLS、memory protection を public OS API の範囲で整理する。
- [ ] kernel extension、private dyld behavior、private OS hook を前提にしない。
- [ ] Rosetta 2 型の OS 統合ではなく、Bara は user-space binary translation runtime として設計する。

メモ:

- ユーザー空間完結は Bara の重要な差別化点。
- process-wide 互換性が必要な箇所も、まず loader/runtime metadata と helper boundary で表現する。

## D7: Binary format input/output の分離

- [ ] Mach-O / PE / ELF の input parser と output writer を別責務にする。
- [ ] input parser は public format から executable image metadata を作る。
- [ ] output writer は target artifact を作る pure planning / serialization 境界にする。
- [ ] writer が育つ場合は oracle crate から独立した crate へ切り出す。

メモ:

- 入力解析と出力生成は同じ Mach-O でも変更理由が違う。
- `bara-oracle` は比較・fixture・外部観測の責務に留め、artifact writer の置き場にしない。

## D8: Clean-room research boundary

- [ ] Rosetta は black-box oracle としてのみ扱い、内部構造を設計根拠にしない。
- [ ] FEX-Emu / Box64 / QEMU user-mode は問題領域と外部挙動の比較対象に限定する。
- [ ] 研究メモには、実装根拠、比較対象、禁止情報の区別を明記する。
- [ ] 新しい設計判断を追加するときは public spec、自前 test、外部観測のどれに基づくかを記録する。

メモ:

- Bara は Rosetta clone ではなく、分解可能な user-space binary translation runtime の研究として進める。
- 既存実装の内部構造を模倣せず、公開仕様と自前検証に基づいて進める。
