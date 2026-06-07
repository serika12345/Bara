---
name: bara-merge-reviewed
description: Use when the user explicitly selects the Bara /merge-reviewed agent action after giving review approval to merge the reviewed branch into main.
---

# Bara /merge-reviewed

Treat this skill invocation as if the user typed `/merge-reviewed`.

Only proceed if the user's prompt gives explicit review approval to merge.
Then follow `AGENTS.md` and the README agent action command section: merge the
reviewed work branch into `main`, update progress documentation if needed,
clean up the branch when appropriate, and run required verification through the
Nix dev shell. If approval is not explicit, stop and ask for approval.
