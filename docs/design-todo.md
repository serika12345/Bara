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
- 2026-06-14 の B8-ARCH1 audit として、B8-HWGUI 完遂後に残った責務集中を
  behavior を変えずに棚卸しした。`crates/btbc-cli/src/b8_debug_bundle.rs` は
  8510 行で、debug bundle I/O、real-entry attempt orchestration、report DTO、
  loader/import projection、modeled continuation state、Objective-C/AppKit helper process
  execution が同居している。`crates/btbc-cli/src/main.rs` は 4365 行で、
  command dispatch、command implementation、fixture/oracle path construction、
  command behavior tests が同居している。

B8-ARCH1 responsibility split audit:

| 現在地 | 主責務 | 問題 | 抽出先候補 | 最初の gate |
| --- | --- | --- | --- | --- |
| `btbc-cli/src/b8_debug_bundle.rs` の `B8Debug*Report` 群 | stable JSON report DTO / schema projection | DTO と orchestration が同じ巨大 file にあり、field 変更と execution 変更が混ざる | `btbc-cli/src/b8_debug_bundle/report.rs` | B8-ARCH2a |
| `generate_b8_debug_bundle` と末尾の read/write helpers | bundle directory layout、JSON/bin/repro file I/O | I/O が decode/lift/report assembly と同じ module にある | `btbc-cli/src/b8_debug_bundle/io.rs` | B8-ARCH2b |
| `B8RealEntryAttempt` | decode/lift/emit/runtime attempt orchestration | runtime attempt policy と debug report assembly が密結合している | `btbc-cli/src/b8_debug_bundle/attempt.rs`、後続で runtime application service | B8-ARCH2c |
| `B8DebugLoaderPlanReport` と import/fixup projection | public Mach-O metadata から loader/import/debug report を作る | loader domain と report DTO が混ざり、`bara-oracle` の parser result を CLI で解釈している | `GuestImage` / `MachOImage` domain model | B8-ARCH2 |
| return-to continuation / epilogue report 群 | B8 fixture の modeled continuation state と blocker classification | runtime dispatcher の前段 model が debug report と一体化している | runtime state / dispatcher planning module | B8-ARCH4 |
| `run_public_objc_*` と Objective-C source constants | public Objective-C/AppKit helper process build/run | `clang` process execution、temporary file、host API observation が report model と同居している | helper bridge / OS personality service boundary | B8-ARCH5 |
| `btbc-cli/src/main.rs` の `run_cli` と `run_*` 関数 | command dispatch と command implementation | CLI 境界が fixture/oracle/runtime commands を直接束ね、tests も同じ file にある | command modules + application service boundary | D1 follow-up |
| `bara-oracle/src/binary_format/` | public Mach-O probing、entry extraction、symbol/fixup resolver | oracle crate に runtime loader model の種が残っている | future loader/image crate or module | B8-ARCH2 |

抽出順:

1. B8-ARCH1a で ISA semantic coverage plan を finish し、code split 前に
   instruction coverage の責務語彙を固定する。
2. B8-ARCH2a で B8 debug bundle report DTO を module split する。schema と JSON output は
   変えない。
3. B8-ARCH2b で bundle file I/O と repro script generation を I/O boundary へ切り出す。
4. B8-ARCH2c で real-entry attempt orchestration と report assembly を分ける。
5. B8-ARCH2 で public Mach-O metadata 由来の `GuestImage` / `MachOImage` domain model を
   `bara-oracle` から runtime-facing boundary へ移す。
6. B8-ARCH4 / B8-ARCH5 で continuation state と Objective-C/AppKit helper bridge を
   dispatcher / OS personality boundary へ移す。
7. D1 follow-up として `main.rs` の command dispatch、command implementation、tests を
   分ける。これは behavior-preserving command split とし、feature work と混ぜない。

B8-ARCH2a result:

- 2026-06-14 に `crates/btbc-cli/src/b8_debug_bundle/report.rs` を追加し、entry
  bytes、decode instruction、unsupported instruction、artifact、launch、runtime
  attempt、blocker、stage / source / memory-width report DTO を
  `b8_debug_bundle.rs` から分けた。
- JSON schema 名、serde tag / rename、field 名は変えない。親 module は existing
  orchestration、loader/import projection、helper process execution、bundle file I/O、
  modeled continuation state を保持し、report DTO constructor だけを `report` module
  から使う。
- `B8DebugLoaderPlanReport` と import/fixup projection、Objective-C/AppKit helper
  bridge、return-to continuation report 群は、それぞれ B8-ARCH2 / B8-ARCH4 /
  B8-ARCH5 の責務境界が決まるまで同じ PR では動かさない。

B8-ARCH2b result:

- 2026-06-14 に `crates/btbc-cli/src/b8_debug_bundle/io.rs` を追加し、bundle
  directory layout、`B8DebugBundleOutputPaths`、JSON/bin/text file read/write helper、
  repro script generation を `b8_debug_bundle.rs` から分けた。
- output path JSON の field 名、bundle file 名、repro script command string は変えない。
  親 module は existing orchestration を保持し、bundle I/O boundary helper だけを
  `io` module から使う。
- `B8RealEntryAttempt` / decode-lift-emit-runtime attempt orchestration、report DTO、
  loader/import projection、Objective-C/AppKit helper process execution、runtime
  dispatcher は同じ PR では動かさない。

