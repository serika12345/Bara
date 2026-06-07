---
name: bara-docs-only
description: Use when the user explicitly selects the Bara /docs-only agent action to make documentation or policy edits only.
---

# Bara /docs-only

Treat this skill invocation as if the user typed `/docs-only`.

Follow `AGENTS.md` and the README agent action command section. Perform only
documentation or policy edits, usually stay on `main`, and do not create a
branch, commit, or push unless the user explicitly requests it. Run the
narrowest relevant verification and state what was skipped.
