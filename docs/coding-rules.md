# コーディングルール

## 方針

実装は、関数型プログラミングの考え方に寄せ、シグネチャから仕様と境界が読める形を優先する。

基本アーキテクチャでは I/O とロジックを分離する。I/O 境界の外にある decode、lift、IR 変換、emit、metadata 生成、比較、検証は、外部から見て純粋関数として振る舞うようにする。

設計判断では、DRY より単一責任を優先する。重複して見えるコードでも、責務、変更理由、検証観点が違うなら早すぎる共通化をしない。

抽象化は継承ではなく移譲と合成を基本にする。Rust では trait、newtype、明示的な field、関数引数を使い、暗黙の親子関係で振る舞いを共有しない。

実行時の都合で `unsafe` が必要な部分は避けない。ただし、`unsafe` は runtime 境界へ局所化し、命令変換、IR、metadata、検証ロジックへ広げない。

## シグネチャで仕様を表す

曖昧な `u64`、`usize`、`Vec<u8>` をそのまま横断的に渡さない。意味のある型、newtype、enum、Result を使い、呼び出し側が誤用しにくい API にする。

優先すること:

- source PC と target PC を型で分ける。
- raw bytes、decoded instruction、IR、emitted code を型で分ける。
- fallthrough、unsupported instruction、helper call などを enum で明示する。
- 成功できない処理は `Result<T, E>` にし、panic を仕様表現として使わない。
- invariant はコメントだけでなく、型、constructor、checker のいずれかで表す。
- 可変状態は API 境界で小さく閉じ、純粋な変換関数として読める箇所を増やす。

例:

```rust
pub fn lift_block(bytes: X86Bytes, start: X86Va) -> Result<BasicBlock, LiftError>;

pub fn emit_program(program: &Program) -> Result<CompiledCode, EmitError>;

pub fn compare_oracle(expected: &OracleResult, actual: &RunResult) -> CompareResult;
```

避けること:

```rust
pub fn compile(bytes: Vec<u8>, addr: u64) -> Vec<u8>;
```

この形では、入力が x86_64 なのか ARM64 なのか、`addr` が source address なのか target PC なのか、失敗時に何が起きるのかが読めない。

## I/O とロジックの分離

ファイル、環境変数、時刻、乱数、プロセス実行、標準入出力、OS API、executable memory などの副作用は、CLI、test harness、runtime 境界へ寄せる。

core logic は入力値を受け取り、結果値を返す形にする。同じ入力に対して同じ出力を返し、外部状態へ依存しないことを基本にする。

優先すること:

- ファイル読み込みは CLI 側で行い、core には `X86Bytes` や `ProgramInput` として渡す。
- JSON の parse / serialize は境界層に寄せ、core は型付き値を扱う。
- Rosetta 実行、native runner 実行、プロセス起動は test harness 側に閉じる。
- decode / lift / optimize / emit は、入力 IR や設定値を受けて新しい値を返す。
- 設定、target triple、ABI、feature flag は global state ではなく明示的な引数にする。
- log や trace が必要な場合は、戻り値の metadata として返すか、明示的な collector を渡す。
- I/O 境界以外では、void 型関数、Rust では `()` を返す関数を書かない。変更後の値、検証結果、metadata、または分類された error を返す。

許容すること:

- パフォーマンスのため、関数内部で `Vec`、arena、buffer、cache などをミューテーションする。
- 外部から観測できる振る舞いが純粋で、呼び出し順序や隠れた global state に依存しない。
- 内部 mutation の結果を API 境界から漏らさない。

例:

```rust
pub fn compile_program(input: ProgramInput, options: CompileOptions) -> Result<CompiledProgram, CompileError>;

pub fn validate_program(program: &Program) -> ValidationReport;
```

避けること:

```rust
pub fn compile_current_file() -> Result<(), CompileError>;

pub fn validate_program(program: &Program);
```

