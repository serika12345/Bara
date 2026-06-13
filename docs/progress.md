# プロジェクト進行履歴

この文書は、コミット履歴を読まなくても Bara がどのように進行してきたかを
把握できるようにするための進行記録である。

詳細な実装 TODO は [TODO.md](../TODO.md)、詳細設計とリファクタリング TODO は
[docs/design-todo.md](design-todo.md)、`hello world` までの詳細な段階履歴は
[docs/hello-world-roadmap.md](hello-world-roadmap.md) に置く。

## 現在の作業スナップショット

最終更新: 2026-06-13 21:16 JST

状態:

- project_state: completed。B8 は「一般アプリ対応」を 1 つの完了条件にせず、
  reviewable GUI 起動 slice の積み上げとして扱う。B8-G6c までで、self-authored
  x86_64 GUI fixture の実 `LC_MAIN` entry から
  `push rbp; mov rbp,rsp; push r15; push r14; push rbx; push rax; call rel32;
  mov rbx,rax; mov rax,qword ptr [rip+disp32]; mov rdx,qword ptr [rax];
  lea rdi,[rip+disp32]; lea rsi,[rip+disp32]; mov rdi,qword ptr [rip+disp32];
  mov rsi,qword ptr [rip+disp32]; mov r14,qword ptr [rip+disp32]; call r14` までを
  decode / lift / emit または stable unsupported boundary として扱える。さらに
  entry image は segment-relative offset ではなく public `LC_SEGMENT_64.vmaddr` ベースの
  Mach-O VM address space で materialize される。`call r14` は public
  `LC_DYLD_CHAINED_FIXUPS` payload の header / starts / imports / symbol strings /
  `DYLD_CHAINED_PTR_64_OFFSET` bind chain entry から `/usr/lib/libobjc.A.dylib` の
  `_objc_msgSend` import identity へ解決される。B8-G5 ではこの decoded import identity を
  `import_helper_call` request の planning input に接続し、`target_register=r14`、
  `call_site=4294972996`、`return_to=4294972999`、`source_isa=x86_64` とともに
  `loader.plan.json` / launch report へ保存する。helper execution は行わず、
  `x86_64_argument_marshaling_unimplemented` /
  `helper_return_marshaling_unimplemented` の stable blocker で停止する。B8-G5a では
  `b8_import_helper_marshaling_contract_v0` として x86_64 macOS System V calling
  convention、`rdi` receiver、`rsi` selector、`rax` return destination を
  stable report に保存し、次の blocker を receiver / selector / return value
  materialization に進める。B8-G5b では
  `b8_objc_message_materialization_boundary_v0` として、`call r14` 直前の `rdi` /
  `rsi` source definition を RIP-relative qword load として保存し、現
  `ProgramImageMetadata.mapped_bytes` では data 側 qword value をまだ読めないことを
  `receiver_mapped_image_qword_unavailable` /
  `selector_mapped_image_qword_unavailable` として分類する。`rax` return destination は
  `write_helper_return_to_x86_64_rax` plan と
  `helper_return_value_materialization_unimplemented` blocker で停止する。B8-G5c では
  `ProgramImageMetadata.mapped_bytes` を public `LC_SEGMENT_64` file-backed segment
  全体から構成し、receiver address `4294988120` と selector address `4294988072` の
  mapped raw qword を stable report に保存する。これにより mapped image qword
  unavailable blocker は解消した。B8-G5d では mapped raw qword を public
  `LC_DYLD_CHAINED_FIXUPS` bind / rebase metadata から解釈し、receiver を
  `_OBJC_CLASS_$_NSApplication` import identity、selector を resolved VM address
  `4294975648` として stable report に保存する。receiver / selector materialization
  blocker は解消し、次の blocker は `helper_return_value_materialization_unimplemented`
  に進む。B8-G5e では helper return value を x86_64 `rax` に書き戻す
  `b8_objc_helper_return_writeback_boundary_v0` を stable report に保存し、remaining
  blocker は `objc_helper_execution_unimplemented` に進む。B8-G6a では
  `b8_objc_helper_execution_request_v0` として `_objc_msgSend` import identity、
  receiver import identity、selector resolved VM address、return write-back boundary、
  required capability `objc_runtime_message_send_helper`、remaining blocker
  `objc_helper_execution_unimplemented` を 1 つの stable helper execution request に
  集約する。B8-G6b ではこの request を
  `b8_objc_runtime_helper_bridge_contract_v0` として public Objective-C runtime helper
  bridge contract に分離し、input / output / error contract、helper output
  `objc_helper_return_value`、error classification
  `objc_runtime_helper_execution_unimplemented` を stable report に保存する。
  B8-G6c ではこの blocker を self-authored B8 GUI fixture に必要な
  `_objc_msgSend(NSApplication, sharedApplication)` だけの host execution slice として扱い、
  public Objective-C runtime / AppKit API helper process の実行結果を
  `b8_objc_runtime_helper_host_execution_v0` として保存する。helper output は
  `objc_helper_return_value` / `host_pointer_u64` として report され、既存の
  `b8_objc_helper_return_writeback_boundary_v0` は `available` になり、x86_64 `rax`
  write-back value へ接続される。B8-G6d では、この helper output と `rax`
  write-back value を `b8_objc_helper_return_continuation_boundary_v0` に保存し、
  `call r14` の `return_to=4294972999` を `next_source_pc` として report する。
  B8-G6e では、この `next_source_pc` から public Mach-O code segment bytes を使って
  continuation block を decode し、
  `b8_return_to_continuation_decode_boundary_v0` に `next_instruction` /
  `unsupported_instruction` / input x86_64 `rax` register state を保存する。B8-G6f では
  先頭 `4c 8b 3d ...` を x86_64 `mov r15, qword ptr [rip+disp32]` として
  decode / lift / emit / debug report 境界に追加し、
  `next_instruction.kind=mov_r15_qword_ptr_rip_relative`、
  `unsupported_instruction.kind.reason=DecodeUnsupportedOpcode { opcode: 73, ... }` まで進める。
  B8-G6g では `49 8b 3f` を x86_64 `mov rdi, qword ptr [r15]` として
  decode / lift / debug report 境界に追加し、`r15` が public chained fixups 上で AppKit
  `_NSApp` import に解決されることを保存する。B8-G6h ではこの `_NSApp` imported global
  pointee load を self-authored B8 GUI fixture の `_objc_msgSend(NSApplication,
  sharedApplication)` host helper return value にだけ接続し、`rdi` を
  `imported_global_pointee_load` / `value_source=objc_shared_application_helper_return_value`
  として materialize する。continuation report は input の x86_64 `rax` register state、
  `r15` import identity、`rdi` available state、`blocked_register_materializations=[]` を
  保持し、次の blocker は `31 d2` at `4294973016` の
  `return_to_continuation_unsupported_instruction` に進む。B8-G6i では `31 d2` を
  x86_64 `xor edx, edx` 専用 slice として decode / lift し、32-bit register zeroing
  semantics により `rdx` が 64-bit zero へ materialize されることを
  `source=xor_edx_edx_zero`、`value=0`、`width=bits64` として continuation report に
  保存する。continuation decode は `call r14` at `4294973018` /
  `return_to=4294973021` まで進み、次の blocker は
  `return_to_continuation_execution_unimplemented` である。B8-G6j ではこの `call r14` を
  `b8_return_to_continuation_call_boundary_v0` として保存し、target は `_objc_msgSend`
  import identity を `preserved_import_helper_call_target` /
  `x86_64_macos_system_v_callee_saved_register` として扱う。arguments は `rdi` の `_NSApp`
  value、`rsi` の `setActivationPolicy:` selector rebase、`rdx=0` を available state として
  report する。B8-G6k ではこの boundary から
  `b8_return_to_continuation_objc_helper_boundary_v0` を派生し、target `_objc_msgSend`、
  receiver `_NSApp` value、selector `setActivationPolicy:`、argument `rdx=0` の
  helper request / bridge contract / available-or-blocked state を stable report に保存する。
  next blocker は `return_to_continuation_objc_helper_execution_unimplemented` である。
  B8-G6l では `_objc_msgSend(NSApp, setActivationPolicy:, 0)` を public Objective-C
  runtime / AppKit API helper process で実行し、
  `b8_return_to_continuation_objc_helper_host_execution_v0` に helper output
  `bool_as_u64`、`next_source_pc=4294973021`、次の continuation decode boundary、
  next blocker `return_to_continuation_unsupported_instruction` を保存する。次の blocker
  は `4294973043` の `48 89 c2` / `mov rdx, rax` decode 未対応であり、直前の
  `_objc_alloc_init` `call rel32` return value materialization と一緒に扱う必要がある。
  B8-G6m では `48 89 c2` / `mov rdx, rax` を focused x86_64 register-copy
  slice として decode / lift / emit / debug report に追加し、`rdx` の
  `source=register_copy_from_rax`、`source_call_return.call_site=4294973028`、
  `source_call_return.target=4294973108`、`source_call_return.return_register=rax` を
  stable report に保存する。next blocker は
  `return_to_continuation_call_rel32_return_value_materialization_unimplemented` である。
  B8-G6n では public Mach-O `section_64.reserved1/reserved2`、`LC_DYSYMTAB` indirect
  symbol table、`LC_SYMTAB` / string table から `call_rel32` target `4294973108` を
  `_objc_alloc_init` に解決し、`b8_return_to_continuation_call_rel32_helper_boundary_v0` と
  `b8_return_to_continuation_call_rel32_return_value_dataflow_v0` を保存する。`objc_alloc_init`
  return value は `rax` から `mov rdx, rax` で `setDelegate:` argument に渡る。next
  blocker は `return_to_continuation_call_rel32_helper_execution_unimplemented` である。
  B8-G6o では `_objc_alloc_init` helper execution request として、`rdi` class argument を
  public mapped bytes / chained fixups から materialize し、`address=4294988128` /
  `resolved_rebase=4294988184` を保存する。next blocker は
  `return_to_continuation_objc_alloc_init_class_bridge_unimplemented` である。
  B8-G6p では `class_rebase.resolved_vm_address=4294988184` を public `LC_SYMTAB` /
  `nlist_64.n_value` から `_OBJC_CLASS_$_BaraGuiHelloWorldDelegate` に解決し、
  `b8_return_to_continuation_objc_alloc_init_class_identity_v0` と
  `b8_return_to_continuation_mach_o_symbol_address_resolution_v0` を保存する。next
  blocker は `return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented`
  である。
  B8-G6q ではこの delegate identity を
  `b8_return_to_continuation_objc_alloc_init_fixture_delegate_bridge_contract_v0` に接続し、
  `objc_alloc_init_fixture_delegate_host_substitute` capability、`host_pointer_u64` output、
  x86_64 `rax` return writeback、後続 `mov rdx, rax` / `setDelegate:` dataflow を
  contract として保存する。next blocker は
  `return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_unimplemented` である。
  B8-G6r ではこの fixture delegate substitute を public Objective-C / AppKit API helper
  で実行し、
  `b8_return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_v0` に
  `status=executed`、`representation=host_pointer_u64`、x86_64 `rax` writeback、
  後続 `mov rdx, rax` / `setDelegate:` argument dataflow を保存する。next blocker は
  `setDelegate:` の
  `return_to_continuation_objc_helper_execution_unimplemented` に進む。
  B8-G6s では `setDelegate:` の helper request / bridge contract / host execution を
  `setActivationPolicy:` から分離し、same-helper-process fixture substitute で
  `BaraGuiHelloWorldDelegate` を `NSApp.delegate` に設定した。
  `b8_return_to_continuation_set_delegate_host_object_boundary_v0` は raw
  cross-process pointer を再利用しないことを
  `raw_argument_pointer_reuse=not_reused_across_helper_processes` として保存し、output は
  `objc_helper_void_return` / `void_no_return_value` / `no_x86_64_return_value_observed` として
  report される。B8-G6t ではこの void return の後続 continuation を x86_64 `rax`
  value なしで decode し、`return_to=4294973049` から `mov rdi, qword ptr [r15]` /
  `mov rsi, qword ptr [rip+disp32]` / `call r14` を stable report に保存した。
  preserved `r15` `_NSApp` import global と preserved `_objc_msgSend` target により、
  receiver `NSApp` と selector `run` は available state として materialize される。
  B8-G6u では selector `run` の `_objc_msgSend(NSApp, run)` を no-argument
  Objective-C helper request として扱い、x86_64 `rdx` argument を要求しない
  `argument_model=no_arguments` / `argument_state=not_required` contract に分けた。
  B8-G6v では `NSApp run` を public AppKit helper process で実行し、
  fixture delegate の `applicationDidFinishLaunching:` 相当から
  `gui_window_created` event を観測した。run loop は
  `timer_after_gui_window_created` / `delay_millis=100` /
  `termination_request=ns_app_terminate_nil` の bounded policy で戻り、
  next blocker は post-run continuation の
  `return_to_continuation_unsupported_instruction` at `source_pc=4294973062` に進む。
  B8-G6w では post-run continuation 先頭の `48 89 df` / `mov rdi, rbx` を
  decode / lift / emit / stable report に追加し、`rdi` は `register_copy_from_rbx`
  として `_objc_autoreleasePoolPop` argument になるが、`rbx` value はまだ
  materialize せず、next blocker は
  `return_to_continuation_saved_register_value_materialization_unimplemented` に進む。
  B8-G6x では initial `_objc_autoreleasePoolPush` return value を
  `mov rbx, rax` の preserved saved-register token として report し、post-run
  `mov rdi, rbx` は `rdi` に `source_saved_register_value` を持つ available state へ進む。
  `_objc_autoreleasePoolPop` の `call_rel32` helper boundary は
  `b8_return_to_continuation_autorelease_pool_pop_boundary_v0` として保存され、next blocker は
  `return_to_continuation_call_rel32_helper_execution_unimplemented` に進む。
  B8-G6y では `_objc_autoreleasePoolPop` boundary を public Objective-C runtime helper
  の fresh push/pop token observation として executed に進め、raw fixture token pointer は
  helper process 間で再利用しない。next blocker は post-run epilogue の
  `return_to_continuation_unsupported_instruction` at `source_pc=4294973072` に進む。
  B8-G6z では `48 83 c4 08` / `add rsp, 8` を post-run helper boundary 後の
  epilogue stack restore として `b8_return_to_continuation_epilogue_stack_adjustment_v0`
  に保存し、next blocker は `5b` / `pop rbx` at `source_pc=4294973076` に進む。
  B8-G6aa では `5b` / `pop rbx` を epilogue preserved-register restore として
  `b8_return_to_continuation_epilogue_register_restore_v0` に保存し、next blocker は
  `41 5e` / `pop r14` prefix at `source_pc=4294973077` に進む。
  B8-G6ab では `41 5e` / `pop r14` も epilogue preserved-register restore 配列に
  保存し、next blocker は `41 5f` / `pop r15` prefix at `source_pc=4294973079` に進む。
  B8-G6ac では `41 5f` / `pop r15` も epilogue preserved-register restore 配列に
  保存し、next blocker は `5d` / `pop rbp` at `source_pc=4294973081` に進む。
  B8-G6ad では `5d` / `pop rbp` を epilogue frame-pointer restore として保存し、
  `ret` at `source_pc=4294973082` まで decode が進む。remaining blocker は `ret`
  後の `DecodeUnsupportedOpcode { opcode: 0 }` at `source_pc=4294973083` である。
  B8-G6ae では `c3` / `ret` を epilogue return completion として
  `b8_return_to_continuation_epilogue_return_completion_v0` に保存し、`ret` 後の zero
  padding は `b8_return_to_continuation_post_ret_padding_boundary_v0` で
  `ignored_after_return_terminator` として分類した。B8-G6af では final continuation を
  `b8_return_to_continuation_modeled_execution_completion_v0` で executed completion とし、
  nested helper request / continuation boundary は `blocker=null`、
  `next_action=review_b8_hello_world_gui_completion` に進む。
  arbitrary dynamic library data symbol read、return-to continuation の一般実行、
  arbitrary call-rel32 execution、translation cache、fallback JIT/interpreter はまだ行わない。
- active_milestone: completed。[TODO.md](../TODO.md) の B8-HWGUI Self-Authored Hello
  World GUI Completion と B8-ARCH0 Post-HWGUI Runtime Architecture Record を
  `task/b8-hello-world-gui-complete` 上で完了した。B8-HWGUI は automated expected/actual、
  real-entry modeled completion、manual visible mode の launch report 保存まで完了し、
  draft PR https://github.com/serika12345/Bara/pull/49 を開いて review gate で停止中。
  B8-ARCH0 では、B8-HWGUI 後の主経路をユーザー visible な converted app 出力ではなく、
  internal translation artifact / runtime cache / dispatcher / OS personality として
  [runtime-architecture-roadmap.md](runtime-architecture-roadmap.md) に固定した。