B8-ARCH2c result:

- 2026-06-14 に `crates/btbc-cli/src/b8_debug_bundle/attempt.rs` を追加し、
  `B8RealEntryAttempt`、decode/lift/emit/runtime attempt orchestration、
  unsupported terminator frontier helper を `b8_debug_bundle.rs` から分けた。
- `generate_b8_debug_bundle` は existing attempt result fields を読むだけにし、JSON
  output、blocker classification、runtime attempt behavior は変えない。
- bundle file I/O、report DTO、loader/import projection、Objective-C/AppKit helper
  process execution、runtime dispatcher は同じ PR では動かさない。

B8-ARCH2d result:

- 2026-06-14 に `crates/btbc-cli/src/b8_debug_bundle/loader.rs` を追加し、
  `B8DebugLoaderPlanReport`、direct loader plan metadata DTO、loader deferred step DTO を
  `b8_debug_bundle.rs` から分けた。
- `generate_b8_debug_bundle` は loader plan report を作り、`helper_boundary_request()`
  で existing helper boundary request を launch report へ接続する形にした。
- `loader.plan.json` schema 名、field 名、JSON output は変えない。
- import/fixup projection、Objective-C/AppKit helper process execution、modeled continuation
  state、runtime dispatcher、`GuestImage` / `MachOImage` 本体抽出は同じ PR では動かさない。

B8-ARCH2e result:

- 2026-06-14 に `crates/btbc-cli/src/b8_debug_bundle/import_boundary.rs` を追加し、
  `B8DebugImportBoundaryReport`、public import metadata report、dyld info / dylib /
  linkedit projection DTO、import boundary resolution / next action enum を
  `b8_debug_bundle.rs` から分けた。
- `loader.rs` は import boundary projection module を呼び、`helper_boundary_request()` で
  existing helper boundary request を launch report へ接続する形にした。
- `loader.plan.json` import boundary field 名、JSON output、helper boundary request の
  launch report 接続は変えない。
- helper request / marshaling、Objective-C/AppKit helper process execution、modeled
  continuation state、runtime dispatcher、`GuestImage` / `MachOImage` 本体抽出は同じ PR では
  動かさない。

B8-ARCH2f result:

- 2026-06-14 に `crates/btbc-cli/src/b8_debug_bundle/helper_boundary.rs` を追加し、
  `B8DebugHelperBoundaryRequestReport`、`B8DebugImportHelperRequestReport`、
  `B8DebugHelperMarshalingReport`、import helper marshaling contract DTO、helper boundary
  blocker / blocked reason を `b8_debug_bundle.rs` から分けた。
- `loader.rs`、`import_boundary.rs`、`report.rs` は helper boundary request type を
  `helper_boundary` module から使う形にした。`loader.plan.json` と launch report の
  helper boundary request field 名、schema 名、JSON output は変えない。
- Objective-C/AppKit helper process execution、modeled continuation state、runtime
  dispatcher、`GuestImage` / `MachOImage` 本体抽出は同じ PR では動かさない。

B8-ARCH2g result:

- 2026-06-14 に `crates/btbc-cli/src/b8_debug_bundle/guest_image.rs` を追加し、
  loader plan の image mapping summary DTO を `b8_debug_bundle.rs` から分けた。
