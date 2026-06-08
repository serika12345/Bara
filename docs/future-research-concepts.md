# 将来構想メモ

この文書は、Bara の本流 TODO から外した未確立な研究構想を保管する場所である。
ここにある項目は、現時点では実装マイルストーンでも本流 TODO でもない。

実装へ移す場合は、先に独立した目的、scope、clean-room 境界、検証方法、
本流 Bara との接続点を文書化し、`TODO.md` または専用 roadmap へ昇格する。

## 扱い

- `TODO.md` の B 系マイルストーンは、実 x86_64 macOS アプリ起動、
  推奨ステップとしての実 x86 32-bit アプリ対応、PE / Wine 接続前段までを
  本流として扱う。
- この文書の項目は、現在の実装順序を決める根拠にしない。
- NDA 系ターゲット、platform adapter、wasm2c、LLVM IR、Wasm については、
  公開仕様、自前 test、外部から観測できる挙動だけを実装根拠にする。
- 別プロジェクトとして確立するまでは、Bara core の API をこれらの構想へ
  過度に最適化しない。

## C1: Platform Abstraction / wasm2c 研究との合流

構想:

Wasm build 可能なオープンソースソフトウェアを `wasm2c` で C へ戻し、
`clang` のみが提供されるターゲットへ最小労力で移植する platform adapter
研究。Bara とは host helper ABI、platform abstraction、artifact packaging、
regression 基盤を共有できる可能性がある。

未確立 TODO:

- [ ] Bara helper boundary と wasm2c platform imports が共有できる host helper ABI を設計する。
- [ ] stdout、file I/O、time、memory allocation、input、audio、rendering、window/event loop を platform capability として分ける。
- [ ] open source core には platform interface、fake backend、tests だけを置く。
- [ ] NDA 系ターゲット固有の API、描画 surface binding、event pump、audio device binding、build glue は closed adapter に閉じる。
- [ ] `wasm -> wasm2c -> C -> clang target` 経路と `x86/x86_64 -> Bara IR -> ARM64/native artifact` 経路を同じ platform abstraction へ接続する。
- [ ] platform adapter は C ABI / Rust FFI / helper table のどれで接続するかを比較する。
- [ ] rendering backend は immediate mode / retained mode / GPU surface handoff の責務を分ける。
- [ ] NDA adapter がなくても public fake backend で CI と regression を回せるようにする。
- [ ] platform capability metadata を artifact report に含める。
- [ ] 将来の Wine 接続では、Wine API bridge と wasm2c platform bridge が同じ helper abstraction を共有できるか検証する。

## C2: LLVM IR / Wasm を副出力ターゲットとして扱う

構想:

Bara IR を semantic IR として維持したまま、LLVM IR や Wasm を backend 実験、
object generation 比較、portable verifier、sandboxed runner などの副出力へ
使えるか検証する。

未確立 TODO:

- [ ] Bara IR を LLVM IR や Wasm に置き換えず、semantic IR として維持する。
- [ ] LLVM IR は backend 実験、object generation 比較、最適化比較の副出力として扱う。
- [ ] Wasm は sandboxed test runner、可視化、portable verifier target の副出力として扱う。
- [ ] `HostTrap` / helper request は LLVM external declaration または Wasm import へ落とす。
- [ ] LLVM / Wasm に落とせない semantics は metadata と helper boundary で保持する。
- [ ] x86_32 / x86_64 の source mode、PC map、flags、部分レジスタ情報を副出力で失わない方針を決める。