- active_design_focus: Bara の主対象は同 OS / 異アーキテクチャ実行である。
  `macOS x86_64 -> macOS arm64` を最初の concrete target とし、将来
  `Linux x86_64 -> Linux arm64` と `Windows x64 -> Wine on arm64` を OS personality として
  接続する。Bara core は ISA translation、IR、artifact/cache、dispatcher、guest CPU state
  を担当し、loader、ABI、OS service、Wine bridge は差し替え可能な personality に分離する。
  B8-HWGUI 後の抽象化順は B8-ARCH1 responsibility split audit、B8-ARCH2 guest image
  model extraction、B8-ARCH3 translation artifact/debug export、B8-ARCH4 runtime
  dispatcher、B8-ARCH5 helper/ABI bridge、B8-ARCH6 OS personality boundary とする。
- active_branch: `task/b8-hello-world-gui-complete`。branch base は `2258806`
  (`docs: define b8 hello world gui completion target`)。latest pushed implementation commit is
  `32c8afb` (`feat: complete b8 modeled gui continuation`)。latest pushed documentation commit
  before this architecture record is `7248baf` (`docs: record b8 hwgui draft pr`)。
  この snapshot は B8-HWGUI / B8-ARCH0 completed state として更新されており、draft
  PR #49 の merge review までは B8-OSS0 や B8-ARCH1 implementation に進まない。
- related_todo: [TODO.md](../TODO.md) B8-D0 / B8-G2 / B8-G3 / B8-G3b / B8-G3c /
  B8-G3d / B8-G3e / B8-G3f / B8-G3g / B8-G3h / B8-G3i / B8-G3j / B8-G3k /
  B8-G3l / B8-G4 / B8-G4a / B8-G4b / B8-G4c / B8-G5 / B8-G5a /
  B8-G5b-G5e / B8-G6a / B8-G6b / B8-G6c / B8-G6d / B8-G6e / B8-G6f /
  B8-G6g / B8-G6h / B8-G6i / B8-G6j / B8-G6k / B8-G6l / B8-G6m /
  B8-G6n / B8-G6o / B8-G6p / B8-G6q / B8-G6r / B8-G6s / B8-G6t / B8-G6u /
  B8-G6v / B8-G6w / B8-G6x / B8-G6y / B8-G6z / B8-G6aa / B8-G6ab /
  B8-G6ac / B8-G6ad / B8-G6ae / B8-G6af / B8-HWGUI / B8-ARCH0 /
  B8-ARCH1 / B8-ARCH2 / B8-ARCH3 / B8-ARCH4 / B8-ARCH5 / B8-ARCH6 /
  B8-OSS0 / B8-WINE0。
- completed_work: B8-G1 として、Rosetta 手動確認済みの
  `target/b8/b8_gui_hello_world_visible_x86_64` を入力に使い、
  translated entry path が `appkit_gui_hello_world` host trap request を発行し、
  AppKit lifecycle helper capability を automated oracle mode / manual visible mode
  の両方から呼べるようにした。automated mode は Rosetta expected と actual の
  stable comparison が空 issue で一致する。manual visible mode は window close /
  `Command-Q` まで戻らず、GUI window 上の `hello world` label を目視確認できる。
  B8-G2 以降の一般アプリ化計画と、その前提になる B8-D0 debug bundle foundation を
  TODO / scope / design TODO に明文化した。B8-D0 として
  `generate-b8-debug-bundle <binary> <out-root>` を追加し、B8-G2 として同 bundle の
  entry source を public `LC_MAIN` entry に切り替えた。B8-G3 として `push rbp`
  (`0x55`) を通過できるようにし、B8-G3b として `mov rbp,rsp` (`48 89 e5`) を
  通過できるようにし、B8-G3c として `push r15` (`41 57`) を通過できるようにし、
  B8-G3d として `push r14` (`41 56`) を通過できるようにし、B8-G3e batch として
  `push rbx` (`53`) と `mov rbx,rax` (`48 89 c3`) を通過できるようにし、B8-G3f として
  `mov rax, qword ptr [rip+disp32]` (`48 8b 05 ff 19 00 00`) を通過できるようにし、
  B8-G3g として `mov rdx, qword ptr [rax]` (`48 8b 10`) を通過できるようにし、
  B8-G3h として `lea rdi, [rip+disp32]` (`48 8d 3d b3 10 00 00`) を通過できるようにし、
  B8-G3i として `lea rsi, [rip+disp32]` (`48 8d 35 b6 10 00 00`) を通過できるようにし、
  B8-G3j として `mov rdi, qword ptr [rip+disp32]` (`48 8b 3d 22 3b 00 00`) を通過できるようにし、
  B8-G3k として `mov rsi, qword ptr [rip+disp32]` (`48 8b 35 eb 3a 00 00`) と
  `mov r14, qword ptr [rip+disp32]` (`4c 8b 35 14 1a 00 00`) を通過できるようにし、
  B8-G3l として `call r14` (`41 ff d6`) を register-indirect call boundary として
  stable report できるようにした。B8-G4a として Mach-O entry image materialization を
  VM-addressed に切り替えた。B8-G4b として `loader.plan.json` に
  public import boundary を追加し、`call r14` と直前の
  `mov r14, qword ptr [rip+disp32]` の target pointer load を
  public `LC_DYLD_CHAINED_FIXUPS` metadata に接続した。現在の generated `blocker.json` は
  `unsupported_instruction` / `register_indirect_call` を `call_site=4294972996`、
  `return_to=4294972999`、`target=r14` で返し、
  `decode.report.json` は `push_rbp`、`mov_rbp_rsp`、`push_r15`、`push_r14`、
  `push_rbx`、`push_rax`、`call_rel32`、`mov_rbx_rax`、
  `mov_rax_qword_ptr_rip_relative`、`mov_rdx_qword_ptr_rax`、`lea_rdi_rip_relative`、
  `lea_rsi_rip_relative`、次の `call_rel32`、`mov_rdi_qword_ptr_rip_relative`、
  `mov_rsi_qword_ptr_rip_relative`、`mov_r14_qword_ptr_rip_relative`、`call_r14` を
  保存する。`launch.report.json` の `source_pc` は `4294972928`、processed source PC
  range は `4294972928..4294972999` である。`loader.plan.json` は
  `lc_segment64_file_range` 由来の `mach_o_virtual_address` mapping と、
  public rebase / bind / import 解決の deferred step を保存する。さらに B8-G4c として
  `chained_fixups.status=resolved_import`、`header.imports_format.kind=dyld_chained_import`、
  `pointer_format.kind=ptr64_offset`、`target_resolution.import.symbol_name=_objc_msgSend`、
  `target_resolution.import.dylib_path=/usr/lib/libobjc.A.dylib` を保存する。B8-G5 として
  `helper_boundary_request.request.kind=import_helper_call`、`source=public_dyld_chained_fixups_import`、
  `target_register=r14`、`call_site=4294972996`、`return_to=4294972999`、
  `required_marshaling.argument_model=x86_64_call_arguments`、
  `required_marshaling.return_model=x86_64_rax_return_value`、
  `helper_boundary_request.reason=import_helper_marshaling_unimplemented`、
  `next_action=define_import_helper_marshaling_contract` を保存する。B8-G5a として
  `required_marshaling.contract.schema=b8_import_helper_marshaling_contract_v0`、
  `calling_convention=x86_64_macos_system_v`、`argument_sources[0].role=objc_receiver` /
  `source.register=rdi`、`argument_sources[1].role=objc_selector` /
  `source.register=rsi`、`return_destination.destination.register=rax`、
  `next_action=define_objc_receiver_selector_materialization` を保存する。B8-G5b として
  `materialization_boundary.schema=b8_objc_message_materialization_boundary_v0`、
  receiver / selector の `source_definition.kind=rip_relative_qword_load`、
  `source_definition.target_register=rdi` / `rsi`、
  `mapped_value.source=program_image_metadata`、
  `receiver_mapped_image_qword_unavailable`、
  `selector_mapped_image_qword_unavailable`、
  `return_value.plan=write_helper_return_to_x86_64_rax`、
  `helper_return_value_materialization_unimplemented`、
  `next_action=extend_mach_o_mapped_image_metadata_for_objc_materialization` を保存する。
  B8-G5c として `ProgramImageMetadata.mapped_bytes` が public file-backed
  `LC_SEGMENT_64` segment を覆うようになり、
  receiver `mapped_value.address=4294988120` /
  `value=9227875636482146321`、selector `mapped_value.address=4294988072` /
  `value=4503599627378848` を保存する。mapped raw qword はまだ public fixup
  resolution 前の値であるため、stable blocker は
  `receiver_mapped_value_fixup_resolution_unimplemented` /
  `selector_mapped_value_fixup_resolution_unimplemented` に進み、
  `next_action=resolve_objc_argument_mapped_value_fixups` を保存する。B8-G5d として
  mapped raw qword に `fixup_resolution` を追加し、receiver
  `fixup_resolution.status=resolved_import` /
  `import.symbol_name=_OBJC_CLASS_$_NSApplication` /
  `import.dylib_path=/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit`、selector
  `fixup_resolution.status=resolved_rebase` /
  `rebase.resolved_vm_address=4294975648` を保存する。receiver / selector の argument
  materialization は `available` へ進み、materialization boundary の remaining blocker は
  `helper_return_value_materialization_unimplemented`、`next_action` は
  `define_helper_return_value_materialization` である。B8-G5e として
  `return_value.writeback_boundary.schema=b8_objc_helper_return_writeback_boundary_v0`、
  `source=objc_helper_return_value`、`destination=x86_64_rax`、`width=bits64`、
  `writeback_plan=write_helper_return_to_x86_64_rax`、
  `ordering=after_helper_call_returns` を保存する。helper result はまだ生成せず、
  materialization boundary と helper marshaling contract の remaining blocker は
  `objc_helper_execution_unimplemented`、`next_action` は
  `define_objc_runtime_helper_bridge` である。B8-G6a として
  `helper_execution_request.schema=b8_objc_helper_execution_request_v0`、
  `kind=objc_msg_send`、`source_import.symbol_name=_objc_msgSend`、
  `receiver_identity.symbol_name=_OBJC_CLASS_$_NSApplication`、
  `selector_vm_address.resolved_vm_address=4294975648`、
  `return_writeback_boundary.schema=b8_objc_helper_return_writeback_boundary_v0`、
  `required_capability=objc_runtime_message_send_helper` を保存する。helper execution request
  の remaining blocker は `objc_helper_execution_unimplemented`、next action は
  `define_objc_runtime_helper_bridge_contract` である。B8-G6b として
  `bridge_contract.schema=b8_objc_runtime_helper_bridge_contract_v0`、input contract、
  output contract、error contract を保存し、helper output を
  `objc_helper_return_value`、error classification を
  `objc_runtime_helper_execution_unimplemented` として分類する。まだ Objective-C runtime /
  AppKit helper の host execution は行わない。B8-G6c として
  `host_execution.schema=b8_objc_runtime_helper_host_execution_v0`、
  `api_boundary=public_objc_runtime_appkit`、`fixture_scope=self_authored_b8_gui_fixture`、
  `selector_identity.name=sharedApplication`、`output.helper_output=objc_helper_return_value`、
  `output.representation=host_pointer_u64`、`return_writeback.destination=x86_64_rax` を
  stable report に保存する。bridge contract の error classification は `null` になり、
  next blocker は `objc_helper_return_continuation_unimplemented`、next action は
  `continue_after_objc_helper_return` である。B8-G6d として
  `return_continuation.schema=b8_objc_helper_return_continuation_boundary_v0`、
  `source.kind=register_indirect_call_return`、`next_source_pc=4294972999`、
  `register_state.register=rax`、`register_state.source=objc_helper_return_value`、
  `blocker=return_to_continuation_execution_unimplemented`、`next_action=decode_return_to_continuation_block`
  を stable report に保存する。B8-G6e として
  `continuation_block.schema=b8_return_to_continuation_decode_boundary_v0`、
  `source.kind=return_to_source_pc`、`source.byte_source=mach_o_code_segment_bytes`、
  `decode_report.entry=4294972999`、`next_instruction.kind=unsupported`、
  `unsupported_instruction.kind.reason=DecodeUnsupportedOpcode { opcode: 76, ... }`、
  `blocker=return_to_continuation_unsupported_instruction`、`next_action=add_return_to_continuation_instruction_support`
  を stable report に保存する。B8-G6f として
  `4c 8b 3d b2 19 00 00` を
  `mov_r15_qword_ptr_rip_relative`、`address=4294979584` として report し、
  `processed_source_pc_range=4294972999..4294973007`、
  `unsupported_instruction.kind.reason=DecodeUnsupportedOpcode { opcode: 73, ... }` まで進める。
  B8-G6g として `49 8b 3f` を `mov_rdi_qword_ptr_r15` として report し、
  `r15` の `fixup_resolution` は AppKit `_NSApp` import に解決される。B8-G6h として
  `_NSApp` imported global pointee load を fixture-scoped helper value から
  materialize し、`rdi` の `source=imported_global_pointee_load`、
  `base_register=r15`、`base_value=9227875636482146304`、
  `value_source=objc_shared_application_helper_return_value` を stable report に保存する。
  `blocked_register_materializations=[]` になり、次の blocker は `31 d2` at
  `4294973016` の `return_to_continuation_unsupported_instruction` へ進む。B8-G6i として
  `31 d2` を `xor_edx_edx` として decode / lift し、`rdx` の
  `source=xor_edx_edx_zero`、`value=0`、`width=bits64` を stable report に保存する。
  decode は次の `call_r14` at `4294973018` / `return_to=4294973021` まで進み、
  blocker は `return_to_continuation_execution_unimplemented` になる。B8-G6j として
  `continuation_call_boundary.schema=b8_return_to_continuation_call_boundary_v0` を追加し、
  target `_objc_msgSend` は preserved `r14` call target、arguments は `rdi` `_NSApp`、
  `rsi` `setActivationPolicy:`、`rdx=0` として available state を stable report に保存する。
- B8-G6k として
  `continuation_call_boundary.objc_helper_boundary.schema=b8_return_to_continuation_objc_helper_boundary_v0`
  を追加し、target `_objc_msgSend`、receiver `_NSApp` value、selector
  `setActivationPolicy:`、argument `rdx=0` の helper request / bridge contract /
  available-or-blocked state を stable report に保存する。continuation block と
  helper boundary request の blocker は
  `return_to_continuation_objc_helper_execution_unimplemented` に進む。
- B8-G6l として
  `b8_return_to_continuation_objc_helper_host_execution_v0` を追加し、
  `_objc_msgSend(NSApp, setActivationPolicy:, 0)` だけを public Objective-C runtime /
  AppKit API helper process で実行する。helper output は `bool_as_u64` として保存し、
  helper 実行後の `next_source_pc=4294973021`、次の continuation decode boundary、
  next blocker `return_to_continuation_unsupported_instruction` を stable report に保存する。
- B8-G6m として `48 89 c2` / `mov rdx, rax` を decode / lift / emit / debug report に
  追加し、直前の `call_rel32` at `4294973028` / target `4294973108` / return_to
  `4294973033` の return value が `rax` source であることを
  `source_call_return` として保存する。`call r14` at `4294973046` /
  return_to `4294973049` と selector `setDelegate:` まで report し、next blocker は
  `return_to_continuation_call_rel32_return_value_materialization_unimplemented` に進む。
- B8-G6n として public Mach-O `section_64.reserved1/reserved2`、`LC_DYSYMTAB` indirect
  symbol table、`LC_SYMTAB` / string table から `__stubs` target `4294973108` を
  `_objc_alloc_init` に解決する focused resolver を追加した。debug bundle は
  `b8_return_to_continuation_call_rel32_helper_boundary_v0` に `call_site=4294973028`、
  `target=4294973108`、`symbol_table_index=46`、`symbol_name=_objc_alloc_init` を保存し、
  `b8_return_to_continuation_call_rel32_return_value_dataflow_v0` に `rax` return value が
  `mov rdx, rax` で `setDelegate:` argument へ渡ることを保存する。next blocker は
  `return_to_continuation_call_rel32_helper_execution_unimplemented` に進む。
- B8-G6o として `_objc_alloc_init` helper execution request を
  `b8_return_to_continuation_call_rel32_helper_execution_request_v0` として保存した。`rdi`
  class argument は `mov rdi, qword ptr [rip+disp32]` から
  `address=4294988128` / `fixup_resolution.status=resolved_rebase` /
  `resolved_vm_address=4294988184` として materialize される。class bridge は
  `b8_return_to_continuation_objc_alloc_init_class_bridge_v0` として保存し、next blocker は
  `return_to_continuation_objc_alloc_init_class_bridge_unimplemented` に進む。
- B8-G6p として public `LC_SYMTAB` / `nlist_64.n_value` から
  `resolved_vm_address=4294988184` を `_OBJC_CLASS_$_BaraGuiHelloWorldDelegate` に解決する
  focused resolver を追加した。debug bundle は
  `b8_return_to_continuation_objc_alloc_init_class_identity_v0`、
  `class_symbol_name=_OBJC_CLASS_$_BaraGuiHelloWorldDelegate`、
  `class_name=BaraGuiHelloWorldDelegate`、`symbol_vm_address=4294988184`、
  `bridge_state=fixture_delegate_bridge_unimplemented` を保存し、next blocker は
  `return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented` に進む。
