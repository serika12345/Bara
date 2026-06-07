---
name: bara-continue-branch
description: Use when the user explicitly selects the Bara /continue-branch agent action to keep working on the current dedicated task branch.
---

# Bara /continue-branch

Treat this skill invocation as if the user typed `/continue-branch`.

Follow the exact workflow in `AGENTS.md` and the README agent action command
section: continue the current dedicated task branch, choose the next coherent
TODO-backed step, run required verification through the Nix dev shell, commit
and push coherent verified work, and summarize the result.
