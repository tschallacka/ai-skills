---
name: planning
description: Use when the user requests a durable plan or a multi-step initiative genuinely needs resumable files, ordered goals and steps, verification instructions, progress trackers, or handoff notes. Do not use for small, self-contained changes or temporary in-chat checklists.
---
<!-- MODE: PROD -->

# Planning

Use this skill to turn an initiative into a directory of Markdown files
that another agent can resume and execute without reconstructing missing
context. Do not use it for a small, self-contained change or a temporary
in-chat plan.

The full skill is generated from a single authored source
(`skill-source.txt`) into this short index plus the parts below, because
the whole thing is 89,860 bytes and at least one harness this repo runs
under silently truncates a file read past roughly 25,000 tokens with no
notice anywhere (`.agents/knowledge/agent-read-limits.md`). Read the part
that applies to what you are doing now; each is well under that budget on
its own.

| Part | Covers |
|---|---|
| [parts/part-1.md](parts/part-1.md) | Setup, operating rules, tool/context-limit discipline, hard planning gates, establishing the plan boundary |
| [parts/part-2.md](parts/part-2.md) | Creating the plan directory |
| [parts/part-3.md](parts/part-3.md) | Mandatory classification and independent review |
| [parts/part-4.md](parts/part-4.md) | Resuming and updating a plan |

Every part carries a load-sanity check (T86): a hidden line at a random
position near its end, and a command (`planning/scripts/verify-skill-load.sh
with --part <N> --token <token>`) that must succeed before the part counts
as read. Stating a token from memory or from this index is not that command
succeeding; the check exists because reporting a token is claimable and
running the command against the real file is not.