- B8-G6q として `_objc_alloc_init` fixture delegate bridge contract を
  `b8_return_to_continuation_objc_alloc_init_fixture_delegate_bridge_contract_v0` として保存した。
  contract は `scope=self_authored_b8_gui_hello_world_delegate_fixture`、
  `source=public_mach_o_symtab_nlist64_and_self_authored_fixture`、
  `required_capability=objc_alloc_init_fixture_delegate_host_substitute`、
  `output_representation=host_pointer_u64`、`return_register=rax`、
  `consumer_register=rdx`、`consumer_selector_name=setDelegate:` を保持する。next blocker は
  `return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_unimplemented` に進む。
- B8-G6r として self-authored fixture delegate substitute を public Objective-C /
  AppKit API helper で実行し、
  `b8_return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_v0` に
  `status=executed`、`effect=alloc_init_fixture_delegate`、
  `representation=host_pointer_u64`、`return_writeback.destination=x86_64_rax` を保存した。
  `_objc_alloc_init` return value は `mov rdx, rax` で `setDelegate:` argument として
  available になり、`source_call_return` と
  `b8_return_to_continuation_call_rel32_return_value_dataflow_v0` に producer/consumer dataflow
  を保持する。next blocker は `setDelegate:` の
  `return_to_continuation_objc_helper_execution_unimplemented` に進む。
- B8-G6s として `setDelegate:` の helper request / bridge contract / host execution を
  `setActivationPolicy:` から分離し、public Objective-C / AppKit API helper で
  `NSApp setDelegate:<BaraGuiHelloWorldDelegate instance>` を実行した。
  `b8_return_to_continuation_set_delegate_host_object_boundary_v0` は
  `process_model=same_helper_process_fixture_substitute` と
  `raw_argument_pointer_reuse=not_reused_across_helper_processes` を保存する。
  `setDelegate:` output は `objc_helper_void_return` / `void_no_return_value` /
  `return_value_handling=no_x86_64_return_value_observed` であり、next blocker は
  `return_to_continuation_objc_helper_void_return_continuation_unimplemented` に進む。
- B8-HWGUI として、self-authored x86_64 Mach-O GUI Hello World fixture を実 `LC_MAIN`
  entry から GUI 起動完遂まで通す大目標、`/advance-large` 利用時の stop 条件、
  および B8-HWGUI merge 後に開始する B8-OSS0 source-built OSS GUI app automation target を
  TODO / design TODO に追加した。
- B8-ARCH0 として、B8-HWGUI 後の runtime architecture record を追加した。
  主経路は user-visible converted app output ではなく、internal translation artifact /
  runtime cache / dispatcher / OS personality とする。Wine 接続は Windows API 実装ではなく、
  Windows x64-on-Wine OS personality として扱う。
- remaining_work: B8-HWGUI / B8-ARCH0 review gate。draft PR #49 の review / merge までは
  B8-ARCH1 implementation や B8-OSS0 に進まない。general continuation execution、
  arbitrary Objective-C message send、translation cache、fallback JIT/interpreter、
  `.app` bundle / resource 一般化、Wine bridge 実装はまだ行わない。
- next_action: https://github.com/serika12345/Bara/pull/49 を review する。承認後は
  `/merge-reviewed` で main に取り込む。次の実装は B8-ARCH1 responsibility split audit から
  開始し、B8-OSS0 は抽象化 milestone の進行状況を見て開始する。
- verification:
  `nix develop -c cargo check -p btbc-cli`、
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`、
  `nix develop -c cargo run -q -p btbc-cli -- generate-b8-debug-bundle target/b8/b8_gui_hello_world_x86_64 /tmp/bara-b8-g6af-inspect`
  は通過。format 後に `nix develop -c ./scripts/verify` も通過した。
  Final review boundary では `target/b8-hwgui-review/expected.json` と
  `target/b8-hwgui-review/actual.json` の `compare-expected-actual` が `{"issues":[]}` で
  一致し、debug bundle の modeled completion も確認済み。manual visible は WindowServer で
  `Bara GUI Hello World` window title / bounds を確認し、
  `target/b8-hwgui-review/manual-visible.launch-report.json` が `mode=manual_visible`、
  `status=gui_visible_ready`、`exit_status=0`、`text=hello world` を保存している。
  B8-ARCH0 は docs-only change として `git diff --check` と
  `nix develop -c ./scripts/check-no-invisible-chars` が通過した。

直近で完了した作業:

- 2026-06-13 21:16 JST: B8-ARCH0 Post-HWGUI Runtime Architecture Record を追加した。
  [runtime-architecture-roadmap.md](runtime-architecture-roadmap.md) に最終目標、同 OS /
  異アーキテクチャ主軸、internal translation artifact / cache 主経路、layer boundaries、
  Wine 接続の責務分担、B8-ARCH1 以降の roadmap を記録した。TODO には
  B8-ARCH1 responsibility split audit、B8-ARCH2 guest image model、B8-ARCH3 translation
  artifact/debug export、B8-ARCH4 runtime dispatcher、B8-ARCH5 helper/ABI bridge、
  B8-ARCH6 OS personality boundary、B8-WINE0 Wine bridge planning を追加した。
- 2026-06-13 19:53 JST: B8-HWGUI Final expected/actual and manual visible review boundary を
  開始した。automated expected / actual は
  `target/b8-hwgui-review/expected.json` と `target/b8-hwgui-review/actual.json` の
  `compare-expected-actual` が `{"issues":[]}` で一致し、
  `feedback-report.json` は `status=matched` / `next_action=review_b8_milestone` /
  `current_blocker.classification=none` になった。debug bundle の launch report は
  `helper_boundary_request.status=executed`、`reason=null`、
  `review_b8_hello_world_gui_completion`、および
  `b8_return_to_continuation_modeled_execution_completion_v0` /
  `launch_path_status=completed` を保存している。
- 2026-06-13 20:01 JST: B8-HWGUI manual visible mode を確認した。
  `run-arm64-gui-hello-world-translated-visible` で helper process を起動し、WindowServer から
  `Bara GUI Hello World` window の on-screen title / bounds を確認した。AppKit terminate
  で window close まで通し、`target/b8-hwgui-review/manual-visible.launch-report.json` は
  `mode=manual_visible`、`status=gui_visible_ready`、`exit_status=0`、
  `stdout={"event":"gui_window_created","title":"Bara GUI Hello World","text":"hello world"}` を
  保存した。
- 2026-06-13 20:09 JST: B8-HWGUI completion docs を commit / push し、draft PR
  https://github.com/serika12345/Bara/pull/49 を開いた。review gate で停止し、merge
  review までは B8-OSS0 に進まない。
- 2026-06-13 19:32 JST: B8-G6af Self-authored continuation execution completion boundary を
  実装した。final continuation は
  `b8_return_to_continuation_modeled_execution_completion_v0` を保存し、
  `role=self_authored_hello_world_gui_launch_path`、
  `completion_model=modeled_real_entry_helper_continuation_chain`、
  `launch_path_status=completed`、`remaining_b8_hwgui_blocker=null` になる。
  `NSApp run` の no-argument selector では未使用 `rdx` argument blocker を残さず、
  import helper request / return continuation / nested continuation decode boundary は
  `status=executed`、`blocker=null`、`next_action=review_b8_hello_world_gui_completion`
  に進む。automated expected/actual comparison と manual visible mode は final
  B8-HWGUI review boundary の `pending_large_target_review` 差分として保存した。
- 2026-06-13 19:14 JST: B8-G6ae Post-run epilogue return terminator completion boundary を
  実装した。`c3` / `ret` at `4294973082..4294973083` を
  `b8_return_to_continuation_epilogue_return_completion_v0` として executed report に保存し、
  `DecodeUnsupportedOpcode { opcode: 0 }` at `4294973083` は
  `b8_return_to_continuation_post_ret_padding_boundary_v0` で
  `ignored_after_return_terminator` / `does_not_extend_function_body` として分類する。
  continuation boundary の `unsupported_instruction` は `null` になり、next blocker は
  `return_to_continuation_execution_unimplemented`。
- 2026-06-13 19:00 JST: B8-G6ad Post-run epilogue frame-pointer restore boundary を実装した。
  `5d` / `pop rbp` at `4294973081..4294973082` を `pop_rbp` として decode / lift し、
  `b8_return_to_continuation_epilogue_register_restore_v0` 配列に
  `role=post_run_epilogue_frame_pointer_restore`、`register=rbp`、
  `stack_slot_source=sequential_epilogue_stack_top` として保存する。`ret` at
  `4294973082..4294973083` まで decode され、next blocker は
  `DecodeUnsupportedOpcode { opcode: 0 }` at `4294973083`。
- 2026-06-13 18:51 JST: B8-G6ac Post-run epilogue preserved r15 restore boundary を実装した。
  `41 5f` / `pop r15` at `4294973079..4294973081` を `pop_r15` として decode / lift
  し、`b8_return_to_continuation_epilogue_register_restore_v0` 配列に
  `source=after_previous_epilogue_register_restore`、`register=r15`、
  `stack_slot_source=sequential_epilogue_stack_top` として保存する。next blocker は
  `DecodeUnsupportedOpcode { opcode: 93 }` / `5d` `pop rbp` at `4294973081`。
- 2026-06-13 18:40 JST: B8-G6ab Post-run epilogue preserved r14 restore boundary を実装した。
  `41 5e` / `pop r14` at `4294973077..4294973079` を `pop_r14` として decode / lift
  し、`b8_return_to_continuation_epilogue_register_restore_v0` 配列に
  `source=after_previous_epilogue_register_restore`、`register=r14`、
  `stack_slot_source=sequential_epilogue_stack_top` として保存する。next blocker は
  `DecodeUnsupportedOpcode { opcode: 65 }` / `41 5f` `pop r15` prefix at `4294973079`。
- 2026-06-13 18:29 JST: B8-G6aa Post-run epilogue preserved rbx restore boundary を実装した。
  `5b` / `pop rbx` at `4294973076..4294973077` を `pop_rbx` として decode / lift し、
  `b8_return_to_continuation_epilogue_register_restore_v0` に
  `role=post_run_epilogue_preserved_register_restore`、
  `source=after_epilogue_stack_adjustment`、`register=rbx`、
  `stack_slot_source=post_adjustment_stack_top` を保存する。next blocker は
  `DecodeUnsupportedOpcode { opcode: 65 }` / `41 5e` `pop r14` prefix at `4294973077`。
- 2026-06-13 18:20 JST: B8-G6z Post-run epilogue stack adjustment boundary を実装した。
  `48 83 c4 08` / `add rsp, 8` at `4294973072..4294973076` を
  `add_rsp_imm8` として decode / lift し、
  `b8_return_to_continuation_epilogue_stack_adjustment_v0` に
  `role=post_run_helper_boundary_stack_restore`、
  `source=after_autorelease_pool_pop_helper_return`、
  `stack_pointer_register=rsp`、`stack_pointer_delta=X86Imm8(8)` を保存する。
  `next_blocker_after_adjustment` は `DecodeUnsupportedOpcode { opcode: 91 }` /
  `pop rbx` at `4294973076`。
- 2026-06-13 18:08 JST: B8-G6y Autorelease pool pop helper boundary を実装した。
  `b8_return_to_continuation_autorelease_pool_pop_boundary_v0` は `status=executed`、
  `target_resolution.symbol_name=_objc_autoreleasePoolPop`、
  `token_argument.source=saved_rbx_from_autorelease_pool_push` を保存する。
  `b8_return_to_continuation_autorelease_pool_pop_host_execution_v0` は
  `effect=autorelease_pool_push_pop`、`input_token_model=fresh_helper_process_push_pop_token`、
  `raw_pointer_reuse=not_reused_across_helper_processes`、`output.helper_output=objc_helper_void_return`
  を保存する。next blocker は post-run epilogue の
  `return_to_continuation_unsupported_instruction` at `4294973072`。
- 2026-06-13 17:58 JST: B8-G6x Autorelease pool saved-register token materialization
  boundary を実装した。entry decode の `call_rel32` at `4294972938` /
  target `_objc_autoreleasePoolPush` と直後の `mov rbx, rax` を
  `b8_return_to_continuation_saved_register_value_v0` として report し、post-run
  `mov rdi, rbx` は `source_saved_register_value` 付きの available `rdi` state へ進む。
  `b8_return_to_continuation_autorelease_pool_pop_boundary_v0` は
  `_objc_autoreleasePoolPop` at `4294973065`、`role=autorelease_pool_token`、
  `source=saved_rbx_from_autorelease_pool_push` を保存し、next blocker は
  `return_to_continuation_call_rel32_helper_execution_unimplemented`。
- 2026-06-13 17:38 JST: B8-G6w Post-Run main continuation unsupported instruction
  boundary を実装した。`48 89 df` は `mov_rdi_rbx` として decode / lift / emit /
  debug report される。`return_to=4294973062` の continuation は
  `mov_rdi_rbx`、`call_rel32` to `_objc_autoreleasePoolPop` stub、`xor_eax_eax` まで
  decode し、`rdi` materialization は `source=register_copy_from_rbx` /
  `source_register=rbx` として block される。next blocker は
  `return_to_continuation_saved_register_value_materialization_unimplemented`、next action は
  `materialize_return_to_continuation_saved_register_value`。
- 2026-06-13 17:24 JST: B8-G6v AppKit run-loop lifecycle observation boundary
  を実装した。selector `run` の `_objc_msgSend(NSApp, run)` は public AppKit
  helper process で実行され、`b8_return_to_continuation_appkit_run_loop_boundary_v0`
  は `status=executed`、`lifecycle_observation.observed_event.event=gui_window_created`、
  `delegate_callback=applicationDidFinishLaunching:`、`bounded_termination_policy.trigger=timer_after_gui_window_created`、
  `delay_millis=100`、`termination_request=ns_app_terminate_nil` を report する。
  `run` は `objc_helper_void_return` / `void_no_return_value` /
  `return_value_handling=no_x86_64_return_value_observed` として扱われ、次 blocker は
  post-run continuation の `return_to_continuation_unsupported_instruction` at
  `source_pc=4294973062` に進む。
- 2026-06-13 17:09 JST: B8-G6u Return-To Continuation NSApp run Helper Boundary
  を実装した。selector `run` の `_objc_msgSend(NSApp, run)` は no-argument request
  として扱われ、`argument_model=no_arguments`、`argument_register=null`、
  `argument_state=not_required` を report する。`NSApp run` は
  `b8_return_to_continuation_appkit_run_loop_boundary_v0` で
  `execution_model=ns_application_run_loop_entry` として block され、next blocker は
  `return_to_continuation_appkit_run_loop_lifecycle_unimplemented` に進む。
- 2026-06-13 16:58 JST: B8-G6t Return-To Continuation setDelegate Void Return
  Continuation Decode を実装した。`setDelegate:` の void return 後は x86_64 `rax`
  value を要求せず、preserved `r15` `_NSApp` と preserved `_objc_msgSend` target から
  `return_to=4294973049` の `mov rdi, qword ptr [r15]` / `mov rsi, qword ptr
  [rip+disp32]` / `call r14` を decode した。selector `run` は available state になり、
  next blocker は `return_to_continuation_objc_helper_execution_unimplemented` である。
- 2026-06-13 16:46 JST: B8-G6s Return-To Continuation setDelegate Helper Execution
  Boundary を実装した。`setDelegate:` を `setActivationPolicy:` 専用 contract から分離し、
  same-helper-process fixture substitute で `BaraGuiHelloWorldDelegate` を `NSApp.delegate`
  に設定する public AppKit helper execution を保存した。raw pointer は process 間再利用せず、
  output は void return として扱う。next blocker は
  `return_to_continuation_objc_helper_void_return_continuation_unimplemented` であり、次は
  B8-G6t の void return continuation decode。
- 2026-06-13 16:31 JST: B8-G6r Return-To Continuation objc_alloc_init Fixture Delegate
  Host Execution を実装した。public Objective-C / AppKit API helper で self-authored
  `BaraGuiHelloWorldDelegate` substitute を alloc/init し、host pointer output を x86_64
  `rax` writeback として保存した。後続 `mov rdx, rax` は
  `source=register_copy_from_rax`、`source_call_return`、dataflow producer
  `_objc_alloc_init` として materialize され、`setDelegate:` argument が available になった。
  next blocker は `return_to_continuation_objc_helper_execution_unimplemented` であり、次は
  B8-G6s の `setDelegate:` helper execution boundary。
- 2026-06-13 16:14 JST: B8-G6q Return-To Continuation objc_alloc_init Fixture Delegate
  Bridge Contract を実装した。`BaraGuiHelloWorldDelegate` class identity を contract input
  に接続し、fixture scope、host substitute capability、`host_pointer_u64` output、x86_64
  `rax` writeback、後続 `mov rdx, rax` / `setDelegate:` dataflow を stable report に保存する。
  next blocker は
  `return_to_continuation_objc_alloc_init_fixture_delegate_host_execution_unimplemented` であり、
  次は B8-G6r の fixture delegate host execution。
- 2026-06-13 15:59 JST: B8-G6p Return-To Continuation objc_alloc_init Delegate Class
  Bridge を実装した。public `LC_SYMTAB` / `nlist_64.n_value` から
  `resolved_vm_address=4294988184` を `_OBJC_CLASS_$_BaraGuiHelloWorldDelegate` に解決し、
  `b8_return_to_continuation_objc_alloc_init_class_identity_v0` と
  `b8_return_to_continuation_mach_o_symbol_address_resolution_v0` を stable report に保存する。
  next blocker は
  `return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented` であり、次は
  B8-G6q の fixture delegate bridge contract。
- 2026-06-13 15:44 JST: B8-G6o Return-To Continuation objc_alloc_init Helper
  Execution Boundary を実装した。`_objc_alloc_init` helper execution request、`rdi`
  class argument materialization、`x86_64_rax` return writeback boundary、class bridge
  blocker を stable report に保存する。class argument は public mapped bytes /
  chained fixups から `address=4294988128`、`resolved_rebase=4294988184` として
  materialized state になり、next blocker は
  `return_to_continuation_objc_alloc_init_class_bridge_unimplemented` になった。arbitrary
  Objective-C allocation / initialization、general call-rel32 execution、general dynamic
  symbol resolver、translation cache / fallback JIT は追加していない。targeted 検証は
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary -- --nocapture`
  が通過した。full verify は `nix develop -c ./scripts/verify` が通過した。