- `loader.rs` は `B8DebugGuestImageMappingReport::from_entry_input` を呼ぶ形にし、
  `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output は
  変えない。
- import/fixup projection、helper boundary、Objective-C/AppKit helper process execution、
  modeled continuation state、runtime dispatcher、`GuestImage` / `MachOImage` 本体抽出は
  同じ PR では動かさない。

B8-ARCH2h result:

- 2026-06-20 に `crates/bara-runtime/src/guest_image/mod.rs` を追加し、
  parser 非依存の runtime-facing `GuestImage` shell を置いた。
- B8 debug bundle の image mapping DTO は、`MachOEntryFunctionInput` から直接値を
  組み立てるだけでなく、`GuestImage::mach_o_executable` へ射影してから existing
  report DTO へ戻す形にした。`loader.plan.json` の `image_mapping` field 名、
  nested field 名、serde 値、JSON output は変えない。
- `MachOImage` 本体、imports/fixups/symbol identity、`bara-oracle` からの loader domain
  抽出、Objective-C/AppKit helper process execution、modeled continuation state、
  runtime dispatcher は同じ PR では動かさない。

B8-ARCH2i result:

- 2026-06-20 に runtime-facing `GuestImage` shell が `ProgramImageMappedBytes` を
  保持するようにした。mapped bytes は引き続き `bara-oracle` の public Mach-O
  file-backed segment materialization 由来だが、runtime-facing image shell から
  read-only に参照できる。
- B8 debug bundle は `MachOEntryFunctionInput::program_image_metadata().mapped_bytes()` を
  `GuestImage::mach_o_executable` へ渡す。`loader.plan.json` の `image_mapping` field 名、
  nested field 名、serde 値、JSON output は変えない。
- imports/fixups/symbol identity、`MachOImage` 本体、`bara-oracle` からの loader domain
  抽出、Objective-C/AppKit helper process execution、modeled continuation state、
  runtime dispatcher は同じ PR では動かさない。

B8-ARCH2j result:

- 2026-06-20 に runtime-facing `GuestImage` shell が `ProgramImageImports` を
  保持するようにした。imports collection は引き続き `ProgramImageMetadata` 由来だが、
  runtime-facing image shell から read-only に参照できる。
- B8 debug bundle は `MachOEntryFunctionInput::program_image_metadata().imports()` を
  `GuestImage::mach_o_executable` へ渡す。`loader.plan.json` の `image_mapping` field 名、
  nested field 名、serde 値、JSON output は変えない。
- fixups/symbol identity、`MachOImage` 本体、`bara-oracle` からの loader domain
  抽出、Objective-C/AppKit helper process execution、modeled continuation state、
  runtime dispatcher は同じ PR では動かさない。

B8-ARCH2k result:

- 2026-06-20 に runtime-facing `GuestImage` shell が `ProgramImageRelocations` を
  保持するようにした。relocations collection は引き続き `ProgramImageMetadata` 由来だが、
  runtime-facing image shell から read-only に参照できる。
- B8 debug bundle は `MachOEntryFunctionInput::program_image_metadata().relocations()` を
  `GuestImage::mach_o_executable` へ渡す。`loader.plan.json` の `image_mapping` field 名、
  nested field 名、serde 値、JSON output は変えない。
- symbol identity、`MachOImage` 本体、`bara-oracle` からの loader domain 抽出、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime
  dispatcher は同じ PR では動かさない。

B8-ARCH2l result:

- 2026-06-20 に runtime-facing `GuestImageMetadata` aggregate を追加し、`GuestImage` が
  `ProgramImageMetadata` 由来の sections / mapped bytes / symbols / relocations / imports /
  unwind を aggregate 経由で保持するようにした。
- B8 debug bundle は `MachOEntryFunctionInput::program_image_metadata()` から
  `GuestImageMetadata::from_program_image_metadata` へ射影し、
  `GuestImage::mach_o_executable` へ渡す。`loader.plan.json` の `image_mapping` field 名、
  nested field 名、serde 値、JSON output は変えない。
- `MachOImage` 本体、`bara-oracle` からの loader domain 抽出、Objective-C/AppKit helper
  process execution、modeled continuation state、runtime dispatcher は同じ PR では動かさない。

B8-ARCH2m result:

- 2026-06-20 に runtime-facing `MachOImage` shell を追加し、valid `GuestImage` /
  `GuestImageMetadata` を Mach-O specific image model から read-only に参照できるようにした。
- B8 debug bundle は `MachOEntryFunctionInput` から `GuestImageMetadata` と
  `MachOImage::executable` を作ってから、existing `B8DebugGuestImageMappingReport` へ
  射影する。`loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、
  JSON output は変えない。
- `bara-oracle` からの loader domain 抽出、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime
  dispatcher は同じ PR では動かさない。

B8-ARCH2n result:

- 2026-06-20 に `MachOImage::executable_from_code_range` を追加し、Mach-O executable
  code segment の source / address-space 決定を runtime constructor 側に閉じた。
- B8 debug bundle は existing `MachOEntryFunctionInput` から `ProgramImageRange` と
  `GuestImageMetadata` を作り、`MachOImage::executable_from_code_range` を通して
  existing `B8DebugGuestImageMappingReport` へ射影する。`loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output は変えない。
- `bara-oracle` からの loader domain 抽出、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime
  dispatcher は同じ PR では動かさない。

B8-ARCH2o result:

- 2026-06-21 に `MachOImage::executable_from_program_image_metadata` を追加し、
  `GuestImageMappedBytesSource::ProgramImageMetadata` の選択と `GuestImageMetadata` assembly を
  runtime constructor 側に閉じた。
- B8 debug bundle は existing `MachOEntryFunctionInput` から `ProgramImageRange` と
  `ProgramImageMetadata` を渡し、`MachOImage::executable_from_program_image_metadata` 経由で
  existing `B8DebugGuestImageMappingReport` へ射影する。`loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output は変えない。
- `bara-oracle` からの loader domain 抽出、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime
  dispatcher は同じ PR では動かさない。

B8-ARCH2p result:

- 2026-06-21 に `MachOExecutableCodeRange` を追加し、`MachOImage` constructor は
  汎用 `ProgramImageRange` ではなく Mach-O specific code range domain type を受け取るようにした。
