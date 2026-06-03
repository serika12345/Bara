# 初期スコープ

## 方針

初期実装では、パフォーマンスより検証しやすさと実装の小ささを優先する。

最初から PE、Wine、syscall、SSE、例外、unwind、fallback まで扱わない。まずは raw x86_64 関数片を ARM64 に変換し、arm64 macOS 上で実行して戻り値を比較できる状態を作る。

## M1 の対象

入力:

- raw x86_64 function bytes
- entry offset は 0
- 引数なし
- 戻り値は x86_64 の `rax`

最初のテストケース:

```text
mov eax, 42
ret
```

期待結果:

```json
{
  "return_value": 42,
  "exit_status": 0
}
```

## 初期実行モデル

関数単位で実行する。

```text
x86_64 bytes
  -> decode
  -> IR
  -> ARM64 emit
  -> executable buffer
  -> runner calls generated function
  -> actual.json
```

プロセス全体の再現は扱わない。

## 初期 ABI

最初は以下だけに限定する。

- 引数なし
- `rax` を戻り値とする
- stack は `ret` のために最小限だけ扱う
- flags は必要になるまで観測対象にしない
- memory access は M1 では扱わない

その後の拡張順:

1. 整数引数
2. `add` / `sub`
3. `cmp` / `test` / `jcc`
4. `push` / `pop`
5. direct `call`
6. helper call

## 初期 CpuState

初期実装では `CpuState` を明示し、検証しやすさを優先する。

```rust
#[repr(C)]
pub struct CpuState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
}
```

最初はすべての命令で state を厳密に最適化しなくてよい。必要なら ARM64 native register に一時的に置き、block 境界や helper 呼び出し前後で `CpuState` に同期する。

## 初期は扱わないもの

- PE loader
- Wine 統合
- Mach-O loader
- ELF loader
- syscall
- libc import
- SSE / AVX
- x87
- segment register
- TLS
- signal
- exception
- unwind
- self-modifying code
- JIT/fallback
- register allocation 最適化
- lazy flags
- indirect branch optimization

## 成功条件

M1 は以下を満たしたら完了とする。

- raw bytes `b8 2a 00 00 00 c3` を入力できる。
- x86_64 instruction を decode できる。
- IR を JSON dump できる。
- ARM64 machine code を生成できる。
- executable buffer で実行できる。
- `actual.json` に `return_value: 42` を出せる。
- Rosetta oracle で得た `expected.json` と比較できる。

