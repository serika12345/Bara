# Public ABI / import boundary

## Purpose / scope

This document is the HW13 planning slice for public ABI, import, host helper,
and syscall-like boundaries. It records the clean-room boundary before adding
implementation.

The scope is deliberately narrow:

- keep the current raw function, Bara executable manifest, and host trap design
  coherent while preparing for larger executable inputs
- separate guest-visible declarations from host execution responsibilities
- define vocabulary for future tests and reports
- document allowed information sources before implementation starts

This is not Wine integration, syscall emulation, dynamic loader work, or libc
ABI reproduction. It also does not define final Rust APIs. The names below are
domain concepts that future Rust types may use only when a test-backed slice
needs them.

The current concrete starting point is the Bara executable manifest import:

- host helper name: `write_stdout`
- host helper signature: `ptr_len_to_unit`
- current use: a manifest `host_traps` stdout plan must declare the helper
  import before the manifest is accepted

## Clean-room source policy

Allowed implementation sources for this boundary:

- Intel/AMD ISA manuals
- ARM Architecture Reference Manual
- System V ABI
- Windows x64 ABI
- public Mach-O / PE / ELF specifications
- public OS API documentation
- self-authored fixtures and tests
- Rosetta black-box observations limited to externally visible behavior:
  exit status, stdout, stderr, return value, explicit harness JSON, and crash
  status
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

Host helper names, import tables, trap plans, and unsupported classifications
must be Bara domain concepts or public-spec concepts. They must not be derived
from private translation-layer internals.

## Boundary responsibilities by crate / layer

`bara-oracle`:

- own testcase and executable manifest domain values
- parse serialization boundary data into typed declarations
- validate fixture and manifest declarations, including required helper imports
  for declared host traps
- compare `expected.json` and `actual.json` as externally observable behavior
- record unsupported boundary classifications in reports
- keep Rosetta usage as black-box oracle execution only
- not execute runtime host calls or own executable memory behavior

`bara-ir`:

- represent pure program semantics and boundary intent after lift
- carry only typed concepts needed by decode, lift, validation, and emit
- allow future host-call or unsupported-boundary nodes only as pure values
- not parse files, resolve imports, call host helpers, run syscalls, or depend
  on Rosetta behavior

`bara-isa-x86`:

- decode and lift x86_64 instructions using public ISA semantics
- classify unsupported external boundary forms, such as syscall-like
  instructions or unsupported call shapes, without executing them
- describe guest-side mechanisms in x86 terms, not host runtime terms
- not parse executable import tables, resolve host helpers, or encode oracle
  behavior

`bara-arm64`:

- emit ARM64 code from typed IR and emit plans
- preserve the chosen runtime calling boundary when such a plan exists
- report unsupported emit boundary cases as classified errors
- not decide import resolution, syscall behavior, fixture policy, or oracle
  expectations

`bara-runtime`:

- own executable memory, protection changes, generated-code invocation, and
  other unsafe runtime boundaries
- execute only a typed host trap plan or host-call plan handed to it by a
  validated boundary layer
- produce externally observable actual results such as stdout, stderr, return
  value, crash status, and classified runtime error
- not parse manifests, inspect file formats, invoke Rosetta, or infer guest OS
  semantics

`btbc-cli`:

- act as the file, JSON, process, and report I/O boundary
- call parsers, validators, compiler stages, runtime execution, and comparison
  in explicit order
- surface classified unsupported boundary results without inventing semantics
- not hide global ABI, import, or syscall policy in CLI-only state

Fixture / corpus:

- contain self-authored inputs, public-format samples, expected JSON, and stable
  unsupported expectations
- declare public ABI profile, imports, helper declarations, host traps, and
  future syscall expectations explicitly
- store only externally observable Rosetta results, never Rosetta internals
- keep helper declarations such as `write_stdout` / `ptr_len_to_unit` as Bara
  fixture vocabulary

## Domain vocabulary

These names are planning vocabulary, not required Rust API names.

`public ABI profile`:

- a named guest-visible ABI surface selected from public specifications or Bara
  fixture policy
- examples: raw no-args `rax` return, one `u64` argument, pointer plus length
  helper convention, future System V or Windows x64 entry conventions
- does not imply host OS behavior or loader behavior

`guest import declaration`:

- a declaration that guest code expects an external boundary to exist
- may later describe a symbol, ordinal, syscall ABI, or Bara manifest import
- remains unresolved until a separate plan accepts, rejects, or classifies it

`host helper declaration`:

