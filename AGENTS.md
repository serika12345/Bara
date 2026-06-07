# AGENTS.md

This file is the operational rulebook for coding agents working on Bara.
Follow it together with the repository documentation, especially
`docs/coding-rules.md`, `docs/clean-room.md`, `docs/scope.md`,
`docs/ir.md`, `docs/test-oracle.md`, `TODO.md`, and
`docs/design-todo.md`, and `docs/progress.md`.

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

Install the local pre-commit hook when working in a clone:

```sh
nix develop -c ./scripts/install-pre-commit-hook
```

The hook runs the fast repository integrity checks before commit. It does not
replace `nix develop -c ./scripts/verify`.

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

## Roadmap and Design TODO Discipline

Before starting implementation, refactoring, or architecture work, read the
current `TODO.md` and `docs/design-todo.md` entries that are relevant to the
requested task.

Use these files for different responsibilities:

- `TODO.md` tracks implementation milestones and large project goals.
- `docs/design-todo.md` tracks detailed design decisions, refactoring
  boundaries, decomposition plans, and single-responsibility audit notes.
- `docs/progress.md` tracks completed milestones, current project state, and
  major direction changes so project history can be understood without reading
  git history.

When selecting the next task, prefer the earliest unfinished implementation
milestone in `TODO.md` unless the user names a different milestone or asks for
design/refactoring work. When the task is refactoring, module splitting, API
boundary cleanup, or architecture review, consult `docs/design-todo.md` first
and keep those changes separate from feature work where practical.

When a change completes, invalidates, or materially changes a roadmap item,
update the appropriate TODO document in the same change:

- implementation progress belongs in `TODO.md`
- design decisions and decomposition notes belong in `docs/design-todo.md`
- completed milestone summaries and project-state changes belong in
  `docs/progress.md`
- completed historical milestone details may remain in focused roadmap docs,
  such as `docs/hello-world-roadmap.md`

The documentation state must match the implementation state. Do not leave a
TODO item marked incomplete when the implementation and verification for that
item are complete. Do not mark a TODO item complete until the implementation,
tests or fixtures, and required verification have actually been completed.

Do not implement work that is not represented by a current TODO milestone,
design TODO, or focused roadmap entry. If the work is not already tracked,
first add or refine the milestone, then split it into the smallest coherent
implementation step.

Project progress should be understandable from documentation alone. Agents must
not rely on git history as the only record of what happened or why. When a
milestone is completed or the project direction changes, add a concise entry to
`docs/progress.md` that records the state reached, the verification performed,
and the next intended direction.

Do not mix broad feature implementation and unrelated refactoring merely to
clear TODO entries. If a refactor is required to make the feature safe, keep it
as a clearly bounded preparatory step and mention the relevant design TODO.

## Agent Implementation Workflow

Agents should advance Bara by repeating a small, auditable implementation
cycle. The user should not need to restate this process in every session.

Default cycle:

1. Read the relevant `TODO.md` milestone and `docs/design-todo.md` design
   notes.
2. Pick the smallest coherent task that moves the current milestone forward.
3. Identify whether the task is feature work, refactoring, design-only
   documentation, or verification.
4. If design or refactoring is needed before feature work, do it as a bounded
   preparatory step and keep its diff separate where practical.
5. Add or update the smallest meaningful test or fixture first, unless the
   change is documentation-only or purely mechanical.
6. Implement the production change using the existing crate/module boundaries.
7. Audit the resulting code for single responsibility, module size, domain
   type boundaries, I/O isolation, and clean-room compliance.
8. If the implementation grew too large or mixed responsibilities, split it
   before moving on.
9. Update `TODO.md`, `docs/design-todo.md`, or focused roadmap docs when the
   change affects roadmap state or design decisions. Update
   `docs/progress.md` when a milestone or major project state changes.
10. Run the required Nix-based verification gate.
11. Summarize what changed, what was verified, and what remains.

When the user asks to "continue", "go next", "advance to the next milestone",
or similar, use this default cycle without asking for process clarification.
Prefer the earliest unfinished relevant milestone in `TODO.md`, unless the
user explicitly names a milestone or changes priority.

Large milestones must be split before implementation. Do not attempt to finish
a broad milestone in one unstructured change. Create or update TODO entries
when the split itself changes the plan.

When multi-agent tooling is available and the task is non-trivial, the primary
agent should delegate bounded implementation work to a sub-agent and then
review the returned code. The primary agent remains responsible for the final
integration and must audit:

- whether each module has a single reason to change
- whether any file or function became too large
- whether a DRY abstraction mixed unrelated responsibilities
- whether public APIs expose primitives where domain types are required
- whether I/O, toolchain calls, or process execution leaked into core logic
- whether tests cover the changed behavior

If the sub-agent result violates these rules, revise it directly or send it
back as a smaller, clearer task before accepting it.

## Milestone Branch Workflow

When the user asks to advance until a small or large milestone is reached, use
a dedicated work branch unless the user explicitly says otherwise.

Branch workflow:

1. Start from the current base branch, normally `main`.
2. Create a task branch before implementation.
3. Advance TODO-backed small steps on that branch.
4. Commit autonomously on the task branch after each coherent verified step.
5. Push the task branch after commits when network access is available.
6. Continue until the requested small or large milestone is reached, or until a
   review gate or blocker is reached.
7. When a large milestone is complete, stop and return a review package with
   the branch already committed and pushed.

Large milestone completion is a review gate. Do not merge back to `main`
autonomously. After user review, if the user approves, merge the branch into
`main`, clean up the branch if appropriate, update progress documentation, and
continue to the next milestone only when instructed.

On a dedicated task branch, autonomous commits and pushes are allowed when all
of these are true:

- the work is on a dedicated task branch, not `main`
- the commit contains one coherent TODO-backed step
- required verification for that step has passed, or the failure is documented
- TODO, progress, and design documents match the implementation state
- the commit does not include unrelated user changes

Do not autonomously commit or push on `main`. For non-milestone tasks, such as
documentation updates, small policy edits, one-off explanations, or exploratory
changes, do not create a branch and do not commit unless the user explicitly
asks. These tasks usually happen on `main`; return the uncommitted diff for
review unless instructed otherwise.

Review package at a milestone stop:

- branch name and latest commit
- completed TODO items
- changed files summary
- verification commands and results
- design or refactoring decisions made
- remaining risks or review points
- recommended next milestone

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

For Nix/package changes, the local gate includes:

```sh
nix develop -c ./scripts/verify-nix-package
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

- Did the relevant `TODO.md` and `docs/design-todo.md` entries guide the work,
  and were they updated if the change affected them?
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
