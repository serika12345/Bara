# Supply Chain Policy

Bara keeps dependency risk visible and mechanically checked.

## Rules

- Run supply-chain commands through the Nix dev shell.
- Keep `Cargo.lock` committed and current.
- Do not add dependencies without a test-backed need and a domain-level reason.
- Prefer small, mature crates with clear licenses and active maintenance.
- Do not use Git dependencies unless there is no crates.io release and the
  dependency is explicitly reviewed.
- Do not accept new advisories, yanked crates, duplicate crate versions, unknown
  registries, or unreviewed licenses silently.
- Do not weaken `deny.toml` to make a dependency pass. If an exception is
  unavoidable, record the narrow exception with a reason in the same change.

## Required Check

Run:

```sh
nix develop -c ./scripts/verify-supply-chain
```

This verifies:

- `cargo metadata --locked --format-version 1`
- `scripts/verify-cves`
- `cargo deny check`

## Dependency Changes

For dependency additions or updates:

1. Add or update the test that requires the dependency.
2. Update `Cargo.toml` and `Cargo.lock`.
3. Run `nix develop -c ./scripts/verify-supply-chain`.
4. Run the normal format and test commands.
5. Review license, advisory, duplicate-version, and source-registry output.

Dependency changes are incomplete unless both the behavioral tests and the
supply-chain checks pass.

## CVE Baseline

Known RustSec advisories are tracked in
`scripts/cve-audit-baseline.json`. The baseline must stay narrow:

- New advisories fail `scripts/verify-cves`.
- Stale baseline entries fail `scripts/verify-cves`.
- Baseline additions require a reason in review and should be removed as soon
  as dependencies can be updated.

Run:

```sh
nix develop -c ./scripts/verify-cves
```

## Nix Package Verification

Bara exposes a default Nix package. Package verification runs:

```sh
nix develop -c ./scripts/verify-nix-package
```

This checks the flake and builds `.#default`, using `Cargo.lock` through Nix.

## Hidden Text Attacks

Git-tracked files must not contain suspicious invisible/control Unicode
characters such as zero-width spaces, Unicode format characters, or BiDi
override characters.

Run:

```sh
nix develop -c ./scripts/check-no-invisible-chars
```

Security-relevant changes should use the aggregate check:

```sh
nix develop -c ./scripts/verify-security
```