- a Bara-defined, explicit helper available at the host boundary
- current example: `write_stdout` with `ptr_len_to_unit`
- exists for testable host interaction before real OS or libc boundaries

`syscall request`:

- a guest attempt to enter an OS/kernel ABI according to public ISA or ABI
  rules
- initially classified as unsupported unless a later slice defines exact
  behavior from public specifications and tests

`host trap plan`:

- a typed plan that the runtime may execute to produce host-observable effects
- current example: stdout text captured in `actual.json`
- should be produced by validated fixture/manifest data, not by ad hoc runtime
  inspection

`import resolution plan`:

- a pure mapping from guest import declarations to supported host helpers,
  unsupported classifications, or future runtime host-call entries
- belongs before runtime execution so unsupported boundaries are reported
  deterministically

`unsupported boundary classification`:

- a stable reason explaining why an import, syscall, helper, ABI profile, or
  call boundary is not executable yet
- should be precise enough for corpus expectations and regression tests

## Native stdout emission boundary

The current stdout path is intentionally a Bara host-helper path, not guest
syscall execution and not libc emulation.

The boundary is:

1. `bara-ir` records stdout as `HostTrapKind::Stdout` and maps that host trap
   to `HostHelperRequest::WriteStdout` with `HostHelperAbi`:
   `write_stdout(ptr_len_to_unit)`.
2. `bara-oracle` executable manifests declare and resolve the
   `write_stdout` / `ptr_len_to_unit` host helper before execution. A manifest
   that requests stdout without declaring the helper is invalid.
3. `btbc-cli` consumes the resolved manifest helper in executable preflight
   and checks it against the IR-level `HostHelperAbi` before running the
   function pipeline.
4. `bara-runtime` may execute a typed `HostTrapPlan` and expose stdout in the
   observed result. The runtime receives the plan from validated fixture or
   manifest data; it does not parse manifests or infer guest OS behavior.
5. Standalone native artifact packaging may convert the accepted
   `write_stdout(ptr_len_to_unit)` helper requirement into host-native stdout
   emission. The current macOS ARM64 artifact path does this by generating a
   packaging prologue that calls the public `_write` symbol with file
   descriptor `1`, a generated stdout buffer pointer, and the buffer length,
   then continues into the generated ARM64 function body.

The conversion from Bara host helper to native stdout belongs at the output
artifact packaging boundary. It must not leak into x86 decode, lift, core IR,
ARM64 instruction emission, manifest parsing, or oracle comparison.

This path has these current limits:

- `write_stdout` means a Bara-defined host effect capability, not a guest
  `write` syscall and not `puts`.
- Native emission is currently a macOS ARM64 standalone artifact strategy.
  Linux, Windows, and future object formats must add explicit output adapters
  behind the same helper boundary instead of changing core IR semantics.
- Output artifact packaging selects native stdout emission by target OS ABI.
  The current implemented strategy is `arm64-apple-macos` using the public
  `_write` symbol. Linux and Windows target triples are represented as explicit
  unsupported stdout emission targets until their adapters are defined.
- Unsupported helper, ABI, platform, or artifact combinations must remain
  classified before execution.

## First implementation sequence after this document

1. Typed import declaration planning

   Keep the existing manifest behavior intact and make future import concepts
   explicit in tests and reports before expanding runtime behavior.

2. Unsupported syscall classification

   Add the smallest public-ISA-based recognition needed to report syscall-like
   boundary attempts as unsupported. Do not emulate the OS call.

3. Helper import validation

   Preserve the current rule that stdout host traps require the
   `write_stdout` / `ptr_len_to_unit` helper declaration. Extend validation only
   with test-backed domain reasons.

4. Import resolution plan

   Introduce a pure plan that maps validated declarations to supported helper
   plans or unsupported classifications. Keep file I/O and runtime execution
   outside this step.

5. Runtime host-call execution

   Only after typed declarations, validation, and resolution are stable, allow
   the runtime to execute a selected host-call plan. Keep unsafe code localized
   in runtime boundaries.

6. Public executable import slices

   Later, parse public-format import data from Mach-O, PE, or ELF in small
   slices. Convert it into guest import declarations before any execution work.

## Non-goals for now

- dynamic loader behavior
- real OS syscall execution or emulation
- libc ABI reproduction
- Wine integration
- private Apple ABI usage
- Rosetta internals
- copying FEX-Emu, Box64, or QEMU internals
- whole-process emulation
- relocation processing as part of this slice
- deciding final Rust API names before tests require them