- B8 debug bundle は existing `MachOEntryFunctionInput` から calculated `ProgramImageRange` を
  作り、`MachOExecutableCodeRange` に変換して
  `MachOImage::executable_from_program_image_metadata` へ渡す。`loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output は変えない。
- `bara-oracle` からの loader domain 抽出、code range 計算、import/fixup/symbol projection の
  意味変更、Objective-C/AppKit helper process execution、modeled continuation state、
  runtime dispatcher は同じ PR では動かさない。

B8-ARCH2q result:

- 2026-06-22 に `MachOExecutableCodeRange::from_program_image_metadata` を追加し、
  `ProgramImageMetadata.sections()` の単一 code section から executable code range を
  選ぶ判断を runtime-facing Mach-O constructor 側に寄せた。
- `MachOImage::executable_from_program_image_metadata` は caller-provided
  `MachOExecutableCodeRange` を受け取らず、entry point と `ProgramImageMetadata` だけから
  code range selection と `GuestImageMetadata` assembly を行う。
- B8 debug bundle は existing `MachOEntryFunctionInput` から code bytes length 由来の
  `ProgramImageRange` を計算せず、entry point と `ProgramImageMetadata` を渡して
  existing `B8DebugGuestImageMappingReport` へ射影する。`loader.plan.json` の
  `image_mapping` field 名、nested field 名、serde 値、JSON output は変えない。
- `bara-oracle` からの loader domain 抽出、public Mach-O parser / resolver logic、
  import/fixup/symbol projection の意味変更、Objective-C/AppKit helper process execution、
  modeled continuation state、runtime dispatcher は同じ PR では動かさない。

B8-ARCH2r result:

- 2026-06-23 に `MachOExecutableEntryPoint` を追加し、Mach-O executable entry point
  address を runtime-facing Mach-O domain type として表すようにした。
- `MachOImage` constructor は generic `GuestImageEntryPoint` ではなく
  `MachOExecutableEntryPoint` を受け取り、underlying `GuestImageEntryPoint` への変換を
  `MachOImage` constructor 内に閉じる。
- B8 debug bundle は existing `MachOEntryFunctionInput` の entry address を
  `MachOExecutableEntryPoint` に変換して existing `B8DebugGuestImageMappingReport` へ射影する。
  `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output は
  変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2s result:

- 2026-06-23 に `MachOExecutableCodeSegment` を追加し、Mach-O executable code segment の
  range、source、address-space を runtime-facing Mach-O domain type として表すようにした。
- `MachOImage` constructor は generic `GuestImageSegment` ではなく
  `MachOExecutableCodeSegment` を受け取り、underlying `GuestImageSegment` への変換を
  `MachOImage` constructor 内に閉じる。
- `MachOImage::code_segment` は `Option<GuestImageSegment>` ではなく
  `MachOExecutableCodeSegment` を返し、valid code segment を持つ `MachOImage` invariant を
  API へ反映する。
- B8 debug bundle は existing `MachOEntryFunctionInput` から `MachOImage` を作り、
  existing `GuestImage` projection 経由で existing `B8DebugGuestImageMappingReport` へ射影する。
  `loader.plan.json` の `image_mapping` field 名、nested field 名、serde 値、JSON output は
  変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2t result:

- 2026-06-23 に `GuestImageMappedBytes` を追加し、mapped bytes source と
  `ProgramImageMappedBytes` payload を一体の runtime-facing value object として表すようにした。
- `GuestImageMetadata` は `mapped_bytes_source` と `ProgramImageMappedBytes` を別々に
  constructor へ受け取らず、`GuestImageMappedBytes` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageMappedBytes::from_program_image_metadata` 経由で source selection と payload clone を
  value object 側に閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `mapped_bytes_source()` と
  `mapped_bytes()` accessor は維持し、B8 debug bundle の existing
  `B8DebugGuestImageMappingReport` projection と `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2u result:

- 2026-06-23 に `GuestImageSections` を追加し、`ProgramImageSections` payload を
  runtime-facing value object として表すようにした。
- `GuestImageMetadata` は `ProgramImageSections` を直接 constructor へ受け取らず、
  `GuestImageSections` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageSections::from_program_image_metadata` 経由で sections clone を value object 側に
  閉じる。
- `GuestImage` / `GuestImageMetadata` の existing `sections()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2v result:

- 2026-06-23 に `GuestImageSymbols` を追加し、`ProgramImageSymbols` payload を
  runtime-facing value object として表すようにした。
- 意図は symbols payload を `GuestImageMetadata` の direct collection field から分け、
  後続の symbol identity / import projection 境界を意味変更なしに切り出しやすくすること。
- `GuestImageMetadata` は `ProgramImageSymbols` を直接 constructor へ受け取らず、
  `GuestImageSymbols` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageSymbols::from_program_image_metadata` 経由で symbols clone を value object 側に
  閉じる。
- これにより runtime-facing metadata assembly は symbols payload を型付き境界として扱える。
  `GuestImage` / `GuestImageMetadata` の existing `symbols()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2w result:

- 2026-06-23 に `GuestImageUnwindMetadata` を追加し、`ProgramUnwindMetadata` payload を
  runtime-facing value object として表すようにした。
- 意図は unwind metadata payload を `GuestImageMetadata` の direct collection field から分け、
  loader / exception 関連 metadata を後続で扱う場合の runtime-facing 境界を先に作ること。
- `GuestImageMetadata` は `ProgramUnwindMetadata` を直接 constructor へ受け取らず、
  `GuestImageUnwindMetadata` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageUnwindMetadata::from_program_image_metadata` 経由で unwind clone を value object 側に
  閉じる。
- これにより runtime-facing metadata assembly は unwind metadata payload を型付き境界として
  扱える。`GuestImage` / `GuestImageMetadata` の existing `unwind()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2x result:

- 2026-06-23 に `GuestImageImports` を追加し、`ProgramImageImports` payload を
  runtime-facing value object として表すようにした。
- 意図は imports payload を `GuestImageMetadata` の direct collection field から分け、
  import projection semantics を変えずに後続の loader/import 境界を扱いやすくすること。
