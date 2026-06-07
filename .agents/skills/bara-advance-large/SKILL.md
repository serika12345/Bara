---
name: bara-advance-large
description: Use when the user explicitly selects the Bara /advance-large agent action to advance the current large milestone to completion on a dedicated branch.
---

# Bara /advance-large

Treat this skill invocation as if the user typed `/advance-large`.

Follow the exact workflow in `AGENTS.md` and the README agent action command
section: create a dedicated work branch, advance TODO-backed small steps,
commit and push coherent verified steps, and stop at the large milestone
review gate with a review package. Run required verification through the Nix
dev shell.