この形では、入力ファイル、出力先、環境、ログ、実行順序がシグネチャに現れず、検証や property test が難しくなる。`validate_program` のようなロジック関数が `()` を返す場合も、何を検証し、何が失敗し、呼び出し後に何が変わったのかがシグネチャから読めない。

## 責務と合成

単一責任を保つため、モジュール、型、関数はそれぞれ 1 つの変更理由を持つようにする。DRY は重要だが、責務が違う処理を共通化して境界を曖昧にしない。

優先すること:

- decode、lift、emit、runtime、oracle 比較を別の責務として扱う。
- 同じような処理でも、ISA semantics と file format handling のように変更理由が違うなら分ける。
- 共通化は、同じ責務の中で意味が一致してから行う。
- 振る舞いの共有は、継承ではなく小さな関数、trait、newtype、明示的な委譲で行う。
- 上位の型は下位コンポーネントを field として持ち、処理を委譲する。

避けること:

- 少し似ているだけの処理を generic helper に押し込む。
- decode と emit の都合を 1 つの型へ混ぜる。
- 共通 base class 的な設計で複数の責務を継承させる。
- trait に不要な method を増やし、実装側へ空実装や到達不能分岐を強いる。

判断基準:

- 変更理由が同じなら共通化を検討する。
- 変更理由が違うなら、多少の重複は許容して責務を分ける。
- 抽象化でシグネチャが曖昧になるなら、具体的な型と明示的な委譲を優先する。

## パッケージング

パッケージ、crate、module、directory は技術駆動ではなくドメイン駆動で切る。`utils`、`common`、`helpers`、`types` のような技術名だけの置き場を先に作らない。

関心ごとごとにディレクトリを作り、その中へ型、関数、テスト、metadata を置く。ルート直下に実装ファイルを増やさない。ルートは workspace 設定、README、ライセンス、トップレベルのドキュメントだけにする。

I/O は専用のディレクトリに固める。ファイル、JSON、CLI、プロセス実行、OS API、環境変数、標準入出力、oracle runner 連携などは、ドメインロジックのディレクトリへ散らさない。

優先すること:

- `decode`、`ir`、`emit`、`runtime`、`oracle`、`metadata` のように、関心ごとが読める名前で切る。
- I/O は `io`、`cli`、`harness`、`runtime_io` など、副作用を持つことが名前から分かる場所へ置く。
- ドメインロジックの module は、I/O module へ依存しない。
- I/O module は境界で値を読み書きし、core の純粋関数へ型付き値を渡す。
- package の公開 API は、その関心ごとの語彙で構成する。

避けること:

- `src/util.rs`、`src/helpers.rs`、`src/common.rs` に関心ごとの違う処理を集める。
- ルート直下に `decoder.rs`、`emitter.rs`、`runner.rs` のような実装ファイルを並べ続ける。
- JSON 読み書きや filesystem access を `ir`、`decode`、`emit` の中へ混ぜる。
- 技術層名だけで `core` を肥大化させ、複数ドメインの責務を抱え込ませる。

例:

```text
crates/
  bara-isa-x86/
    src/
      decode/
      lift/
  bara-ir/
    src/
      program/
      block/
      validate/
  bara-arm64/
    src/
      emit/
      fixup/
  bara-runtime/
    src/
      io/
      executable_memory/
      runner/
  bara-oracle/
    src/
      io/
      rosetta/
      compare/
```

## 開発環境

開発環境は Nix で定義する。ホスト OS に直接入った toolchain やグローバルインストール済みのコマンドを前提にしない。

Nix Flake で dev shell を定義し、Rust toolchain、formatter、linter、test runner、assembler/linker などをそこから使う。Haskell など追加の言語 toolchain は、実装対象になった時点で dev shell に追加する。

優先すること:

- Rust、formatter、linter、test runner、assembler/linker などは Nix dev shell から使う。
- Haskell など追加の toolchain は、必要になった時点で Nix dev shell に定義してから使う。
- 日常コマンドは `nix develop -c ...` または direnv 経由の dev shell 内で実行する前提にする。
- toolchain の version、native dependency、macOS/Linux 差分は Nix 側に寄せる。
- エディタ設定は EditorConfig を使い、改行、文字コード、末尾改行、空白、インデントを repository で統一する。
- `rustfmt` など言語固有 formatter と EditorConfig の責務を分ける。EditorConfig は基本的なテキスト整形の下限を揃える。