- `GuestImageMetadata` は `ProgramImageImports` を直接 constructor へ受け取らず、
  `GuestImageImports` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageImports::from_program_image_metadata` 経由で imports clone を value object 側に
  閉じる。
- これにより runtime-facing metadata assembly は imports payload を型付き境界として扱える。
  `GuestImage` / `GuestImageMetadata` の existing `imports()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import projection semantics の意味変更、
  fixup/symbol projection の意味変更、Objective-C/AppKit helper process execution、
  modeled continuation state、runtime dispatcher は同じ PR では動かさない。

B8-ARCH2y result:

- 2026-06-23 に `GuestImageRelocations` を追加し、`ProgramImageRelocations` payload を
  runtime-facing value object として表すようにした。
- 意図は relocations payload を `GuestImageMetadata` の direct collection field から分け、
  relocation/fixup projection semantics を変えずに後続の loader/fixup 境界を扱いやすく
  すること。
- `GuestImageMetadata` は `ProgramImageRelocations` を直接 constructor へ受け取らず、
  `GuestImageRelocations` を受け取って保持する。
- `GuestImageMetadata::from_program_image_metadata` は
  `GuestImageRelocations::from_program_image_metadata` 経由で relocations clone を
  value object 側に閉じる。
- これにより runtime-facing metadata assembly は relocations payload を型付き境界として
  扱える。`GuestImage` / `GuestImageMetadata` の existing `relocations()` accessor は維持し、
  B8 debug bundle の existing `B8DebugGuestImageMappingReport` projection と
  `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、relocation/fixup projection semantics の意味変更、
  import/symbol projection の意味変更、Objective-C/AppKit helper process execution、
  modeled continuation state、runtime dispatcher は同じ PR では動かさない。

B8-ARCH2z result:

- 2026-06-23 に `crates/bara-runtime/src/guest_image/metadata.rs` を追加し、
  `GuestImageMetadata` aggregate と metadata value object 群を
  `guest_image/mod.rs` から分けた。
- 意図は `GuestImage` / `MachOImage` の image shell と metadata payload boundary の
  変更理由を分け、runtime image model の親 module を肥大化させずにすること。
- `guest_image/mod.rs` は metadata module を re-export し、
  `bara_runtime::GuestImageMetadata`、`GuestImageMappedBytes`、
  `GuestImageSections`、`GuestImageImports`、`GuestImageRelocations`、
  `GuestImageSymbols`、`GuestImageUnwindMetadata` などの existing public API 名を維持する。
- これにより metadata aggregate / value object 群の追加や調整を
  `guest_image/metadata.rs` 側で扱える。`GuestImage` / `MachOImage` の caller-visible
  behavior と B8 debug bundle の `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection semantics の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2aa result:

- 2026-06-23 に `crates/bara-runtime/src/guest_image/mach_o.rs` を追加し、
  `MachOImage`、`MachOExecutableEntryPoint`、`MachOExecutableCodeRange`、
  `MachOExecutableCodeSegment` を `guest_image/mod.rs` から分けた。
- 意図は generic `GuestImage` shell と Mach-O specific image shell の変更理由を分け、
  runtime image model の親 module が Mach-O constructor / executable code range の詳細で
  肥大化しないようにすること。
- `guest_image/mod.rs` は Mach-O module を re-export し、
  `bara_runtime::MachOImage`、`MachOExecutableEntryPoint`、
  `MachOExecutableCodeRange`、`MachOExecutableCodeSegment` の existing public API 名を維持する。
- `GuestImageSegment::mach_o_executable_code` は generic shell から外し、
  Mach-O executable code segment assembly は `MachOExecutableCodeSegment::new` 側へ閉じた。
  これにより Mach-O specific constructor boundary の追加や調整を `guest_image/mach_o.rs`
  側で扱える。
- `GuestImage` / `MachOImage` の caller-visible behavior と B8 debug bundle の
  `loader.plan.json` output は変えない。`bara-oracle` からの loader domain 抽出、
  entry extraction / load command interpretation、public Mach-O parser / resolver logic、
  import/fixup/symbol projection semantics の意味変更、Objective-C/AppKit helper process
  execution、modeled continuation state、runtime dispatcher は同じ PR では動かさない。

B8-ARCH2ab result:

- 2026-06-23 に `crates/bara-runtime/src/guest_image/image.rs` を追加し、generic
  `GuestImage` shell、entry point、segments、segment identity、image error を
  `guest_image/mod.rs` から分けた。
- 意図は generic image invariant と親 `guest_image/mod.rs` の変更理由を分け、
  parent module を submodule wiring / re-export / tests に近づけること。
- `guest_image/mod.rs` は generic image module を re-export し、
  `bara_runtime::GuestImage`、`GuestImageEntryPoint`、`GuestImageSegments`、
  `GuestImageSegment`、`GuestImageError` などの existing public API 名を維持する。
