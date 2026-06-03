# AGENTS.md

This file is the operational rulebook for coding agents working on Bara.
Follow it together with the repository documentation, especially
`docs/coding-rules.md`, `docs/clean-room.md`, `docs/scope.md`,
`docs/ir.md`, and `docs/test-oracle.md`.

## Project Intent

Bara is a research project for a binary-to-binary compiler. The motivating
concern is the future of CrossOver/Wine-style compatibility layers after
Rosetta 2 becomes unavailable for general use.

The project is not a Rosetta clone. The goal is to study a decomposed design
where the translation core, runtime, verifier, metadata, and OS/ABI/loader
boundaries can evolve independently and eventually connect to compatibility
layers such as Wine.

Initial scope is deliberately small:

- raw x86_64 function bytes
- function-level execution, not whole-process emulation
- no arguments
- `rax` return value
- minimal decode, IR, ARM64 emit, executable runner
- `expected.json` / `actual.json` comparison
- first success case: `mov eax, 42; ret`

## Clean-Room Rules

Use only public specifications, self-authored tests, and externally observable
behavior as implementation grounds.

Allowed implementation sources:

- Intel/AMD ISA manuals
- ARM Architecture Reference Manual
- System V ABI
- Windows x64 ABI
- public Mach-O / PE / ELF specifications
- public OS API documentation
- self-authored test cases
- externally observed Rosetta outputs through the test harness
- public documentation for FEX-Emu, Box64, and QEMU user-mode
- externally observed behavior of existing translation layers

Forbidden implementation sources:

- Rosetta disassembly
- Rosetta internal symbols
- Rosetta internal metadata formats
- private Apple ABIs
- Rosetta function layout or control-flow structure
- code structure derived from Rosetta binaries
- copying or mimicking FEX-Emu, Box64, or QEMU internals
- existing translation-layer helper layouts, internal metadata, or code
  structure as implementation grounds

Rosetta is only a black-box oracle for externally observable behavior. Existing
Linux user-space translation layers are research and comparison targets only.

## Development Environment

Use the Nix dev shell for development commands. Do not treat host-global tools
as authoritative.

Preferred commands:

```sh
nix develop
nix develop -c cargo --version
nix develop -c rustc --version
```

The repository uses `.envrc` with `use flake`. If using direnv, allow it before
working in the shell.

Rust tooling is provided by `flake.nix`. Add more tools to the flake before
making them required in scripts or documentation.

Use `.editorconfig` for baseline text formatting:

- UTF-8
- LF line endings
- final newline
- trailing whitespace removed by default
- 2-space default indentation
- 4-space Rust indentation
- Markdown may keep trailing whitespace when needed

Use language-specific formatters, such as `rustfmt`, for language formatting.
EditorConfig is only the baseline.

## Architecture Rules

Separate I/O from logic. Outside explicit I/O boundaries, behavior should be
externally pure: the same input produces the same output, without hidden global
state, time, randomness, process state, or filesystem dependency.

Keep these operations pure from the outside:

- decode
- lift
- IR transformation
- emit planning
- metadata generation
- oracle comparison
- validation

Internal mutation is allowed for performance, such as mutating a `Vec`, arena,
buffer, or cache inside a function. It must not change externally observable
purity or leak through the API boundary.

Do not write void/unit-returning logic functions outside I/O boundaries. In
Rust, non-I/O logic should not return `()`. Return a changed value, validation
report, metadata, or classified error instead.

Avoid APIs like:

```rust
pub fn validate_program(program: &Program);
```

Prefer APIs like:

```rust
pub fn validate_program(program: &Program) -> ValidationReport;
```

## Signatures as Specifications

Design APIs so signatures explain the boundary and behavior.

Prefer:

- newtypes for addresses and identifiers
- explicit source PC vs target PC types
- separate types for raw bytes, decoded instructions, IR, emitted code, and
  executable code
- enums for unsupported instructions, terminators, helper calls, and
  fallthroughs
- `Result<T, E>` for fallible operations
- constructors or checkers for invariants

Avoid untyped cross-boundary APIs such as:

```rust
pub fn compile(bytes: Vec<u8>, addr: u64) -> Vec<u8>;
```

The signature must make it clear what architecture, address space, ownership,
failure modes, and semantic boundary are involved.

## Responsibility, DRY, and Composition

Prefer single responsibility over DRY. Do not abstract merely because two pieces
of code look similar. If their reasons to change, validation concerns, or domain
responsibilities differ, keep them separate.

Prefer delegation and composition over inheritance-style designs.

In Rust, share behavior through:

- small functions
- narrow traits
- newtypes
- explicit fields
- explicit function arguments

Avoid:

- catch-all generic helpers
- `utils`, `common`, `helpers`, or `types` dumping grounds
- mixing decode and emit concerns in one type
- base-class-style designs
- traits with methods that force empty implementations or unreachable branches

## Packaging Rules

Package, crate, module, and directory boundaries should be domain-driven, not
technology-driven.

Do not place implementation files directly at the repository root. The root is
for workspace configuration, README, license, and top-level documentation.

Group code by concern. I/O must live in dedicated directories and must not be
scattered into domain logic.

Preferred direction:

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

Domain logic modules must not depend on I/O modules. I/O modules should read or
write values at the boundary and pass typed values into pure logic.

## Unsafe Rules

Localize `unsafe` to runtime boundaries.

Allowed `unsafe` areas:

- executable memory allocation, protection changes, and release
- low-level machine-code buffer handling that cannot be wrapped safely
- calling generated code through function pointers
- FFI, OS API, and ABI boundaries

Rules:

- keep `unsafe` blocks as small as possible
- prefer private unsafe functions wrapped by safe public APIs
- write a `Safety:` comment explaining the invariant before each unsafe block
- do not put `unsafe` in decode, lift, IR, metadata, or verifier logic
- centralize runtime `unsafe` in the runtime crate

## Testing and Verification

Initial verification is expected/actual comparison:

```text
x86_64 testcase
  -> Rosetta black-box execution
  -> expected.json

x86_64 testcase
  -> Bara compile
  -> ARM64 native runner
  -> actual.json

expected.json vs actual.json
```

Rosetta may provide only externally visible behavior:

- exit status
- stdout / stderr
- return value
- JSON explicitly emitted by the test harness
- whether the testcase crashed

If tests fail, fix implementation using public ISA/ABI specifications and add a
regression case. Do not infer or copy Rosetta internals.

When there is no Rust workspace yet, verify environment changes with:

```sh
nix flake check
nix develop -c rustc --version
nix develop -c cargo --version
```

Once Rust crates exist, prefer checks through the Nix dev shell.

## Review Checklist

Before completing a change, check:

- Does the signature explain inputs, outputs, failure, ownership, and address
  space?
- Are raw `u64`, `usize`, or `Vec<u8>` values crossing boundaries without
  domain meaning?
- Are fallible operations represented with classified errors?
- Did I/O, time, randomness, global state, or process execution leak into core
  logic?
- Do non-I/O logic functions avoid returning `()`?
- Did a DRY abstraction mix different reasons to change?
- Is behavior shared through delegation and composition?
- Are package and module names domain-oriented?
- Are implementation files kept out of the repository root?
- Is I/O isolated in dedicated directories?
- Were commands run through the Nix dev shell?
- Are EditorConfig and language formatter responsibilities respected?
- Does internal mutation preserve external purity?
- Is `unsafe` absent from core logic and documented at runtime boundaries?
- Are clean-room boundaries preserved?
