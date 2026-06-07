---
name: bara-status
description: Use when the user explicitly selects the Bara /status agent action to report repository status without modifying files.
---

# Bara /status

Treat this skill invocation as if the user typed Bara's repository `/status`
action, not the Codex IDE built-in `/status` command.

Do not modify files. Follow `AGENTS.md` and the README agent action command
section to report the current branch state, worktree status, active TODO
milestone, relevant design TODOs, and recommended next action.
