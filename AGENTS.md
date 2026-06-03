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

The default local verification gate is:

```sh
nix develop -c ./scripts/verify
```

Run it before completing any coding-agent task that changes code, scripts,
configuration, lockfiles, or repository policy. If the task is documentation
only, run the narrower relevant checks and state what was skipped.

Supply-chain checks are mandatory for dependency, lockfile, or toolchain
changes:

```sh
nix develop -c ./scripts/verify-supply-chain
```

Do not add dependencies without a test-backed need and a domain-level reason.
Keep `Cargo.lock` committed. Do not weaken `deny.toml` to make a dependency
pass; narrow exceptions must be documented in the same change.

Repository security checks must reject Trojan Source-style invisible/control
Unicode characters:

```sh
nix develop -c ./scripts/check-no-invisible-chars
```

For security-relevant changes, run the aggregate check:

```sh
nix develop -c ./scripts/verify-security
```

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

## Domain Type Rules

Bara uses a domain-driven, functional style. Primitive obsession is a design
defect, not a neutral default.

Public APIs in domain crates must use domain types by default. Do not expose
bare `u8`, `u16`, `u32`, `u64`, `usize`, `i32`, `String`, `Vec<u8>`, or
`&[u8]` across module or crate boundaries when the value has domain meaning.
Define a named newtype, enum, value object, or validated collection instead.

Primitive values are allowed only in these places:

- private implementation details
- tests and fixtures
- low-level runtime/FFI boundaries
- serialization/deserialization boundaries
- constructors, parsers, validators, or accessors that explicitly convert
  between untrusted primitive input and trusted domain values

When a primitive must remain in a public API, it must be deliberate and
accounted for in `docs/domain-primitive-baseline.txt`. New public primitive
exposures are blocked by `scripts/check-domain-types` unless the baseline is
updated in the same change with a clear reason in the review.

Prefer domain types that encode invariants at construction time. Use
`new`/`try_new`, parsers, or validators to reject invalid values before they
enter core logic. Do not smuggle invalid states through default empty strings,
sentinel integers, or loosely typed byte buffers.

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

Development is test-driven by default.

For new behavior, write or update the smallest meaningful test first, confirm
that it fails for the intended reason when practical, then implement the
production change and keep the test as a regression case. Refactors must keep
the relevant tests passing before and after the change.

When changing existing behavior that lacks direct coverage, add the missing
test before changing the implementation. Do not extend decode, lift, IR, emit,
runtime, oracle comparison, or CLI behavior without a corresponding test unless
the change is documentation-only or purely mechanical formatting.

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

For ordinary code, script, policy, or configuration changes, run:

```sh
nix develop -c ./scripts/verify
```

For dependency, lockfile, `deny.toml`, or `flake.nix` changes, also run:

```sh
nix develop -c ./scripts/verify-supply-chain
```

For security-relevant changes, also run:

```sh
nix develop -c ./scripts/verify-security
```

## Review Checklist

Before completing a change, check:

- Does the signature explain inputs, outputs, failure, ownership, and address
  space?
- Are raw `u64`, `usize`, or `Vec<u8>` values crossing boundaries without
  domain meaning?
- Did `scripts/check-domain-types` pass, and did any baseline update have a
  domain-level justification?
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
- Did `nix develop -c ./scripts/verify` pass for code/script/policy/config
  changes?
- Was new or changed behavior driven by a test first?
- Did supply-chain checks pass when dependencies, lockfiles, or toolchain
  settings changed?
- Did invisible/control Unicode checks pass?
- Are EditorConfig and language formatter responsibilities respected?
- Does internal mutation preserve external purity?
- Is `unsafe` absent from core logic and documented at runtime boundaries?
- Are clean-room boundaries preserved?
