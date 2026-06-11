---
name: bara-advance-pr
description: Use when the user explicitly selects the Bara /advance-pr agent action to advance to the next TODO.md PR Gate on a dedicated branch.
---

# Bara /advance-pr

Treat this skill invocation as if the user typed `/advance-pr`.

Follow the exact workflow in `AGENTS.md` and the README agent action command
section: find the next unfinished `PR Gate` in `TODO.md`, create or continue
that gate's dedicated branch, complete only that gate's criteria, run required
verification through the Nix dev shell, commit and push coherent verified
steps, open a draft pull request, then stop with a review package. Do not
continue into the next `PR Gate` automatically.