- 2026-06-13 15:27 JST: B8-G6n Return-To Continuation call_rel32
  objc_alloc_init Helper Boundary を実装した。public Mach-O `section_64.reserved1/reserved2`、
  `LC_DYSYMTAB` indirect symbol table、`LC_SYMTAB` / string table から `__stubs` target
  `4294973108` を `_objc_alloc_init` に解決し、`call_rel32` at `4294973028` /
  return_to `4294973033` を helper boundary として保存する。`rax` return value が
  `mov rdx, rax` で `setDelegate:` argument へ渡る dataflow も保存し、next blocker は
  `return_to_continuation_call_rel32_helper_execution_unimplemented` になった。domain primitive
  baseline は `MachOStub*` newtype の constructor/accessor だけを追加した。これは B8 debug
  report が public Mach-O stub virtual address、stub index、indirect symbol table slot、
  symbol table index を JSON 境界で安定表示するための accessor 例外であり、raw byte reader
  や general dynamic symbol resolver を公開するものではない。targeted 検証は
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`、
  `nix develop -c cargo test -p bara-oracle`、
  `nix develop -c cargo test -p btbc-cli mach_o_stdout_input_reaches_pure_writer_serialization_plan -- --nocapture`
  が通過した。full verify は `nix develop -c ./scripts/verify` が通過した。
- 2026-06-13 14:03 JST: B8-G6j Return-To Continuation Call R14 Boundary Planning
  を実装した。continuation block 内の `call_r14` at `4294973018` / `return_to=4294973021` を
  `b8_return_to_continuation_call_boundary_v0` として保存し、target は `_objc_msgSend` import
  identity を preserved `r14` call target として扱う。arguments は `rdi` の `_NSApp` value、
  `rsi` の `setActivationPolicy:` selector identity、`rdx=0` を available state として
  report する。targeted 検証は
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture` が通過した。
  full verify は `nix develop -c ./scripts/verify` が通過した。progress 更新後に
  `nix develop -c ./scripts/check-no-invisible-chars` が通過した。draft PR
  <https://github.com/serika12345/Bara/pull/47> を開いた。
- 2026-06-13 13:37 JST: B8-G6i Return-To Continuation XOR EDX Zero Slice
  を実装した。`31 d2` を x86_64 `xor edx, edx` として decode / lift し、32-bit register
  zeroing semantics により `rdx` を 64-bit zero として materialize する。continuation
  report は `source=xor_edx_edx_zero`、`value=0`、`width=bits64` を保存し、次の decoded
  instruction は `call_r14` at `4294973018` / `return_to=4294973021` まで進む。targeted
  検証は `nix develop -c cargo test -p bara-isa-x86 xor_edx -- --nocapture` と
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture` が通過した。
  full verify は `nix develop -c ./scripts/verify` が通過した。progress 更新後に
  `nix develop -c ./scripts/check-no-invisible-chars` が通過した。
- 2026-06-13 13:06 JST: B8-G6h Return-To Continuation NSApp Global Load Boundary
  を実装した。`r15` の public chained fixups 解決が AppKit `_NSApp` import である場合に限り、
  self-authored B8 GUI fixture の `_objc_msgSend(NSApplication, sharedApplication)` host
  helper return value を `_NSApp` imported global pointee load の fixture-scoped 値として
  扱う。continuation report は input の `rax` state と `r15` import identity を保持し、
  `rdi` を `imported_global_pointee_load` として materialize する。次 blocker は
  `31 d2` at `4294973016` の `return_to_continuation_unsupported_instruction` である。
  検証は `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`、
  `nix develop -c ./scripts/verify` が通過した。progress 更新後に
  `nix develop -c ./scripts/check-no-invisible-chars` が通過した。
- 2026-06-13 12:45 JST: B8-G6g Return-To Continuation R15-Indirect RDI Load Slice
  を実装した。`49 8b 3f` を x86_64 `mov rdi, qword ptr [r15]` として decode / lift /
  stable debug report に追加し、continuation report は input の `rax` state と
  `r15` materialized state を保持する。`r15` は
  `address=4294979584`、`raw_pointer=9227875636482146304` として mapped bytes から読まれ、
  public chained fixups 上で AppKit `_NSApp` import に解決される。`rdi` materialization は
  imported global pointee load が未実装であるため
  `return_to_continuation_import_global_load_unimplemented` blocker として保存する。
  検証は `nix develop -c cargo test -p bara-isa-x86 r15 -- --nocapture`、
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`、
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-13 11:04 JST: B8-G6f Return-To Continuation R15 RIP-Relative Load Slice
  を実装した。continuation block 先頭の `4c 8b 3d b2 19 00 00` を
  `mov r15, qword ptr [rip+disp32]` として decode / lift / emit / stable debug report
  に追加し、`next_instruction.kind=mov_r15_qword_ptr_rip_relative`、
  `address=4294979584`、`processed_source_pc_range=4294972999..4294973007` を保存する。
  次 blocker は `0x49` at `4294973006` の
  `return_to_continuation_unsupported_instruction` であり、次の B8-G6g では
  `49 8b 3f` を `mov rdi, qword ptr [r15]` slice として扱う。検証は
  `nix develop -c cargo test -p bara-isa-x86 mov_r15 -- --nocapture`、
  `nix develop -c cargo test -p bara-arm64 r15_from_rip -- --nocapture`、
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`、
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-13 10:33 JST: B8-G6e Return-To Continuation Decode Boundary を実装した。
  G6d の `b8_objc_helper_return_continuation_boundary_v0` に
  `b8_return_to_continuation_decode_boundary_v0` を追加し、`next_source_pc=4294972999` から
  public Mach-O code segment bytes を使って continuation block を decode する。
  G6d の x86_64 `rax` register state は `input_register_state` として保持し、
  `processed_source_pc_range=4294972999..4294973002`、`next_instruction.kind=unsupported`、
  `unsupported_instruction.kind.reason=DecodeUnsupportedOpcode { opcode: 76, ... }` を
  stable report に保存する。helper boundary の current blocker は
  `return_to_continuation_unsupported_instruction` に進む。`return_to` block の実行、
  arbitrary indirect call target execution、translation cache、fallback JIT/interpreter は追加しない。
  targeted check と full `nix develop -c ./scripts/verify` が通過し、draft PR
  <https://github.com/serika12345/Bara/pull/42> を開いた。
- 2026-06-13 10:11 JST: B8-G6d ObjC Helper Return Continuation Boundary を実装した。
  B8 debug bundle の helper execution request に
  `b8_objc_helper_return_continuation_boundary_v0` を追加し、`call r14` の
  `call_site=4294972996` / `return_to=4294972999`、helper output
  `objc_helper_return_value` / `host_pointer_u64`、x86_64 `rax` への `written_value`、
  `register_state.register=rax`、`next_source_pc=4294972999` を stable report に保存する。
  host execution 内の `objc_helper_return_continuation_unimplemented` は履歴として残し、
  helper boundary の current blocker は
  `return_to_continuation_execution_unimplemented` に進める。`return_to` block の実行、
  arbitrary indirect call target execution、translation cache、fallback JIT/interpreter は追加しない。
  targeted check と full `nix develop -c ./scripts/verify` が通過し、draft PR
  <https://github.com/serika12345/Bara/pull/41> を開いた。
- 2026-06-13 00:22 JST: B8-G6c ObjC Runtime Helper Bridge Host Execution Slice を実装した。
  selector VM address を `ProgramImageMetadata.mapped_bytes` の NUL-terminated UTF-8 から
  `sharedApplication` として解決し、`_objc_msgSend` / `_OBJC_CLASS_$_NSApplication` /
  `sharedApplication` に限定した public Objective-C runtime / AppKit helper process を
  build/run する。helper output は `objc_helper_return_value` / `host_pointer_u64` として
  report され、既存 x86_64 `rax` return write-back boundary に `available` な
  `written_value` として接続される。arbitrary indirect call target execution、translation
  cache、fallback JIT/interpreter は追加しない。targeted checks と full
  `nix develop -c ./scripts/verify` が通過し、draft PR
  <https://github.com/serika12345/Bara/pull/40> を開いた。
- 2026-06-12 23:50 JST: B8-G6b ObjC Runtime Helper Bridge Contract を実装した。
  B8 debug bundle の helper execution request に
  `b8_objc_runtime_helper_bridge_contract_v0` を追加し、source import、receiver identity、
  selector VM address、return write-back boundary、helper output、error classification を
  stable bridge contract に分離する。Objective-C runtime / AppKit helper の host
  execution は追加しない。targeted check、manual debug bundle generation、
  full `nix develop -c ./scripts/verify` が通過し、draft PR
  <https://github.com/serika12345/Bara/pull/39> を開いた。
- 2026-06-12 23:35 JST: B8-G6a ObjC Helper Execution Boundary を実装した。
  B8 debug bundle の import helper request に
  `b8_objc_helper_execution_request_v0` を追加し、`_objc_msgSend` source import、
  receiver identity、selector VM address、return write-back boundary、required
  capability、remaining blocker を 1 つの stable report に集約する。Objective-C
  runtime / AppKit helper の host execution は追加しない。targeted check、manual debug
  bundle generation、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-12 23:04 JST: B8-G5b〜B8-G5e を 1 つの PR Gate として統合し、
  ObjC message materialization から helper return write-back boundary までを実装した。
  B8 debug bundle は `rdi` receiver と `rsi` selector の source definition、
  public file-backed mapped qword、public chained fixups による receiver import /
  selector rebase resolution、x86_64 `rax` return write-back plan を stable report に
  保存する。helper result はまだ生成せず、remaining blocker は
  `objc_helper_execution_unimplemented` である。targeted checks、manual debug bundle
  generation、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-12 20:19 JST: B8-G5e Helper Return Value Materialization を実装した。
  B8 debug bundle の `return_value` に
  `b8_objc_helper_return_writeback_boundary_v0` を追加し、ObjC helper return value を
  x86_64 `rax` に 64-bit write-back する plan、source、destination、ordering を
  stable report として保存するようにした。helper return value はまだ実行結果として
  生成せず、remaining blocker は `objc_helper_execution_unimplemented` である。
  `_objc_msgSend` 実行や Objective-C / AppKit bridge は追加していない。targeted check は
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture` が通過し、
  full `nix develop -c ./scripts/verify` も通過した。
- 2026-06-12 12:51 JST: B8-G5d ObjC Argument Fixup Resolution を実装した。
  public chained fixups decoder が bind target だけでなく rebase target も解決できるようにし、
  `MachOChainedFixupsTargetStatus::ResolvedRebase` と resolved VM address report を追加した。
  B8 debug bundle は receiver mapped raw qword を
  `_OBJC_CLASS_$_NSApplication` import identity、selector mapped raw qword を
  resolved VM address `4294975648` として保存する。`_objc_msgSend` 実行や
  Objective-C / AppKit bridge は追加していない。targeted checks は
  `nix develop -c cargo test -p bara-oracle chained_fixups -- --nocapture` と
  `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture` が通過し、
  full `nix develop -c ./scripts/verify` も通過した。
- 2026-06-12 12:20 JST: B8-G5c ObjC Materialization Mapped Image Metadata を実装した。
  `mach_o_entry_function_input` が作る `ProgramImageMetadata.mapped_bytes` を、entry
  executable segment だけでなく public `LC_SEGMENT_64` file-backed segment 全体から
  構成するようにした。B8 debug bundle は receiver qword load address `4294988120` から
  raw value `9227875636482146321`、selector qword load address `4294988072` から
  raw value `4503599627378848` を保存する。これらはまだ chained fixups / rebase / bind
  resolution 前の mapped raw qword なので、次 blocker は
  `receiver_mapped_value_fixup_resolution_unimplemented` /
  `selector_mapped_value_fixup_resolution_unimplemented` である。`_objc_msgSend` 実行や
  Objective-C / AppKit bridge は追加していない。targeted checks と full
  `nix develop -c ./scripts/verify` は通過した。
- 2026-06-12 12:02 JST: B8-G5b ObjC Message Materialization Boundary を実装した。
  B8 debug bundle の `helper_boundary_request.request.required_marshaling.contract` に
  `b8_objc_message_materialization_boundary_v0` を追加し、`rdi` receiver と `rsi`
  selector の source definition を `call r14` 直前の RIP-relative qword load として
  保存する。現 `ProgramImageMetadata.mapped_bytes` では receiver / selector qword
  value を読めないため、stable blocker は
  `receiver_mapped_image_qword_unavailable` /
  `selector_mapped_image_qword_unavailable` である。`rax` return destination は
  `write_helper_return_to_x86_64_rax` plan と
  `helper_return_value_materialization_unimplemented` blocker に留める。
  `_objc_msgSend` 実行や Objective-C / AppKit bridge は追加していない。targeted check
  として `nix develop -c cargo test -p btbc-cli generate_b8_debug_bundle -- --nocapture`
  が通過し、full `nix develop -c ./scripts/verify` も通過した。
- 2026-06-12 10:11 JST: B8-G5a Import Helper Marshaling Contract を実装した。
  B8 debug bundle の `helper_boundary_request.request.required_marshaling.contract` に
  `b8_import_helper_marshaling_contract_v0` を追加し、x86_64 macOS System V calling
  convention、`rdi` receiver、`rsi` selector、`rax` return destination を
  stable report に保存する。`_objc_msgSend` 実行や Objective-C / AppKit bridge は
  追加せず、次 blocker は `objc_receiver_materialization_unimplemented`、
  `objc_selector_materialization_unimplemented`、
  `helper_return_value_materialization_unimplemented` である。targeted check は通過し、
  full `nix develop -c ./scripts/verify` も通過した。