- これにより generic image invariant、entry point、segment identity、image error の追加や
  調整を `guest_image/image.rs` 側で扱える。metadata / Mach-O specific module と
  B8 debug bundle の `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection semantics の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

B8-ARCH2ac result:

- 2026-06-23 に `crates/bara-runtime/src/guest_image/tests.rs` を追加し、existing
  `guest_image` unit test 群を `guest_image/mod.rs` から分けた。
- 意図は parent module に残った test fixture / regression coverage と production module
  wiring の変更理由を分け、`guest_image/mod.rs` を submodule wiring / re-export /
  test module declaration に近づけること。
- `guest_image/mod.rs` は production type definitions を持たず、`image` / `mach_o` /
  `metadata` module wiring と public re-export、test module declaration だけを持つ。
- これにより production boundary の diff と test coverage の diff を分けて読める。
  existing `guest_image` test names / coverage、caller-visible behavior、B8 debug bundle の
  `loader.plan.json` output は変えない。
- `bara-oracle` からの loader domain 抽出、entry extraction / load command interpretation、
  public Mach-O parser / resolver logic、import/fixup/symbol projection semantics の意味変更、
  Objective-C/AppKit helper process execution、modeled continuation state、runtime dispatcher は
  同じ PR では動かさない。

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

## D4a: x86_64 ISA semantic coverage strategy

- [x] x86_64 coverage を opcode checklist ではなく semantic bucket catalog として管理する。
- [x] decode、canonical instruction、operand semantics、guest semantic IR、backend lowering、
  helper/fallback の責務を分ける。
- [x] unsupported instruction report は opcode だけでなく、operand shape、semantic bucket、
  required runtime service、fallback possibility を含める。
- [x] direct ARM64 lowering へ進める bucket と、helper / interpreter fallback へ逃がす
  bucket の判断基準を固定する。
- [x] permissive license decoder を採用する場合でも、lift / IR / runtime semantics は
  Bara の clean-room domain model として保持する。
- [x] Intel SDM、Arm A64 docs、ABI specs、Mach-O public docs を primary source として
  coverage catalog に紐づける。
- [x] Intel XED、iced-x86、Zydis、Capstone、Remill / McSema、FEX、Box64、DynamoRIO を
  dependency candidate / research reference / non-candidate に分類する。
- [x] dependency candidate を採用する前に、license、NOTICE、transitive dependency、
  Nix packaging、`verify-supply-chain` 対象を audit する。

メモ:

- 一般アプリ実行に広げるとき、Bara は x86 opcode を 1 つずつ平たく潰す設計にはしない。
  実装単位は opcode ではなく、命令を構成する意味の部品とする。
- 典型的な流れは
  `x86 bytes -> decoder -> canonical instruction -> operand semantics -> guest semantic IR -> lowering/helper/fallback`
  である。
- 抽象化できる bucket は、prefix / width / ModRM / SIB / immediate decode、
  register / memory / immediate operand read-write、RIP-relative / register-indirect /
  base+index*scale+disp addressing、integer ALU family、common flags builder、
  condition-code evaluation、stack / direct control-flow family、helper request boundary である。
- 個別に頑張る bucket は、x86 flags の細かい差分、partial register aliasing、
  implicit operands、string instructions、atomic / `LOCK`、SIMD / FP / x87 / MXCSR、
  environment-dependent instruction、self-modifying code、indirect branch / callback /
  exception / signal / TLS / thread である。
- hot path かつ共通性が高い bucket は direct ARM64 lowering へ進める。rare または複雑な
  bucket は最初は helper または interpreter fallback へ逃がしてよいが、silent fallback は
  禁止し、report に条件と不足している runtime service を保存する。
- B8-HWGUI で追加した focused instruction slice は、今後は
  `prologue/epilogue stack-control`、`RIP-relative data access`、`ObjC import helper call`、
  `fixture-scoped host service` などの semantic bucket へ再分類する。B8-ARCH1a はこの分類を
  implementation 前の design audit として扱う。
- OSS app cycle では、debug bundle の observed blocker を source of truth にしつつ、
  TODO は「次の opcode」ではなく「次の semantic bucket」として切る。
- 参照するとよい primary sources と permissive candidates は
  [Runtime Architecture Roadmap](runtime-architecture-roadmap.md) の
  `Reference Materials And Permissive Candidates` を source of truth とする。
  特に Intel SDM は x86_64 semantics の primary source、Intel XED / iced-x86 / Zydis は
  decoder dependency 候補、Capstone は debug disassembly 候補、Remill / McSema は
  lifting の責務分離の先行研究、FEX / Box64 / DynamoRIO は user-mode runtime /
  dispatcher / code cache の比較研究とする。
- QEMU user-mode、Valgrind、Ghidra、Binary Ninja、Rosetta は有用な比較対象になり得るが、
  GPL / LGPL / proprietary license または clean-room 境界の理由により、Bara core の
  permissive dependency candidate にはしない。

B8-ARCH1a semantic bucket catalog (`b8_arch1a_isa_semantic_bucket_catalog_v0`):

| bucket | B8-HWGUI で確認済みの focused slice | 現状態 | 次の抽象化境界 |
| --- | --- | --- | --- |
| prefix / width / register alias decode | `48 89 e5`、`41 57`、`41 56`、`41 5e`、`41 5f`、32-bit `xor edx,edx` | lift-ready / direct-lowering-ready の混在 | REX、operand width、32-bit write zero-extension、partial register aliasing を canonical operand semantics に分ける |
| register transfer | `mov rbx,rax`、`mov rdx,rax`、`mov rdi,rbx`、`mov rbp,rsp` | direct-lowering-ready | arbitrary register-copy engine ではなく、register family / width / writeback rule を先に domain type 化する |
| RIP-relative data access | `mov rax/rdi/rsi/r14/r15, qword ptr [rip+disp32]` | lift-ready、mapped image / fixup resolution required | `MemRipRelative` operand を loader/image model と接続し、raw qword、rebase、bind、import identity を区別する |
| RIP-relative address materialization | `lea rdi,[rip+disp32]`、`lea rsi,[rip+disp32]` | lift-ready | address calculation を memory load と分け、source VA / target VA を混ぜない API にする |
| register-indirect data access | `mov rdx,qword ptr [rax]`、`mov rdi,qword ptr [r15]` | helper-required / stable-blocker 併用 | mapped-image read、imported global pointee load、runtime memory read を別 bucket に分ける |
| stack frame and epilogue | `push rax/rbx/rbp/r14/r15`、`pop rbx/rbp/r14/r15`、`add rsp,8` | direct-lowering-ready for focused shapes | stack state model と callee-saved restore report を runtime dispatcher 境界へ移す |
| direct control flow | `call rel32`、`ret`、`jmp rel8`、`jcc rel8/rel32` | direct-lowering-ready for simple internal targets | external stub call、post-ret padding、function completion を terminator semantics として分ける |
| indirect / imported control flow | `call r14` -> `_objc_msgSend` | helper-required / stable-blocker | indirect target identity、import helper request、dispatcher/cache requirement を分ける |
| integer ALU and flags | `add/sub/cmp/test eax,*`、`xor eax,eax`、`xor edx,edx` | direct-lowering-ready for narrow set | common flags builder、condition-code evaluation、parity/auxiliary flags policy を分ける |
| byte load / zero extension | `movzx eax, byte ptr [rdi]` | lift-ready / direct-lowering-ready for fixture shape | memory width、zero extension、addressing mode を generic operand semantics へ移す |
| syscall / host service request | `syscall`、Bara host trap sentinel | helper-required / stable-blocker | syscall、libc/import helper、runtime helper、fixture host trap を separate capability にする |
| Objective-C / AppKit helper service | `sharedApplication`、`setActivationPolicy:`、`_objc_alloc_init`、`setDelegate:`、`run`、autorelease pool | fixture-scoped host service | generic `GuestCall -> HostService -> GuestReturn` contract へ移す |
| fallback / runtime state | unknown indirect target、callback、TLS、signal、thread、SIMD/FP/string/atomic | stable-blocker / future fallback-required | fallback interpreter or JIT は dispatcher interface 上の explicit capability として追加する |

status vocabulary:

- `decode_only`: public bytes から canonical instruction / operand shape までは分かるが、
  guest semantic IR へまだ載らない。
- `lift_ready`: guest semantic IR が source-level observable semantics を表せる。
- `direct_lowering_ready`: ARM64 backend が helper なしで deterministic に lowering できる。
- `helper_required`: OS personality、loader/import、ABI、host service が必要で、core translator
  だけで完結しない。
- `fallback_required`: interpreter / JIT / dispatcher fallback が必要な可能性が高い。
- `stable_blocker`: silent fallback せず、source PC、operand shape、required service、
  next action を report して停止する。

direct lowering / helper / fallback 判断基準:

- direct lowering は、guest-visible semantics が pure IR と target backend metadata だけで
  表現でき、hidden OS state、loader state、time、host object identity、callback を要求しない
  bucket に限定する。
- helper は、ABI marshaling、import symbol、syscall/libc/Objective-C/AppKit、loader fixup、
  process state など OS personality が責務を持つ bucket に使う。
- fallback は、unknown indirect target、self-modifying code、runtime-generated target、
  complex SIMD/FP/string/atomic、exception/signal/thread/TLS のように dispatcher state が
  ないと進めない bucket にだけ使う。fallback 可能性は report するが、実装は別 gate にする。

unsupported report schema 方針:

- 既存の opcode / `DecodeUnsupportedOpcode` だけでなく、`semantic_bucket`、
  `operand_shape`、`source_pc_range`、`raw_bytes_hex`、`required_runtime_service`、
  `fallback_possibility`、`clean_room_source`、`next_action` を持てる形に進める。
- B8 debug bundle の report DTO split 後に schema を広げる。B8-ARCH1a では schema 変更や
  Rust code change は行わない。

decoder / reference adoption checklist:

- dependency 採用は別 PR Gate とし、B8-ARCH1a では採用しない。
- 採用前に license / NOTICE / transitive dependency / Nix packaging / `deny.toml` /
  `nix develop -c ./scripts/verify-supply-chain` を確認する。
- decoder / disassembler dependency は canonical instruction construction までに留める。
  lift、IR semantics、runtime helper、metadata schema、fallback policy は Bara の
  clean-room domain model として保持する。
- Intel SDM、Arm A64 docs、ABI specs、Mach-O public docs を primary source とし、
  dependency candidate / research reference の分類は
  [Runtime Architecture Roadmap](runtime-architecture-roadmap.md) の
  `Reference Materials And Permissive Candidates` を source of truth とする。

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
- 2026-06-13 の B8-G6aa として、post-run `5b` / `pop rbx` は arbitrary pop /
  stack-memory semantics ではなく epilogue preserved-register restore として扱う。
  `b8_return_to_continuation_epilogue_register_restore_v0` は `instruction=pop_rbx`、
  `register=rbx`、`stack_slot_source=post_adjustment_stack_top`、次 blocker
  `DecodeUnsupportedOpcode { opcode: 65 }` at `4294973077` を保存し、次 gate は
  `41 5e` / `pop r14` に分離する。
- 2026-06-13 の B8-G6ab として、post-run `41 5e` / `pop r14` も
  epilogue preserved-register restore として同じ register restore report 配列へ保存する。
  `source=after_previous_epilogue_register_restore`、
  `stack_slot_source=sequential_epilogue_stack_top` として `pop_rbx` からの連続性を残し、
  next blocker は `DecodeUnsupportedOpcode { opcode: 65 }` at `4294973079` /
  `41 5f` `pop r15` prefix に進める。
- 2026-06-13 の B8-G6ac として、post-run `41 5f` / `pop r15` も同じ
  epilogue register restore 配列に追加する。`pop_rbx` / `pop_r14` / `pop_r15` は
  sequential restore として保存され、next blocker は
  `DecodeUnsupportedOpcode { opcode: 93 }` at `4294973081` / `5d` `pop rbp` に進む。
  frame-pointer restore と function return completion は次 gate へ分離する。
- 2026-06-13 の B8-G6ad として、post-run `5d` / `pop rbp` は arbitrary pop ではなく
  epilogue frame-pointer restore として同じ register restore report 配列に追加する。
  `role=post_run_epilogue_frame_pointer_restore`、`register=rbp` として保存し、`ret` at
  `4294973082` まで decode が進む。remaining blocker は `ret` 後の
  `DecodeUnsupportedOpcode { opcode: 0 }` at `4294973083` なので、return terminator /
  post-ret padding completion は次 gate へ分離する。
- 2026-06-13 の B8-G6ae として、post-run `c3` / `ret` は
  `b8_return_to_continuation_epilogue_return_completion_v0` として executed report に分離する。
  `ret` 直後の `DecodeUnsupportedOpcode { opcode: 0 }` at `4294973083` は
  `b8_return_to_continuation_post_ret_padding_boundary_v0` で
  `ignored_after_return_terminator` / `does_not_extend_function_body` として分類し、
  continuation boundary の `unsupported_instruction` は `null` に戻す。remaining blocker は
  `return_to_continuation_execution_unimplemented` に進むため、self-authored fixture の modeled
  completion 判定は次 gate へ分離する。
- 2026-06-13 の B8-G6af として、G6ae の final continuation は
  `b8_return_to_continuation_modeled_execution_completion_v0` を持つ executed boundary へ進める。
  条件は `NSApp run` helper chain 後の autorelease pool pop、epilogue stack/register restore、
  `ret` completion、post-ret zero padding classification がすべて揃うことに限定する。
  `NSApp run` は no-argument selector として未使用 `rdx` blocker を残さず、
  nested helper request / continuation boundary は `blocker=null`、
  `next_action=review_b8_hello_world_gui_completion` になる。automated expected/actual と
  manual visible mode は final B8-HWGUI review boundary の差分として report に残す。
- 2026-06-13 の B8-HWGUI final review として、automated expected/actual は
  `{"issues":[]}` で一致し、manual visible mode は WindowServer の on-screen window title /
  bounds と `manual-visible.launch-report.json` の `gui_window_created` event で確認した。
  この時点で self-authored Hello World GUI は review gate 到達とし、B8-OSS0 は branch
  review / merge 後に開始する。
- 2026-06-13 の B8-ARCH0 として、B8-HWGUI 後の主経路を「ユーザー visible な
  converted `.app` / arm64 executable 出力」ではなく、内部 `TranslationArtifact` /
  runtime cache / dispatcher / OS personality として固定する。file export は debug /
  review / regression 用の補助機能に留める。Rosetta 2 の公開情報では JIT は process
  内変換、AOT は system service 管理の storage artifact であり、通常の変換済み app として
  ユーザーが扱うものではないため、Bara も同じ問題分割を採用する。ただし Rosetta の内部
  artifact layout、private signing/cache mechanism、private dyld / kernel integration は
  実装根拠にしない。
- B8-ARCH0 以降の architecture は、同 OS / 異アーキテクチャを主対象にする。
  `macOS x86_64 -> macOS arm64`、`Linux x86_64 -> Linux arm64`、および
  `Windows x64 -> Wine on arm64` を代表形とする。異 OS 互換性は Bara core に埋め込まず、
  OS personality / Wine bridge に分離する。Wine 接続では PE loader、Windows API、
  registry、filesystem、windowing semantics は Wine の責務とし、Bara は x86_64
  guest code execution、Windows x64 ABI state、guest callback、exception / TLS /
  thunk boundary、translation cache を担当する。
- B8-HWGUI で `btbc-cli` と `b8_debug_bundle.rs` に集まった logic は、次の順で
  抽象化する。まず responsibility split audit、次に `GuestImage` / `MachOImage`
  extraction、`TranslationArtifact` / debug export、runtime dispatcher foundation、
  helper / ABI bridge generalization、OS personality boundary を進める。この順序は
  [runtime-architecture-roadmap.md](runtime-architecture-roadmap.md) と TODO の
  B8-ARCH1 以降を source of truth にする。
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