避けること:

- 手元に入っている `cargo`、`rustc`、`ghc`、`nasm`、`clang` などへ暗黙に依存する。
- 開発者ごとの shell 設定や IDE 設定を前提にする。
- Nix で定義されていないツールを README や script の必須手順にする。
- dev shell に入らず、手元のグローバル toolchain で成功した結果を正規の確認結果として扱う。

EditorConfig の基本方針:

- UTF-8
- LF
- final newline あり
- trailing whitespace は原則削除
- default indent は space 2
- Rust は space 4
- Markdown は trailing whitespace を必要に応じて許容する

## unsafe の局所化

`unsafe` は、原則として以下の境界だけに置く。

- executable memory の確保、保護属性変更、解放
- machine code buffer への低レベル書き込みのうち、安全 API で包めない箇所
- generated code を function pointer として呼び出す箇所
- FFI、OS API、ABI 境界

ルール:

- `unsafe` block は最小スコープにする。
- `unsafe` を含む関数は、可能なら private にし、安全な wrapper だけを公開する。
- `unsafe` の前後で満たすべき条件を `Safety:` コメントで書く。
- decode / lift / IR / metadata / verifier には `unsafe` を置かない。
- runtime crate に `unsafe` を集約し、他の crate からは型付き API 経由で使う。

例:

```rust
pub struct ExecutableBuffer {
    ptr: NonNull<u8>,
    len: usize,
}

impl ExecutableBuffer {
    pub fn call_no_args_u64(&self) -> Result<u64, RunError> {
        // Safety: buffer was allocated executable, contains a complete function,
        // and the runner only exposes the no-args u64-return ABI.
        unsafe { call_generated_no_args_u64(self.ptr.as_ptr()) }
    }
}
```

## 境界ごとの責務

- `bara-isa-x86`: x86_64 decode と lift。I/O と `unsafe` なし。
- `bara-ir`: IR、invariant、validation、metadata model。I/O と `unsafe` なし。
- `bara-arm64`: ARM64 emit 計画、fixup、machine code bytes の構築。OS I/O と `unsafe` なし。
- `bara-oracle`: oracle 比較と Rosetta 連携境界。I/O は `io` directory に集約する。
- `btbc-runtime`: executable memory、generated code 呼び出し、OS/ABI 境界。`unsafe` をここへ集約する。
- `btbc-cli`: 入出力、JSON、コマンド実行。仕様境界は各ドメイン crate と runtime の型に委譲する。
- `spec/`: Haskell 仕様モデル、property test、検証器。

## レビュー基準

新しい API を追加するときは、以下を確認する。

- シグネチャだけで、入力、出力、失敗、所有権、アドレス空間が読めるか。
- `u64` や `Vec<u8>` が意味を失ったまま境界を越えていないか。
- panic ではなく、分類可能な error / unsupported reason になっているか。
- I/O、時刻、乱数、global state、プロセス実行が core logic に漏れていないか。
- I/O 境界以外の関数が `()` を返していないか。
- DRY のために、変更理由が違う責務を 1 つの抽象へ混ぜていないか。
- 継承的な共有ではなく、明示的な移譲と合成で表現できているか。
- package や module が技術名ではなく、関心ごとの名前で切られているか。
- ルート直下に実装ファイルを増やしていないか。
- I/O が専用ディレクトリに集約され、ドメインロジックへ散っていないか。
- 開発手順が Nix dev shell 前提になっているか。
- EditorConfig と formatter の責務が分かれているか。
- 内部 mutation が外部から見た純粋性を壊していないか。
- `unsafe` が core logic に漏れていないか。
- `unsafe` の根拠が `Safety:` コメントと型の invariant で説明できるか。