- 2026-06-12 08:26 JST: B8-G5 Import Helper Boundary Request を実装した。
  `bara-oracle` の chained fixups target report から resolved import identity を typed
  report として取り出せるようにし、B8 debug bundle は decoded `_objc_msgSend`
  import identity を `import_helper_call` request に接続する。`loader.plan.json` と
  launch report は `target_register=r14`、`call_site=4294972996`、
  `return_to=4294972999`、`source_isa=x86_64`、`symbol_name=_objc_msgSend`、
  `dylib_path=/usr/lib/libobjc.A.dylib` を保存し、helper execution ではなく
  `x86_64_argument_marshaling_unimplemented` /
  `helper_return_marshaling_unimplemented` で停止する。targeted checks と
  `check-domain-types`、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-12 08:02 JST: B8-G4c Public Chained Fixups Import Decoder を実装した。
  `bara-oracle` に public `LC_DYLD_CHAINED_FIXUPS` payload parser を追加し、header、
  starts-in-image / starts-in-segment、`DYLD_CHAINED_IMPORT` table、uncompressed symbol
  strings、現 fixture に必要な `DYLD_CHAINED_PTR_64_OFFSET` bind chain entry を
  typed report として decode できるようにした。B8 debug bundle の `loader.plan.json`
  は `target_pointer_load.address=4294979672` を `__DATA_CONST` chain の import
  ordinal 11 へ解決し、`/usr/lib/libobjc.A.dylib` の `_objc_msgSend` として保存する。
  helper boundary request は `import_helper_boundary_unimplemented` として停止し、
  次 action は `connect_import_helper_boundary_request` になった。targeted checks と
  manual debug bundle generation、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 22:35 JST: B8-G4b Public Chained Fixups Import Boundary を実装した。
  `loader.plan.json` は `call r14` の `target_register=r14`、`call_site=4294972996`、
  `return_to=4294972999` と、直前の R14 RIP-relative qword load が読む
  `target_pointer_load.address=4294979672` を保存する。public Mach-O metadata として
  dylib import command、dyld info range、`LC_DYLD_CHAINED_FIXUPS dataoff=24576
  datasize=584`、symbol table count を report する。現 fixture では import symbol
  identity はまだ解決せず、helper boundary request は
  `import_symbol_identity_unresolved` の stable blocker として停止し、次 action は
  `decode_public_dyld_chained_fixups_imports` になった。targeted check と manual debug
  bundle generation、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 21:54 JST: B8-G4a User-Space Mach-O VM Image Mapping を実装した。
  `MachOExecutableImagePlan` は selected segment の file range、segment `vmaddr`、
  entry segment offset、entry virtual address を分けて保持する。materialization は
  code segment base を `LC_SEGMENT_64.vmaddr`、entry PC を `vmaddr + entry_segment_offset`
  として `ExecutableImage` を作る。entry bytes の切り出し、embedded stdout metadata、
  `ProgramImageMetadata` の mapped bytes / code / const-data range、B8 debug bundle の
  source PC / call site は同じ Mach-O VM address space に揃った。`loader.plan.json` は
  image mapping を executed として保存し、public rebase / bind / import resolution は
  deferred step として残す。targeted checks、manual debug bundle generation、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 21:31 JST: B8-G3l Indirect CALL R14 Boundary を実装した。
  x86_64 `41 ff d6` (`call r14`) を `CallR14` として decode し、call 後の bytes を
  別 blocker として先読みしないようこの boundary で decode を止める。lift は
  `RegisterIndirectCallUnsupported { target: R14, call_site, return_to }` を持つ unsupported
  terminator に変換する。B8 debug bundle は lifted IR の frontier unsupported terminator を
  stable `register_indirect_call` boundary として report し、
  `call_site=5700`、`return_to=5703`、`target=r14` を保存する。arbitrary indirect target
  execution、translation cache、fallback JIT/interpreter は導入していない。targeted checks と
  manual debug bundle generation、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 21:12 JST: B8-G3k RIP-Relative MOV Load Batch Boundary を実装した。
  x86_64 `48 8b 35 eb 3a 00 00` (`mov rsi, qword ptr [rip+disp32]`) を
  `MovRsiQwordPtrRipRelative` として decode し、`MemRipRelative { width: Bits64 }`
  operand から `Rsi` へ lift する。続く `4c 8b 35 14 1a 00 00`
  (`mov r14, qword ptr [rip+disp32]`) も `MovR14QwordPtrRipRelative` として扱う。
  ARM64 emit は `ProgramImageMetadata` の mapped bytes から qword を読み、`rsi` は `x1`、
  `r14` は `x14` に immediate materialize する。どちらも `rax` destination ではないため
  `rax` value availability は維持する。debug bundle は両 load を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 65 }` (`41 ff d6`, `call r14`) を `X86Va(5700)` で保存する。
  targeted checks と manual debug bundle generation、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 20:53 JST: B8-G3j RIP-Relative MOV RDI Load Boundary を実装した。
  x86_64 `48 8b 3d 22 3b 00 00` (`mov rdi, qword ptr [rip+disp32]`) を
  `MovRdiQwordPtrRipRelative` として decode し、`MemRipRelative { width: Bits64 }`
  operand から `Rdi` へ lift する。ARM64 emit は `ProgramImageMetadata` の mapped bytes
  から qword を読み、現状の ABI-focused mapping である `x0` に immediate materialize する。
  destination は `rdi` なので `rax` value availability は無効化する。debug bundle は
  `mov_rdi_qword_ptr_rip_relative` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 72 }`
  (`48 8b 35 eb 3a 00 00`, `mov rsi, qword ptr [rip+disp32]`) を `X86Va(5686)` で保存する。
  targeted checks と manual debug bundle generation が通過し、full
  `nix develop -c ./scripts/verify` も通過した。
- 2026-06-11 20:40 JST: B8-G3i RIP-Relative LEA RSI Address Boundary を実装した。
  x86_64 `48 8d 35 b6 10 00 00` (`lea rsi, [rip+disp32]`) を
  `LeaRsiRipRelative` として decode し、memory read と区別する
  `AddressRipRelative { address }` operand から `Rsi` へ lift する。ARM64 emit は
  `rsi` を現状の ABI-focused mapping である `x1` に immediate materialize し、
  `rax` value availability は維持する。debug bundle は `lea_rsi_rip_relative` を通過し、
  次 blocker として `DecodeUnsupportedOpcode { opcode: 72 }`
  (`48 8b 3d 22 3b 00 00`, `mov rdi, qword ptr [rip+disp32]`) を `X86Va(5679)` で保存する。
  targeted checks と manual debug bundle generation が通過し、full
  `nix develop -c ./scripts/verify` も通過した。
- 2026-06-11 20:12 JST: B8-G3h RIP-Relative LEA RDI Address Boundary を実装した。
  x86_64 `48 8d 3d b3 10 00 00` (`lea rdi, [rip+disp32]`) を
  `LeaRdiRipRelative` として decode し、memory read と区別する
  `AddressRipRelative { address }` operand から `Rdi` へ lift する。ARM64 emit は
  `rdi` を現状の ABI-focused mapping である `x0` に immediate materialize し、その後は
  `rax` value を available とみなさない。debug bundle は `lea_rdi_rip_relative` を通過し、
  次 blocker として `DecodeUnsupportedOpcode { opcode: 72 }`
  (`48 8d 35 b6 10 00 00`, `lea rsi, [rip+disp32]`) を `X86Va(5667)` で保存する。
  targeted checks と manual debug bundle generation が通過し、full
  `nix develop -c ./scripts/verify` も通過した。
- 2026-06-11 19:56 JST: B8-G3g RAX-Indirect MOV RDX Load Boundary を実装した。
  x86_64 `48 8b 10` (`mov rdx, qword ptr [rax]`) を `MovRdxQwordPtrRax` として decode し、
  `MemRegIndirect { base: Rax, width: Bits64 }` から `Rdx` へ lift する。ARM64 emit は
  RAX が静的に既知で、その address が `ProgramImageMetadata` の mapped bytes にある場合
  だけ qword を `x2` immediate として materialize する。RAX が runtime value の場合や
  mapped bytes から読めない場合は typed unsupported reason を返し、loader / mapped
  runtime memory boundary を silent fallback しない。debug bundle は
  `mov_rdx_qword_ptr_rax` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 72 }` (`48 8d 3d b3 10 00 00`,
  `lea rdi, [rip+disp32]`) を `X86Va(5660)` で保存する。targeted checks と manual debug
  bundle generation が通過し、full `nix develop -c ./scripts/verify` も通過した。
- 2026-06-11 19:36 JST: B8-G3f RIP-Relative MOV Load Slice を実装した。
  x86_64 `48 8b 05 ff 19 00 00` (`mov rax, qword ptr [rip+disp32]`) を
  `MovRaxQwordPtrRipRelative` として decode し、RIP-relative target
  `X86Va(12312)` と 64-bit read width を持つ `MemRipRelative` operand へ lift する。
  Mach-O entry pipeline は materialized executable segment bytes を
  `ProgramImageMetadata` の mapped bytes として渡し、ARM64 emit はこの slice では
  raw mapped qword を `x0` immediate として materialize する。debug bundle は
  `mov_rax_qword_ptr_rip_relative` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 72 }` (`48 8b 10`, `mov rdx, qword ptr [rax]`) を
  `X86Va(5657)` で保存する。targeted checks と manual debug bundle generation が
  通過し、full `nix develop -c ./scripts/verify` も通過した。
- 2026-06-11 19:01 JST: B8-G3e Opcode-Only Blocker Batch を実装した。
  `53` (`push rbx`) を `PushRbx` として decode し、`IrOp::Push { src: Rbx }` へ
  lift し、ARM64 emit では `str x19, [sp, #-16]!` を生成する。続けて
  `48 89 c3` (`mov rbx,rax`) を `MovRbxRax` として decode し、
  `IrOp::Mov { dst: Rbx, src: Rax }` へ lift し、ARM64 emit では `mov x19,x0` を
  生成する。debug bundle は `push_rbx` と `mov_rbx_rax` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 72 }` (`48 8b 05 disp32`, RIP-relative load) を
  `blocker.json` と `launch.report.json` に保存する。targeted checks と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 18:46 JST: B8-G3d REX Push R14 Prologue Slice を実装した。
  x86_64 `41 56` (`push r14`) を `PushR14` として decode し、
  `IrOp::Push { src: R14 }` へ lift し、ARM64 emit では `str x14, [sp, #-16]!` を
  生成する。debug bundle は `push_r14` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 83 }` (`53`, `push rbx`) を
  `blocker.json` と `launch.report.json` に保存する。targeted
  `nix develop -c cargo test push_r14 -- --nocapture` と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 18:36 JST: B8-G3c REX Push R15 Prologue Slice を実装した。
  x86_64 `41 57` (`push r15`) を `PushR15` として decode し、
  `IrOp::Push { src: R15 }` へ lift し、ARM64 emit では `str x15, [sp, #-16]!` を
  生成する。debug bundle は `push_r15` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 65 }` (`41 56`, `push r14`) を
  `blocker.json` と `launch.report.json` に保存する。targeted
  `nix develop -c cargo test push_r15 -- --nocapture` と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 18:23 JST: B8-G3b REX Mov RBP/RSP Prologue Slice を実装した。
  x86_64 `48 89 e5` (`mov rbp,rsp`) を `MovRbpRsp` として decode し、
  `IrOp::Mov { dst: Rbp, src: Rsp }` へ lift し、ARM64 emit では `mov x29, sp` を
  生成する。debug bundle は `mov_rbp_rsp` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 65 }` (`41 57`, `push r15`) を
  `blocker.json` と `launch.report.json` に保存する。targeted
  `nix develop -c cargo test mov_rbp_rsp -- --nocapture` と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 18:00 JST: B8-G3 First ISA Blocker Slice を実装した。
  x86_64 `push rbp` (`0x55`) を `PushRbp` として decode し、`IrOp::Push`
  の `Rbp` operand へ lift し、ARM64 emit では `str x29, [sp, #-16]!` を生成する。
  debug bundle は `push_rbp` を通過し、次 blocker として
  `DecodeUnsupportedOpcode { opcode: 72 }` (`48 89 e5`, `mov rbp,rsp`) を
  `blocker.json` と `launch.report.json` に保存する。targeted
  `nix develop -c cargo test push_rbp -- --nocapture` と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 17:37 JST: B8-G2 Real LC_MAIN First-Block Report を実装した。
  `generate-b8-debug-bundle <binary> <out-root>` は B8-G1 専用 translated host-trap
  sentinel ではなく、入力 Mach-O の public `LC_MAIN` entry bytes を保存し、その実
  entry に対する decode / lift / emit / runtime attempt、loader plan、
  `launch.report.json`、`blocker.json`、repro command を出力する。debug bundle の
  current blocker は `unsupported_instruction` /
  `DecodeUnsupportedOpcode { opcode: 85 }` で、B8-G3 の最初の対象は x86_64
  `push rbp` prologue slice になった。targeted `cargo check` と
  `generate_b8_debug_bundle_writes_real_entry_first_block_report` test、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 17:18 JST: B8-D0 Debug Bundle Foundation を実装した。
  `generate-b8-debug-bundle <binary> <out-root>` は
  `<out-root>/b8_gui_hello_world/` に `input.probe.json`、`entry.bytes.bin`、
  `entry.bytes.json`、`decode.report.json`、`lift.ir.json`、`emit.report.json`、
  `pcmap.json`、`fixups.json`、`helpers.json`、`loader.plan.json`、
  `runtime-attempt.json`、`blocker.json`、`repro.sh` を保存する。D0 では実
  `LC_MAIN` first-block translation は行わず、B8-G1 translated host trap entry を
  debug bundle foundation として保存し、B8-G2 の blocker を
  `real_lc_main_entry_not_attempted` として残す。targeted checks と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 16:57 JST: B8 以降のアプリ起動フェーズで、計画時点から PR 提出地点を
  明確にするため、`TODO.md` に B8-D0 / B8-G2 / B8-G3 の `PR Gate` を追加した。
  `AGENTS.md` と `README.md` には `/advance-pr` を追加し、repo-scoped skill
  `$bara-advance-pr` からも次の未完了 PR Gate まで進められるようにした。
  documentation-only policy / roadmap update のため code verification は未実行で、
  `git diff --check` と `nix develop -c ./scripts/check-no-invisible-chars` が通過した。
- 2026-06-11 16:42 JST: B8-D0 以降でぶつかりそうな大きな壁を、想定順で
  TODO / scope / design TODO に記録した。現状は AOT 的 pipeline を主軸にし、
  JIT / on-demand translation は unknown indirect target、callback、lazy binding、
  runtime-generated target が stable blocker として頻出し始めた段階で、
  translation cache、PC map、runtime helper boundary とセットで導入する判断を
  明文化した。documentation-only update のため code verification は未実行で、
  `git diff --check` と `nix develop -c ./scripts/check-no-invisible-chars` が通過した。
- 2026-06-11 16:34 JST: B8-G2 の前提として B8-D0 debug bundle foundation を追加した。
  一般アプリ化では unsupported boundary を細かく潰す必要があるため、input probe、
  entry extraction、decode / lift / emit / runtime attempt、loader plan、helper
  request、blocker、repro command を 1 directory に保存する。debug bundle は
  failure analysis 用 sidecar であり、通常の actual / launch / feedback report の
  代替ではない。documentation-only update のため code verification は未実行で、
  `git diff --check` と `nix develop -c ./scripts/check-no-invisible-chars` が通過した。
- 2026-06-11 16:27 JST: B8-G1 後の一般アプリ化ゴールと B8-G2 以降の線形計画を
  `TODO.md`、`docs/b8-gui-hello-world-scope.md`、`docs/design-todo.md` に明文化した。
  当面の次 step は、専用 sentinel ではなく実 Mach-O `LC_MAIN` entry から
  first-block translation attempt を行い、最初の unsupported boundary を stable
  report に残すこと。documentation-only update のため code verification は未実行で、
  `git diff --check` と `nix develop -c ./scripts/check-no-invisible-chars` が通過した。
- 2026-06-11 16:18 JST: B8-G1 を完了した。self-authored x86_64 GUI binary の
  Rosetta manual visible check はユーザー確認済み。Bara 側には
  `appkit_gui_hello_world` host trap contract を追加し、専用 x86_64 entry
  `0f0b4238473131c0c3` を decode / lift / emit / runtime execution に通したうえで
  AppKit lifecycle helper capability を呼ぶ。CLI に automated stable comparison 用
  `generate-arm64-gui-hello-world-translated-actual` と、GUI 目視確認用
  `run-arm64-gui-hello-world-translated-visible` を追加した。targeted tests と
  automated translated actual / comparison、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 15:53 JST: B8-G1 の最初の実装 step として、GUI Hello World fixture に
  manual-visible run mode を追加した。automated oracle 用 binary は従来どおり
  short-lived deterministic stdout を維持し、manual-visible binary は public AppKit
  API で window と `hello world` label を描画したまま、window close / `Command-Q`
  まで event loop を維持する。CLI に
  `build-x86_64-gui-hello-world-visible-fixture` を追加し、
  `target/b8/b8_gui_hello_world_visible_x86_64` を生成した。Rosetta 手動確認は
  ユーザー確認待ち。targeted tests、generated binary checks、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 15:42 JST: B8 milestone の切り方を再定義した。B8-H1 の
  helper capability execution は reviewable intermediate slice として残し、
  B8 全体の完了扱いから外した。次の active target は B8-G1:
  x86_64 entry path が実際に Bara の変換レイヤーを通り、その結果として
  AppKit lifecycle helper capability を呼び、GUI window 上の `hello world`
  フォント描画を developer-visible mode で確認できる状態にすること。
  documentation-only recut のため、`git diff --check` と
  `nix develop -c ./scripts/check-no-invisible-chars` が通過した。
