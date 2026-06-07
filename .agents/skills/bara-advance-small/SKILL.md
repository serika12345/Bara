---
name: bara-advance-small
description: Use when the user explicitly selects the Bara /advance-small agent action to advance the next small TODO-backed step on a dedicated branch.
---

# Bara /advance-small

Treat this skill invocation as if the user typed `/advance-small`.

Follow the exact workflow in `AGENTS.md` and the README agent action command
section: create or continue a dedicated task branch, complete the next coherent
TODO-backed step, run required verification through the Nix dev shell, commit
and push the verified step, then stop with a concise review package.
