# Plan read contract

Use this contract whenever the planning skill handles, reads, or inspects plan
artifacts — plan documents, goal/step files, progress trackers, work-unit
inventories, adversarial review records, or any file under a plan directory
(`.plans/<plan>/`). It applies to the **main planning agent** and to **every
fresh subagent** it spawns (adversarial review, reviewer, monitor, cleanup).

The key terms below use RFC 2119 meaning: **MUST** = absolute requirement,
**MUST NOT** = absolute prohibition, **SHOULD** = recommendation.

## Scope

| Contract clause | Requirement |
|---|---|
| 1. Gated reads only | Plan reads **MUST** go through the gated reader (`plan-context.sh`). The main agent and every subagent **MUST** use `plan-context.sh read --plan-dir <PLAN_DIR> --document ID` (or `--unit WNN`) for any plan content. |
| 2. No whole-file reads | Agents **MUST NOT** read plan artifacts with `cat`, a native `Read`/file tool, `head`/`tail`, or any tool that loads an entire plan file, an entire plan directory, or the `.plans/` tree wholesale. A wholesale plan read is a **context-overflow violation**. |
| 3. No bypass | Agents **MUST NOT** bypass the gate when it cannot serve a document — they **MUST** report it as a limitation instead of falling back to a raw file read. |
| 4. Subagents locked | Every spawned subagent's starting prompt **MUST** include the verbatim bounded-read lock (SKILL.md §3) **and** the no-skill-load clause (SKILL.md Operating rules). |
| 5. Byte-bounded | Plan reads **SHOULD** prefer the default summary view and honor `--max-records`/`--max-bytes`; when `ROLE_ID` is set, the gate caps `--max-bytes` to the per-role budget, and agents **MUST NOT** work around that cap. |
| 6. Read discipline reported | Subagents **MUST** state in their returned findings that every plan read went through the gate and list any wholesale read they performed, so a violation is visible (soft audit). |

## Why

The gated readers strip plan metadata and metadata-bearing front-matter that is
not needed to act, and they page/bound the output. Reading a whole plan file or
directory floods agent context, sidesteps the per-role allow-list, and defeats
the bounded-context design. A whole-file read of a plan artifact is treated the
same way a context-overflow violation is: fail loudly, do not silently recover.

## Enforcement

- The main agent follows clauses 1–3, 5 for its own plan reads.
- The main agent inserts clauses 1–6 into every subagent's starting prompt via
  the SKILL.md §3 bounded-read lock.
- The soft-audit clause (6) makes a violation visible in returned findings so
  it can be corrected rather than silently absorbed.