- 2026-06-11 15:29 JST: B8-H1 helper execution slice として、Bara actual path に
  host AppKit lifecycle helper execution を接続した。input x86_64 GUI Mach-O は
  public probe され、self-authored AppKit source を host helper として build/run し、
  stdout lifecycle event を actual observation にした。B8 actual は Rosetta expected
  と一致し、feedback report は `matched`、comparison issues 空、current blocker
  `none`、next action `review_b8_milestone` になった。この時点では B8 全体の
  完了扱いとして記録したが、15:42 JST の milestone 再定義により B8-H1 の
  reviewable intermediate slice として扱い直した。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 15:15 JST: B8 の helper capability contract step として、
  `UserSpaceHelperCapabilityPlan` を追加し、Objective-C runtime / AppKit lifecycle
  helper capability を actual launch report と feedback report に接続した。
  `runtime_preparation.helper_capability` と `helper_capability_plan` は
  `appkit_gui_lifecycle_event`、`planned` bridge / lifecycle event、
  `stdout_lifecycle_event`、`planned_not_executed` を保存する。current blocker は
  `unsupported_objc_runtime_boundary` のままで、next action は
  `connect_appkit_lifecycle_helper_execution` に進んだ。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 15:04 JST: B8 の Objective-C runtime helper boundary step として、
  `helper_boundary_plan.next_blocker` を `unsupported_objc_runtime_boundary` に進め、
  actual result / launch report / feedback report に接続した。B8 actual の
  `stderr` は `unsupported_boundary: unsupported_objc_runtime_boundary` になり、
  current blocker は Objective-C runtime helper boundary、candidate boundary は
  Objective-C runtime のみに絞られた。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 14:54 JST: B8 の AppKit import helper boundary step として、
  `helper_boundary_plan.next_blocker` の `unsupported_import` を actual result /
  launch report / feedback report に接続した。B8 actual の `stderr` は
  `unsupported_boundary: unsupported_import` になり、current blocker は public
  AppKit import boundary、candidate boundary は import と Objective-C runtime に
  絞られた。targeted tests、`btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 14:47 JST: B8 の AppKit import / Objective-C runtime boundary
  の最初の小ステップとして、`UserSpaceHelperBoundaryPlan` を詳細化した。
  public AppKit framework import、import resolution、Objective-C runtime、
  OS API request は helper capability required として report され、次 blocker は
  `unsupported_import` として保存される。B8 feedback report は
  `helper_boundary_plan` を含み、next action は
  `connect_appkit_import_objc_runtime_helper_boundary` になった。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 14:37 JST: B8 の `unsupported_loader_feature` に対する
  最初の修正フィードバック対象として、public Mach-O metadata 由来の
  user-space loader 実行計画を model 化した。`UserSpaceLaunchPlan` は
  `loader_execution` に `public_mach_o_probe`、`lc_main_entryoff`、
  `lc_segment_64_file_ranges`、`dylib_load_commands_to_helper_boundary`、
  `linkedit_rebase_bind_metadata`、`helper_boundary`、`planned_not_executed` を
  typed plan として保持し、B8 actual launch report と feedback report は
  `runtime_preparation.loader_execution` / `loader_execution_plan` に同じ plan を
  保存する。targeted tests、`bara-runtime` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 14:25 JST: B8 の Rosetta 比較フィードバックサイクル開始点として、
  `btbc-cli generate-arm64-gui-hello-world-feedback` を追加した。Rosetta expected
  JSON と Bara actual / launch report から
  `b8_gui_hello_world_feedback_report_v0` を生成し、observed result の
  `exit_status` / `stdout` / `stderr` mismatch、current blocker
  `unsupported_loader_feature`、next action
  `implement_user_space_loader_for_mach_o_gui_executable` を stable JSON に保存する。
  targeted tests、`target/b8` での手動 CLI 確認、`btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 14:05 JST: B8 の Rosetta 比較フィードバックサイクル直前の
  boundary 固定として、`bara-runtime::UserSpaceLaunchPlan` に
  `platform_model`、`macos_constraints`、`fallback_policy` を追加した。
  B8 actual launch report は signal / exception / thread / TLS / memory
  protection、macOS code signing / W^X / hardened runtime 制約、fallback
  方針、top-level `launch_result` を stable JSON に保存する。interpreter
  fallback と外部 fallback engine は候補だが未実装 / 未接続、feedback cycle は
  ready not started として記録される。targeted tests、`bara-runtime` /
  `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:54 JST: B8 の register model guardrail step として、
  `bara-ir::X86Reg` に `rax` / `eax` / `ax` / `al` と `rdi` / `edi` / `di` /
  `dil` を追加し、register family、view width、full-width register、
  partial view 判定を domain model として公開した。`btbc-cli` の function
  artifact projection も partial register view を stable JSON 名へ変換できる。
  targeted tests、`bara-ir` / `bara-arm64` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:43 JST: B8 の source ISA profile step として、
  `bara-runtime::UserSpaceLaunchPlan` に `source_isa_profile` を追加した。
  現在は x86_64 long mode、address size 64-bit、default operand size 32-bit、
  stack width 64-bit を typed profile として保持し、B8 actual launch report の
  `runtime_preparation.source_isa_profile` に保存する。profile model は
  x86_32 protected mode も表現できる。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 13:34 JST: B8 の syscall / OS API bridge boundary step として、
  `bara-runtime::UserSpaceLaunchPlan` に `bridge_boundary` を追加した。
  syscall bridge と OS API bridge は helper boundary の責務として B8 actual
  launch report に保存され、core IR / ARM64 emit の bridge 実装は
  `not_embedded` として保存される。targeted tests、`bara-runtime` / `btbc-cli`
  clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:25 JST: B8 の execution strategy selection boundary step
  として、`bara-runtime::UserSpaceLaunchPlan` に `execution_strategy` を追加した。
  JIT、AOT、fallback interpreter は同じ `user_space_runtime` boundary から
  `selectable` として B8 actual launch report に保存される。targeted tests、
  `bara-runtime` / `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が
  通過した。
- 2026-06-11 13:15 JST: B8 の executable memory public OS API boundary step
  として、`bara-runtime::UserSpaceLaunchPlan` に `executable_memory` を追加した。
  allocation は `mmap_private_anonymous`、protection transition は
  `mprotect_read_write_to_read_execute`、release は `munmap` として B8 actual
  launch report に保存される。targeted tests、`bara-runtime` / `btbc-cli`
  clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:07 JST: B8 の user-space process boundary step として、
  `bara-runtime::UserSpaceLaunchPlan` に `process_boundary` を追加した。
  loader、translation cache、runtime helper、artifact cache は current
  user-space process 内の責務として B8 actual launch report に保存される。
  targeted tests、`bara-runtime` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 13:01 JST: B8 の private kernel / dyld assumption exclusion
  step として、`bara-runtime::UserSpaceLaunchPlan` に `integration_policy` を
  追加した。B8 actual launch report は current user-space process を scope とし、
  kernel extension、private kernel hook、private dyld behavior をすべて
  `not_required` として保存する。targeted tests、`bara-runtime` / `btbc-cli`
  clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:50 JST: B8 の user-space loader/runtime preparation step として、
  `bara-runtime::UserSpaceLaunchPlan` を追加し、image mapping、entry trampoline、
  initial stack、helper boundary の準備責務を分けた。B8 actual launch report は
  `runtime_preparation` に `planned_not_executed` の plan projection を保存する。
  targeted tests、`bara-runtime` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:39 JST: B8 の loader metadata model 化ステップとして、
  entry、segments、sections、imports、relocations、rebases、binds に必要な
  public Mach-O metadata を parser/report 境界へ載せた。`LC_SYMTAB`、
  `LC_DYSYMTAB`、`LC_DYLD_INFO(_ONLY)`、`LC_DYLD_CHAINED_FIXUPS` は symbol table、
  dynamic symbol table、dyld rebase / bind blob、chained fixups metadata を typed
  summary として保持する。targeted tests、`bara-oracle` / `btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:23 JST: B8 の 9 つ目の小ステップとして、public dylib load
  commands から Mach-O imports metadata を model 化した。`LC_LOAD_DYLIB` 系 command
  は dependent dylib path、timestamp、current version、compatibility version を
  typed metadata として保持する。B8 actual launch report の `imports` status は
  `modeled_from_dylib_load_commands` になった。targeted tests、`bara-oracle` /
  `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 12:13 JST: B8 の 8 つ目の小ステップとして、public
  `LC_SEGMENT_64` section table から Mach-O sections metadata を model 化した。
  `section_64` の section name、segment name、addr、size、offset、align、
  reloff、nreloc、flags を typed metadata として parser/report 境界に載せる。
  B8 actual launch report の `sections` status は
  `modeled_from_lc_segment_64_section_table` になった。targeted tests、`bara-oracle` /
  `btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:55 JST: B8 の 7 つ目の小ステップとして、Bara 側の
  GUI Hello World actual launch report に public Mach-O probe 由来の loader
  metadata summary を保存するようにした。`input.loader_metadata` は
  `public_mach_o_probe` を source とし、file type、load command table、
  recognized entry points / segments、executable image conversion blocker を保持する。
  sections、imports、relocations は parser 未対応のため `not_modeled` として
  明示した。targeted tests、`btbc-cli` clippy、full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:43 JST: B8 の 6 つ目の小ステップとして、Bara 側の
  GUI Hello World actual launch attempt が raw function fixture ではなく
  x86_64 Mach-O executable image 全体を入力として受け取るようにした。
  `btbc-cli generate-arm64-gui-hello-world-actual` は `<binary> <actual.json>
  <launch-report.json>` を受け取り、input binary 全体を `BinaryInput` として
  public Mach-O probe に通す。launch report の `input.kind` は
  `mach_o_executable_image` になり、probe の format / status summary を保存する。
  targeted tests、`btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:28 JST: B8 の 5 つ目の小ステップとして、GUI Hello World の
  initial blocker を stable な launch boundary 分類として固定した。
  `b8_gui_hello_world_actual_launch_report_v0` の `blocker` は
  `boundary`、`selected_by`、`candidate_boundaries` を持ち、分類候補を
  `unsupported_loader_feature`、`unsupported_import`、
  `unsupported_objc_runtime_boundary` に限定する。選択規則は
  `first_unsupported_launch_boundary` で、現時点では loader が最初の未対応境界
  であるため `unsupported_loader_feature` を initial blocker とする。targeted
  tests、`btbc-cli` clippy、full `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:17 JST: B8 の 4 つ目の小ステップとして、Bara 側の
  GUI Hello World 起動 attempt を `actual.json` と launch report sidecar へ保存
  する CLI 境界を追加した。`tests/expected/b8_gui_hello_world.bara.actual.json`
  は現在の blocked process observation を保存し、
  `tests/expected/b8_gui_hello_world.bara.launch-report.json` は
  `b8_gui_hello_world_actual_launch_report_v0` として Bara runtime、input identity、
  `unsupported_loader_feature` blocker を保存する。targeted tests と full
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-11 11:03 JST: B8 の 3 つ目の小ステップとして、GUI Hello World
  fixture を Rosetta black-box oracle で実行し、`expected.json` と launch
  metadata の初期 schema を固定した。`tests/expected/b8_gui_hello_world.json`
  は stdout 上の deterministic `gui_window_created` event、exit status 0、
  stderr 空を保存する。`tests/expected/b8_gui_hello_world.launch.metadata.json`
  は `b8_gui_hello_world_launch_metadata_v0` として oracle identity、fixture
  identity、observed lifecycle event を保存する。検証は snapshot の targeted
  tests と commit 前の full verification。

- 2026-06-11 10:48 JST: B8 の 2 つ目の小ステップとして、self-authored
  single-binary GUI Hello World source を追加し、x86_64 Mach-O executable
  としてビルドできる host-specific fixture にした。AppKit source は
  GUI window creation 後に deterministic lifecycle event を stdout へ出し、
  短時間後に終了する。`btbc-cli build-x86_64-gui-hello-world-fixture` は
  `gui_hello_world_mach_o_executable` metadata を返し、public Mach-O probe で
  x86_64 Mach-O として認識できる。検証は snapshot の targeted tests と
  commit 前の full verification。

- 2026-06-11 10:38 JST: B8 の最初の小ステップとして、実 x86_64 macOS
  アプリ起動の初期ターゲットを self-authored single-binary GUI Hello World
  に固定した。`.app` bundle や private dyld integration を初期対象から外し、
  public system framework imports は loader/runtime/helper boundary で扱う。
  成功条件は stdout、stderr、exit status、return value または process-level
  equivalent、launch metadata、blocker classification を含む stable JSON report
  の Rosetta expected / Bara actual 比較とした。検証は snapshot の docs-only
  checks。

- 2026-06-11 10:22 JST: B7 の 19 個目の小ステップとして、IR invariant を
  Rust verifier report に接続した。`validate_program` の validation issue は
  `EmittedFunctionVerificationIssue::IrInvariant` として report に入り、CLI artifact
  では stable `ir_*` issue に変換される。これで B7 の implementation TODO は完了し、
  次は large milestone review gate として PR を開く。検証は snapshot の targeted
  test と commit 前の full verification。

- 2026-06-11 10:10 JST: B7 の 18 個目の小ステップとして、
  stable failure classification kind を追加し、final-state mismatch から具体分類へ
  接続した。`return_value_mismatch` は `WrongRegisterValue`、`stdout_mismatch` は
  `WrongExternalCall`、`exit_status_mismatch` は `WrongCallReturn` になる。
  `UnsupportedReason::DecodeUnsupportedOpcode` などの未対応命令系 emit error は
  `UnsupportedInstruction` に分類する。検証は snapshot の targeted test と
  commit 前の full verification。

- 2026-06-11 10:10 JST: B7 の 17 個目の小ステップとして、
  verification lane scripts を分離した。`verify-quick` は format / security /
  domain / check / clippy / library unit tests、`verify-native` は workspace tests、
  `verify-oracle` は blackbox oracle、`verify-nightly` は small-case shrink tests と
  nightly output directory への failure package 保存を担当する。検証は snapshot の
  lane scripts と commit 前の full verification。

- 2026-06-11 10:04 JST: B7 の 16 個目の小ステップとして、
  Rust deterministic 小ケース生成と shrink candidate plan を追加した。
  `bara_oracle::small_case` は no-args/u64 の小ケース集合と expected final state、
  非ゼロ immediate return の `return 0` shrink 候補を pure に返す。検証は
  snapshot の targeted test と commit 前の full verification。

- 2026-06-11 09:59 JST: B7 の 15 個目の小ステップとして、
  expected / actual final state comparator report を failure package に接続した。
  comparison mismatch 時の `failure.json` は `final_state` field に
  `ComparisonReport` を保存する。検証は snapshot の targeted test と commit 前の
  full verification。

- 2026-06-11 09:52 JST: B7 の 14 個目の小ステップとして、
  Rust verifier report が branch fixup consistency を検査できるようにした。
  fixup target は PC map source に解決できる必要があり、fixup offset / source は
  生成 code 内の 4-byte instruction slot を指す必要がある。検証は snapshot の
  targeted tests と commit 前の full verification。

- 2026-06-11 09:45 JST: B7 の 13 個目の小ステップとして、
  Rust verifier report を追加し、PC map が全 IR block start の source PC を
  保持していることを検査できるようにした。`bara-arm64::verify` は I/O を持たない
  pure report を返し、`emit-fixture-artifacts` / `check-corpus --out` /
  `check-blackbox --out` は `verifier.report.json` を保存する。検証は snapshot の
  targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-11 09:32 JST: B7 の 12 個目の小ステップとして、
  Haskell verifier package / schema reader / small x86 semantics interpreter の
  導入可否を判断した。B7 では Haskell を追加せず、既存 Rust workspace 内で
  verifier report を先に整える。Haskell は schema が安定し、QuickCheck /
  Hedgehog generator / shrinker または独立仕様モデルが必要になった時点で
  `spec/` と Nix toolchain を同じ change で追加する。検証は snapshot の
  documentation-only checks と最終 `nix develop -c ./scripts/verify`。

- 2026-06-11 09:27 JST: B7 の 11 個目の小ステップとして、
  fixture shrink / failure classification / corpus update の初期運用 package を
  追加した。`check-corpus --out` / `check-blackbox --out` は失敗 fixture ごとに
  `failures/<case_id>/failure.json` を保存し、raw testcase の comparison mismatch
  では `testcase.json`、`expected.json`、`actual.json` も保存する。
  `failure.json` には failure kind、message、shrink `not_attempted`、corpus update
  action を含める。検証は snapshot の targeted test と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-11 09:18 JST: B7 の 10 個目の小ステップとして、
  Rosetta black-box oracle 経路を clean-room ルール内で再検討した。
  `x86_64_mach_o_fixture` は `RosettaOracleObservation` を介して runner
  subprocess の status / stdout / stderr だけを扱い、`expected.json` の
  testcase behavior は runner stdout の `ObservedResult` JSON だけから作る。
  `docs/clean-room.md` と `docs/test-oracle.md` に同じ境界を記録した。
  検証は snapshot の targeted test と最終 `nix develop -c ./scripts/verify`。

- 2026-06-11 09:09 JST: B7 の 9 つ目の小ステップとして、
  `check-corpus --out` / `check-blackbox --out` が raw testcase fixture の
  compile artifact metadata を `compiled/<case_id>/` に保存するようにした。
  `actual/<case_id>.json` は外部観測結果の stdout、stderr、exit status、
  return value を保持し、artifact metadata は sidecar として同じ regression
  output bundle に含める。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 23:01 JST: B7 の 8 つ目の小ステップとして、
  generated executable smoke を `ObservedResult` regression gate に昇格した。
  `check-blackbox --out` は `return_42_native_executable_smoke` と
  `mach_o_return_42_native_executable_smoke` の process execution result を
  `actual/*.json` に保存する。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 22:51 JST: B7 の 7 つ目の小ステップとして、
  `emit-fixture-artifacts` が `artifact.report.json` を保存するようにした。
  report は function-level v0 state layout、fixture function v0 cache validation
  identity、helper requirements を含む。stdout host trap fixture では
  `write_stdout(ptr_len_to_unit)` requirement が記録される。検証は snapshot の
  targeted test と最終 `nix develop -c ./scripts/verify`。

- 2026-06-10 22:35 JST: B7 の 6 つ目の小ステップとして、
  `emit-fixture-artifacts` CLI を追加した。testcase を Bara の decode / lift /
  ARM64 emit pipeline に通し、`compiled.ir.json`、`pcmap.json`、`fixups.json`、
  `helpers.json` を指定 directory に保存する。ARM64 emitter は branch lowering で
  適用した fixup の offset / source / target / kind を `EmittedFunction` に保持し、
  CLI 側の stable JSON DTO へ写す。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 21:36 JST: B7 の 5 つ目の小ステップとして、
  `compare-expected-actual` CLI を追加した。保存済みの `expected.json` と
  `actual.json` を `ObservedResult` として parse し、M1 の比較対象フィールドを
  `ComparisonReport` で比較する。一致時は空 issue report を stdout に出し、
  不一致時は `ComparisonMismatch` として非ゼロ終了する。検証は snapshot の
  targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-10 21:19 JST: B7 の 4 つ目の小ステップとして、
  `generate-arm64-actual` CLI を追加した。testcase を Bara の decode / lift /
  ARM64 emit / native runner pipeline に通し、`ObservedResult` JSON を
  `actual.json` として保存する。比較は次ステップに残した。検証は snapshot の
  targeted test と最終 `nix develop -c ./scripts/verify`。

- 2026-06-10 21:00 JST: B7 の 3 つ目の小ステップとして、
  `generate-x86_64-expected` CLI を追加した。一時 x86_64 oracle runner を
  build して Rosetta 上で実行し、stdout の `ObservedResult` JSON を
  `expected.json` として保存する。Rosetta host 非対応時は `RunError` として
  分類する。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-10 20:40 JST: B7 の 2 つ目の小ステップとして、
  `build-x86_64-oracle-runner` CLI を追加した。runner は testcase bytes を
  executable memory に配置して no-args / `u64` function として呼び出し、
  `ObservedResult` 互換 JSON を stdout に出す x86_64 Mach-O executable として
  build される。Rosetta 実行と `expected.json` 保存は次ステップに残した。検証は
  snapshot の targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 22:22 JST: B7 の先頭小ステップとして、
  `build-x86_64-macho-fixture` CLI を追加した。`return_42` testcase は
  x86_64 Mach-O `_main` として assemble / link され、生成 binary の Mach-O
  magic と public header 上の x86_64 cputype を regression で確認する。引数 ABI
  と host trap fixture は後続 runner harness へ分離し、現時点では classified
  unsupported とした。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-09 21:55 JST: B6 の最後の小ステップとして、pure writer の
  serialized output Mach-O を既存の public Mach-O probe に通す regression を
  追加した。`mach_o_hello_world_stdout.bin` 由来の compile 経路で作った writer
  output が、writer layout の entry offset、load command size、segment file
  size と一致する `LC_MAIN` / `LC_SEGMENT_64` metadata として probe される。
  これにより B6 の実装 TODO は完了し、次は large milestone review gate として
  branch の PR を開く。検証は snapshot の targeted test と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-09 21:42 JST: B6 の 9 つ目の小ステップとして、`bara-mach-o`
  の pure writer に offset / size / byte serialization 境界を追加した。
  writer は minimal ARM64 Mach-O の header、`LC_SEGMENT_64`、section table、
  `LC_MAIN`、text / const payload bytes を型付き layout と serialized bytes
  として返す。`btbc-cli` の実 Mach-O stdout fixture 入力経路から compile した
  ARM64 main bytes と binary metadata 由来 stdout const bytes が、この writer
  serialization plan の text / const range に配置されることを検証した。検証は
  snapshot の targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 21:28 JST: B6 の 8 つ目の小ステップとして、Mach-O entry
  pipeline の Program image metadata を entry-aware にした。code section は
  selected segment 全体ではなく entry offset 以降の range として保持する。
  Embedded stdout metadata がある場合は、entry 前の self-authored
  `BARA_STDOUT\0` payload を `ConstData` section として保持し、同じ binary
  metadata から stdout host trap request を作る。検証は snapshot の targeted
  tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 21:18 JST: B6 の 7 つ目の小ステップとして、
  `ProgramImageMetadata` を `bara-ir` に追加した。metadata は code sections、
  symbols、relocations、imports、unwind entries を typed collection として
  持つ。`Program::new` は空 metadata の互換 API として残し、
  `Program::with_image_metadata` と
  `lift_decoded_function_with_image_metadata` が metadata 付き Program を作る。
  Mach-O entry pipeline は selected code segment range を code section として
  `MachOEntryFunctionInput` に添付し、Mach-O native artifact compile 経路は
  その metadata を IR へ渡す。検証は snapshot の targeted tests と最終
  `nix develop -c ./scripts/verify`。

- 2026-06-09 20:46 JST: B6 の 6 つ目の小ステップとして、
  `MachOEntryFunctionInput` を追加し、Mach-O executable image 全体と
  entry-derived `TestCase` を同じ pipeline 出力として保持するようにした。
  既存の `mach_o_entry_function_test_case*` API は互換 wrapper として残し、
  `btbc-cli` の Mach-O native artifact link 経路は `TestCase` 単体ではなく
  `MachOEntryFunctionInput` を受ける。回帰テストでは entry bytes だけでなく、
  selected code segment 全体と entry offset が保持されることを確認した。検証は
  snapshot の targeted tests と最終 `nix develop -c ./scripts/verify`。

- 2026-06-09 20:08 JST: B6 の 5 つ目の小ステップとして、malformed /
  unsupported Mach-O の artifact 生成時 blocker classification を回帰テストで
  固定した。`link-mach-o-arm64-main` と `link-mach-o-arm64-stdout-main` は
  short Mach-O input を `MachOEntryFunctionTestCaseError::Probe(InputTooShort)`
  として、entry point はあるが segment がない input を
  `MachOEntryFunctionTestCaseError::Plan(NotConvertible { blocker:
  MissingSegment })` として返し、native artifact output を作らない。検証は
  targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 22:28 JST: B6 の 4 つ目の小ステップとして、fixture 専用
  host trap JSON への依存を減らした。`mach_o_hello_world_stdout.bin` は
  selected segment の entry 前に self-authored `BARA_STDOUT\0` payload を
  持ち、`mach_o_entry_function_test_case_with_embedded_host_traps` はその
  payload から `TestCaseHostTrapPlan::stdout` を作る。`check-blackbox` と
  native stdout artifact のデフォルト経路は host-traps JSON を読まず、
  明示 JSON 経路は互換テストとして残す。検証は snapshot の targeted tests
  と `nix develop -c ./scripts/verify`。

- 2026-06-08 21:47 JST: B6 の 3 つ目の小ステップとして、input Mach-O の
  entry / segment / stack metadata を native output packaging に渡す境界を
  追加した。`NativeArtifactMetadata` は raw fixture では既存 JSON を維持し、
  Mach-O artifact 経路では optional `source_image` として `entryoff`、
  `stacksize`、selected segment の `name` / `vmaddr` / `fileoff` / `filesize`
  を保持する。Mach-O artifact CLI は既存 entry function testcase 変換を先に
  通すため、malformed / unsupported Mach-O の既存分類を優先する。検証は
  snapshot の targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 21:33 JST: B6 の 2 つ目の小ステップとして、Mach-O backed
  `hello world` 入力を native executable artifact へ変換する CLI /
  blackbox 経路を追加した。`link-mach-o-arm64-stdout-main` は Mach-O 入力と
  host trap plan を既存の `mach_o_entry_function_test_case_with_host_traps`
  経由で `TestCase` に変換し、stdout helper-aware compile と
  `link_arm64_stdout_main_executable` に委譲する。
  `mach_o_hello_world_stdout_native_executable` を blackbox report に追加し、
  生成 artifact を実行して stdout `hello world\n` と exit status 0 を確認する。
  検証は snapshot の targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 21:12 JST: B6 の先頭小ステップとして、Mach-O backed
  `return_42` 入力を native executable artifact へ変換する CLI / blackbox
  経路を追加した。`link-mach-o-arm64-main` は Mach-O 入力を既存の
  `mach_o_entry_function_test_case` 経由で `TestCase` に変換し、
  standalone artifact compile と `link_arm64_main_executable` に委譲する。
  `mach_o_return_42_native_executable_smoke` を blackbox report に追加し、
  生成 artifact を実行して exit status 42 を確認する。検証は snapshot の
  targeted tests と `nix develop -c ./scripts/verify`。

- 2026-06-08 20:18 JST: B5 large milestone completion の最終小ステップとして、
  IR validation に missing branch/fallthrough/call target report を追加し、
  decoder / lifter は short / near `jcc` 全条件を `CondJump` へ接続した。
  ARM64 emitter は parity 以外の条件を `b.cond` へ lower し、parity 条件を
  explicit unsupported として維持する。`jl_rel32_return_42` を repository
  fixture に追加し、signed-less rel32 branch を decode / lift / emit / runtime
  regression にした。検証は snapshot の targeted tests と
  `nix develop -c ./scripts/verify`。

- 2026-06-08 20:05 JST: B5 large milestone completion の小ステップとして、
  `Push` / `Pop` IR、internal-target `DirectCall` terminator、ARM64 の
  16-byte aligned stack slot lowering、direct call `bl` fixup、link-register
  save/restore、block 間 `rax` live-in propagation を追加した。
  `push_pop_return_42`、`loop_countdown_return_0`、`nested_call_return_42` を
  repository fixture に追加し、nested call は linked native executable artifact
  としても実行した。検証は snapshot の targeted tests と
  `nix develop -c ./scripts/verify`。

- 2026-06-08 19:53 JST: B5 large milestone completion の小ステップとして、
  short `jmp rel8` を `DecodedInstructionKind::JmpRel8` と
  `Terminator::DirectJump` へ接続した。`direct_jmp_return_42` を repository
  fixture に追加し、decode / lift / emit と native runtime 実行の regression
  とした。検証は `nix develop -c cargo test -p bara-isa-x86
  decodes_jmp_rel8_and_continues_with_target_block`、`nix develop -c cargo test
  -p bara-isa-x86 lifts_jmp_rel8_to_direct_jump_terminator`、`nix develop -c
  cargo test -p bara-runtime direct_jmp_return_42`、`nix develop -c cargo test
  -p btbc-cli check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures`、
  および `nix develop -c ./scripts/verify`。

- 2026-06-08 19:45 JST: B5 large milestone completion の小ステップとして、
  ARM64 emitter に `cmp x0,#imm12`、`tst x0,x0`、`b.eq` / `b.ne`、
  unconditional `b` の branch fixup を追加した。`branch_eq_return_42` を
  repository fixture に追加し、decode / lift / emit と native runtime 実行の
  regression とした。検証は `nix develop -c cargo test -p bara-arm64
  emits_conditional_branch_fixups_for_equal`、`nix develop -c cargo test -p
  bara-arm64 emits_cmp_x0_immediate_for_rax_compare_immediate`、`nix develop -c
  cargo test -p bara-arm64 emits_tst_x0_x0_for_rax_test_rax`、
  `nix develop -c cargo test -p bara-runtime branch_eq_return_42`、
  `nix develop -c cargo test -p btbc-cli
  check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures`、および
  `nix develop -c ./scripts/verify`。

- 2026-06-08 19:36 JST: B5 large milestone completion の準備として、decoder が
  `ret` と direct `call rel32` の後続 block bytes を保持できるようにした。
  explicit terminator で EOF に到達した場合は missing return sentinel を追加しない。
- 検証: `nix develop -c cargo test -p bara-isa-x86 trailing_block_bytes`、
  `nix develop -c cargo test -p bara-isa-x86 call_rel32_then_fallthrough_instruction`、
  および `nix develop -c cargo test -p bara-isa-x86
  missing_ret_becomes_unsupported_instruction` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 19:26 JST: B5 の 9 つ目の小ステップとして、
  short `jne/jnz rel8` を decode / lift し、`X86Cond::NotEqual` の
  `Terminator::CondJump` として IR に追加した。負 displacement の target
  計算も regression で確認した。
- 検証: `nix develop -c cargo test -p bara-isa-x86 jne` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 19:07 JST: B5 の 8 つ目の小ステップとして、
  short `je/jz rel8` を decode / lift し、`X86Cond::Equal` の
  `Terminator::CondJump` として IR に追加した。fallthrough 側の後続命令は
  次 block として保持する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 je` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:59 JST: B5 の 7 つ目の小ステップとして、
  `test eax,eax` を decode / lift し、`IrOp::Test` として
  flags-producing IR に追加した。ARM64 emit は flag lowering 実装前の
  explicit unsupported として分類する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 test_eax`、
  `nix develop -c cargo test -p bara-ir test_op`、および
  `nix develop -c cargo test -p bara-arm64 test_ops_are_not_emitted_before_flag_lowering`
  が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:48 JST: B5 の 6 つ目の小ステップとして、
  `cmp eax, imm8/imm32` を decode / lift し、`IrOp::Cmp` として
  flags-producing IR に追加した。ARM64 emit は flag lowering 実装前の
  explicit unsupported として分類する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 cmp`、
  `nix develop -c cargo test -p bara-ir cmp`、および
  `nix develop -c cargo test -p bara-arm64 cmp_ops_are_not_emitted_before_flag_lowering`
  が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:41 JST: B5 の 5 つ目の小ステップとして、
  `FlagValue::{Known, Unknown}` と CF/PF/AF/ZF/SF/OF を持つ `Flags`
  domain model を `bara-ir` に追加した。`cmp` / `test` / `jcc` の
  decode / lift / emit は後続小ステップに分離した。
- 検証: `nix develop -c cargo test -p bara-ir flags` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 18:36 JST: B5 の 4 つ目の小ステップとして、
  `Fallthrough`、`DirectJump`、`CondJump`、`X86Cond` を typed IR
  terminator として追加した。branch lowering / fixup はまだ実装せず、
  ARM64 emit は explicit unsupported として分類する。
- 検証: `nix develop -c cargo test -p bara-ir` と
  `nix develop -c cargo test -p bara-arm64 emit::tests` が通過した。
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 17:15 JST: B5 の 3 つ目の小ステップとして、terminator
  がない decoded stream 末尾を暗黙 fallthrough とせず、
  `MissingReturnTerminator` の typed unsupported terminator を持つ
  `BasicBlock` として lift するようにした。
- 検証: `nix develop -c cargo test -p bara-isa-x86 lift::tests` と
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 16:45 JST: B5 の 2 つ目の小ステップとして、lifter に
  basic block 分割境界を導入した。`ret` などの terminator instruction で
  block を確定し、後続 instruction があれば次の `BlockId` と source range
  を持つ `BasicBlock` として lift する。
- 検証: `nix develop -c cargo test -p bara-isa-x86 lift::tests` と
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 16:32 JST: B5 の最初の小ステップとして、`add` / `sub`
  fixture coverage が control-flow 前段の regression corpus に含まれている
  状態を TODO と進行履歴へ反映した。既存の `tests/cases`、`tests/expected`、
  `crates/bara-runtime` regression、`tests/expected-reports/blackbox.json` が
  `add` / `sub` 単独および複合 fixture を保持している。
- 検証: `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 15:59 JST: B4 の最後の小ステップとして、unsupported
  syscall / external call の分類と report schema を安定させた。
  function-level の emit unsupported boundary は corpus failure
  `message` に stable JSON として出力される。syscall は ABI と
  address range、external call は symbol id、unresolved/public symbol
  import target、call site / return address を記録する。
- 検証: `nix develop -c cargo test -p btbc-cli
  function_run::tests::unsupported`、`nix develop -c ./scripts/check-domain-types`、
  `nix develop -c ./scripts/check-no-invisible-chars`、`git diff --check`、
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 15:51 JST: B4 の 7 つ目の小ステップとして、libc / dyld /
  import 呼び出しを直接模倣せず、public symbol/import identity として扱う
  model を追加した。`ExternalCallRequest` は `ExternalSymbolImport` を保持し、
  `libc::puts`、`libc::write`、`dyld_stub_binder` を public symbol identity
  として表現できる。
- 検証: `nix develop -c cargo test -p bara-ir` が通過した。続く final B4
  step で full `nix develop -c ./scripts/verify` を実行する。
- 2026-06-08 15:40 JST: B4 の 6 つ目の小ステップとして、macOS / Linux /
  Windows の OS ABI 差分を stdout helper emission strategy 境界で分離した。
  `arm64-apple-macos` は public `_write` prologue strategy に解決され、
  `aarch64-unknown-linux-gnu` と `aarch64-pc-windows-msvc` は
  `write_stdout` helper emission の explicit unsupported target として分類される。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact`、
  `nix develop -c ./scripts/check-domain-types`、`nix develop -c ./scripts/check-no-invisible-chars`、
  `git diff --check`、および `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 14:34 JST: B4 の 5 つ目の小ステップとして、stdout 相当を
  Bara host helper から native stdout emission へ変換する境界を文書化した。
  `write_stdout(ptr_len_to_unit)` は Bara host effect capability であり、
  macOS ARM64 standalone artifact では output packaging 境界が public
  `_write` prologue に変換する。decode / lift / core IR / ARM64 emit /
  manifest parsing / oracle comparison へ native emission の責務を漏らさない。
- 検証: documentation-only 変更として
  `nix develop -c ./scripts/check-no-invisible-chars` と `git diff --check` が
  通過した。code/script/config 変更がないため full `./scripts/verify` は省略した。
- 2026-06-08 14:26 JST: B4 の 4 つ目の小ステップとして、
  `puts` / `write` 相当の stdout effect を Bara host helper
  `write_stdout(ptr_len_to_unit)` の typed request として扱えるようにした。
  `HostTrapKind::Stdout` は `HostHelperRequest::WriteStdout` へ写像され、
  executable manifest preflight は resolved manifest helper を IR 側の
  `HostHelperAbi` と照合してから実行へ進む。
- 検証: `nix develop -c cargo test -p bara-ir`、
  `nix develop -c cargo test -p btbc-cli executable_run`、および
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 13:13 JST: B4 の 3 つ目の小ステップとして、
  `helper_call_external`、`helper_unimplemented`、`helper_exit` の最小 ABI を
  typed domain value として定義した。helper ABI は名前と signature の pure
  value であり、runtime 実行や host syscall 呼び出しはまだ行わない。
- 検証: `nix develop -c cargo test -p bara-ir` と
  `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 13:00 JST: B4 の 2 つ目の小ステップとして、external
  symbol / import call を `BoundaryRequest::Helper(HelperRequest::CallExternal(...))`
  として IR に残す境界を追加した。ARM64 emit は実行コードを出さず
  `ExternalCallUnsupported { request }` を返す。
- 検証: `nix develop -c cargo test -p bara-ir`、
  `nix develop -c cargo test -p bara-arm64`、`nix develop -c ./scripts/verify`
  が通過した。
- 2026-06-08 12:50 JST: B4 の先頭小ステップとして、x86_64 `syscall` を
  typed public ABI request として IR に残す境界を追加した。`syscall` は
  `BoundaryRequest::Syscall(SyscallRequest { abi: X86_64, at, return_to })`
  として lift され、ARM64 emit は実行コードを出さず
  `SyscallUnsupported { request }` を返す。
- 検証: `nix develop -c cargo test -p bara-ir`、
  `nix develop -c cargo test -p bara-isa-x86`、
  `nix develop -c cargo test -p bara-arm64`、`nix develop -c ./scripts/verify`
  が通過した。
- 2026-06-08 12:06 JST: 旧 M 系マイルストーンと `当面の最短 TODO` の
  具体情報を、削除ではなく [TODO.md](../TODO.md) の B1-B8 へ吸収した。
  `add/sub`、`cmp/test/jcc`、`push/pop/call`、Rosetta oracle、Haskell
  verifier、fallback、metadata 出力などの項目を線形ロードマップ内に残し、
  実行順は B1-B10 のまま維持した。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:59 JST: 実装順を [TODO.md](../TODO.md) の
  `線形実装ロードマップ` に一本化した。README も独立した実装順を持たず
  TODO の線形ロードマップへ案内する形にした。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:51 JST: B8 と PE / Wine 接続前段の間に、B9:
  実 x86 32-bit アプリ対応を挿入した。B9 は互換性上の論点を Wine 接続前に
  発見するための推奨ステップとし、blocker が大きい場合は記録したうえで
  飛ばして B10: PE / Wine 接続前段へ進んでよいことを明記した。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:42 JST: TODO 本流の長期目標を再整理した。PE / Wine
  接続前に B8: 実 x86_64 macOS アプリ起動へ到達することを明記し、旧 B9
  の source ISA mode / x86_32 guardrail と旧 B10 の user-space runtime
  architecture を B8 の設計制約へ統合した。旧 B11/B12 の wasm2c /
  platform adapter / LLVM IR / Wasm 副出力は
  [将来構想メモ](future-research-concepts.md) へ移し、本流 TODO から外した。
- 検証: documentation-only 変更として `nix develop -c ./scripts/check-no-invisible-chars`
  と `git diff --check` が通過した。full `./scripts/verify` は code/script/config
  変更がないため省略した。
- 2026-06-08 11:21 JST: B3 の最後の小ステップとして、`clang` packaging 経路と pure writer 経路の差分検証を `bara-mach-o` の公開仕様ベース model 比較として追加した。`MachOArm64ClangPackagingModel`、comparison report、classified mismatch issue を定義し、`_main` / `__TEXT` / `__text` / optional `__const` / minimal load commands の parity を検証できるようにした。B3 は review gate に到達した。
- 検証: `nix develop -c cargo test -p bara-mach-o` は未実装 comparison API の compile error で期待どおり失敗し、実装後に通過した。変更全体の `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 11:08 JST: B3 の 3 つ目の小ステップとして、`bara-mach-o` の writer plan に public Mach-O model を追加した。`_main` entry、`__TEXT` segment、mandatory `__text` section、const payload がある場合の `__const` section、最小 `LC_SEGMENT_64` / `LC_MAIN` 相当の load command model を domain type として定義した。
- 検証: `nix develop -c cargo test -p bara-mach-o` は未実装 model API の compile error で期待どおり失敗し、実装後に通過した。変更全体の `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:46 JST: B3 の 2 つ目の小ステップとして、`bara-mach-o` crate を追加し、ARM64 Mach-O executable writer の pure planning 境界を設計した。`MachOArm64MainCode`、`MachOArm64ConstData`、writer request、payload、plan、target を domain type として定義し、empty payload parts は classified input error にする。
- 検証: `nix develop -c cargo test -p bara-mach-o` は未実装 API の compile error で期待どおり失敗し、実装後に通過した。変更全体の `nix develop -c ./scripts/verify-supply-chain` と `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:26 JST: B3 の最初の小ステップとして、Mach-O input parser と output artifact planning / materialization の責務を module 境界で分離した。`binary_format::input` が public format probe / Mach-O metadata / load command parsing を扱い、`binary_format::output` が executable image plan / materialization を扱う。外部公開 API は `binary_format` と crate root の re-export で維持した。
- 検証: 移動前後で `nix develop -c cargo test -p bara-oracle binary_format::tests` が通過した。変更全体の `nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:09 JST: B2 の最後の小ステップとして、unsupported host を classified stable output にした。`NativeArtifactError::UnsupportedHost` は `EmitError` の分類を保ちつつ、artifact kind、target triple、host os/arch を含む JSON message を返す。
- 検証: `nix develop -c cargo test -p btbc-cli unsupported_host_error_serializes_as_stable_json_message` は既存 text message との差分で期待どおり失敗し、実装後に同テスト、`nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:04 JST: B2 の 4 つ目の小ステップとして、外部 `clang` packaging を `NativeArtifactPackager` trait 境界へ分離した。`ClangNativeArtifactPackager` が現行 process 実行を担当し、test fake packager は同じ request から linked executable metadata を返せる。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_packaging_boundary_accepts_different_packagers` は未実装 trait / request の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 10:00 JST: B2 の 3 つ目の小ステップとして、generated code、stdout data、toolchain command、output path の責務を分離した。`NativeGeneratedCode`、`NativeStdoutData`、`NativeToolchainCommand`、`NativeArtifactOutputPath` を導入し、`link_assembly_source` は typed output path と toolchain command を組み立ててから外部 process を呼ぶようにした。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_request_types_separate_code_stdout_command_and_output_path` は未実装型 / method の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 09:48 JST: B2 の 2 つ目の小ステップとして、native linked executable artifact の metadata JSON 出力を追加した。`link-fixture-arm64-main` は text ではなく artifact metadata JSON を返し、metadata は execution result と別の domain value として保持される。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_metadata_serializes_as_stable_json` は未実装 serializer / accessor の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact`、`nix develop -c ./scripts/verify-supply-chain`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 09:35 JST: merge 済み B1 branch を local cleanup し、B2 branch `task/b2-artifact-domain-types` を開始した。B2 の最初の小ステップとして、raw ARM64 bytes、native assembly source、linked executable を `native_artifact` module 内の別 domain type として分離した。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_types_separate_raw_source_and_linked_executable` は未実装型の compile error で期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact::tests`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-08 09:23 JST: B1 の最後の小ステップとして、`docs/hello-world-roadmap.md` を完了済みロードマップに整理し、B1 安定化成果から B2 の実行可能成果物モデルへ接続した。
- 検証: `nix develop -c ./scripts/check-no-invisible-chars`、`git diff --check`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 21:47 JST: B1 の先頭小ステップとして、生成 executable の smoke test を blackbox report に追加した。`return_42_native_executable_smoke` は `return_42` fixture を native executable として link し、実プロセス exit status 42 と空 stdout/stderr を確認する。
- 検証: 期待 fixture 更新後に `nix develop -c cargo test -p btbc-cli check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures` が期待どおり失敗し、実装後に同テスト、`nix develop -c cargo test -p btbc-cli check_blackbox_writes_report_and_schema_specific_actual_outputs`、`nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 21:54 JST: B1 の 2 つ目の小ステップとして、`link-fixture-arm64-stdout-main` の出力を stable `ObservedResult` JSON report にした。生成 artifact は command 内で実行され、stdout `hello world\n`、exit status 0、stderr 空が JSON に含まれる。
- 検証: 期待 fixture 更新後に `nix develop -c cargo test -p btbc-cli link_fixture_arm64_stdout_main_writes_hello_world_executable` が期待どおり失敗し、実装後に同テストと `nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 22:10 JST: B1 の 3 つ目の小ステップとして、native artifact packaging / toolchain / execution の failure classification を整理した。temporary assembly と `clang` 呼び出し、linked executable 欠落は `EmitError`、artifact 実行失敗は `RunError` に分類する。
- 検証: 期待分類テスト追加後に `nix develop -c cargo test -p btbc-cli packaging_and_toolchain_failures_are_emit_errors` が期待どおり失敗し、実装後に `nix develop -c cargo test -p btbc-cli native_artifact::tests` と `nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 23:46 JST: B1 の 4 つ目の小ステップとして、native artifact 関連の CLI behavior tests を `main.rs` から `crates/btbc-cli/src/native_artifact_cli_tests.rs` へ分割した。
- 検証: `nix develop -c cargo test -p btbc-cli native_artifact_cli_tests` と `nix develop -c ./scripts/verify` が通過した。
- 2026-06-07 14:48 JST: Bara の agent action commands を VSCode / Codex IDE から選べるように、repo-scoped skill として `.agents/skills/bara-*` を追加した。
- 検証: `nix develop -c ./scripts/verify` は `verify-cves` の pipe 処理で停止したため中断。代わりに同等 gate を分解して実行し、`cargo fmt --all -- --check`、`./scripts/check-no-invisible-chars`、`./scripts/check-domain-types`、`cargo metadata --locked --format-version 1`、manual `cargo audit` baseline check、`cargo deny check`、`./scripts/verify-nix-package`、`cargo check --workspace --all-targets`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`./scripts/verify-blackbox` が通過した。

## 進行記録の更新規律

この文書は「履歴」だけでなく、コンテキストなしで現在何が進行中かを把握する入口でもある。

今後、TODO-backed implementation、refactoring、architecture work、milestone branch work、
または大きな documentation / policy change を開始、停止、完了、保留、方針転換するときは、
この文書の `現在の作業スナップショット` を同じ変更で更新する。

各進行記録には、最低限以下を含める:

- 更新時刻。形式は `YYYY-MM-DD HH:MM JST` とする。
- 状態。`planned`、`in_progress`、`paused`、`blocked`、`completed`、`superseded` のどれかを明記する。
- 対応する `TODO.md`、`docs/design-todo.md`、または focused roadmap entry。
- 作業 branch と、commit 済みなら最新 commit。
- 何が終わり、何が未完了で、次に何をするべきか。
- 実行した検証、または検証を狭めた理由。

進行中の項目を放置しない。作業が完了、保留、または別方針に置き換わった場合は、
古い `in_progress` 状態を必ず更新する。

## 現在地

最小 `hello world` milestone は完了済み。

到達済み:

- raw x86 function fixture を decode / lift / ARM64 emit できる。
- ARM64 machine code artifact をファイルへ出力できる。
- macOS ARM64 executable artifact として package できる。
- 生成 executable を OS 上で起動し、実 OS stdout に `hello world\n` を出せる。

現在の主な次フェーズ:

- fixture 専用の成功経路を実バイナリ対応へ広げる。
- B4-B7 で syscall / libc 境界、control flow、Mach-O 入力、oracle /
  regression 基盤を広げる。
- B8 で実 x86_64 macOS アプリを user-space runtime から起動できる状態を
  目指す。
- B9 で実 x86 32-bit アプリ対応を扱う。blocker が大きい場合は記録して
  飛ばせるが、B10 の PE / Wine 接続前に先に処理するのが望ましい。
- PE / Wine 接続前段は B10 として扱う。
- wasm2c、NDA target adapter、LLVM IR / Wasm 副出力は
  [将来構想メモ](future-research-concepts.md) に分離し、本流 TODO から外す。

## 完了済みマイルストーン

### Hello World milestone

状態:

- 完了。

到達点:

- raw x86_64 function bytes から ARM64 native runner で `rax` return value を比較できる。
- stdout host trap を fixture として扱い、expected / actual JSON で比較できる。
- Bara executable manifest v0 から raw function pipeline へ変換できる。
- public Mach-O probe、Mach-O backed raw function 実行、Mach-O backed stdout fixture 実行を扱える。
- raw testcase から ARM64 machine code artifact を出力できる。
- raw testcase から macOS ARM64 executable artifact を生成できる。
- stdout host trap fixture を standalone macOS ARM64 executable artifact として package し、実 OS stdout へ `hello world\n` を出せる。

検証:

- `nix develop -c ./scripts/verify` が Hello World milestone 完了時点で通過済み。
- 詳細な段階履歴は [docs/hello-world-roadmap.md](hello-world-roadmap.md) に保存済み。

## 進行上の決定

### TODO と設計 TODO の分離

状態:

- 完了。

決定:

- [TODO.md](../TODO.md) は線形実装ロードマップを管理する。
- [docs/design-todo.md](design-todo.md) は詳細設計、分割方針、リファクタリング、単一責任監査の TODO を管理する。

理由:

- 実装作業とリファクタリング作業が同じ TODO に混ざると、差分の目的が曖昧になる。
- 今後は feature work と refactoring work をできるだけ分けて進行できるようにする。

### エージェント進行規律の固定

状態:

- 運用ルールとして追加済み。

決定:

- エージェントは実装前に関連する `TODO.md` と `docs/design-todo.md` を参照する。
- TODO にない作業は、先に milestone または focused roadmap entry として記録してから実装する。
- 実装状況と TODO の状態を一致させる。
- 完了済みマイルストーンや大きな方向転換は、この文書に記録する。

理由:

- セッションごとにコンテキストを再説明しなくても、ドキュメントから次に進むべき作業を判断できるようにする。
- プロジェクトがどのように進行したかを、コミット履歴に依存せず把握できるようにする。

### タイムスタンプ付き進行スナップショット

状態:

- completed: 2026-06-07 20:00 JST。

決定:

- [docs/progress.md](progress.md) の先頭付近に `現在の作業スナップショット` を置く。
- 作業の開始、停止、完了、保留、方針転換時に、時刻、状態、対応 TODO、branch/commit、検証、次の作業を記録する。
- `in_progress` 状態は放置せず、完了、保留、または置き換え時に必ず更新する。

理由:

- コミット履歴や会話ログがなくても、次に読むべき TODO、現在の作業状態、直近の完了作業、必要な検証を把握できるようにする。
- エージェントが別セッションで再開しても、古いコンテキストに依存せず同じ運用判断をできるようにする。

## 次に進む場所

現在の実装ロードマップは [TODO.md](../TODO.md) の `線形実装ロードマップ`
だけを参照する。上から順に読み、最初の未完了項目を次の作業候補にする。
現時点の次候補は B4: x86 syscall / libc 境界。

優先度の高い設計監査は [docs/design-todo.md](design-todo.md) の D1 と D2:

- D1: CLI と command 境界
- D2: Artifact domain model
