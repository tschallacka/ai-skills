<!-- MODE: PROD -->
# Roles — index (router)

The canonical registry (all role ids, names, and full authority boundaries)
lives in the maintainer contract (`MAINTAINER-STYLE-CONTRACT.md`), because
it is internal and must not load future-phase knowledge into an agent's
context.

Load the role doc for the phase you are in:

- **Planning** → `roles/planning.md` (Alex, Benny, Chris/Christian/Christoph)
- **Execution** → `roles/execution.md` (Alex, Benny, Chris/Christian/Christoph, Dana)
- **Cleanup** → `roles/cleanup.md` (Frank, Alex, Willie)
- **Benchmark harness** → maintainer contract (Oracle/Pythia, Analyzer/Eve)

Rules:
- A role's `name` (Alex, Benny, Chris, Christian, Christoph, Dana, Frank, …)
  is only meaningful alongside its canonical id; documents must use the id.
- Phase docs intentionally omit roles that belong to other phases, so a
  planning agent is never exposed to execution/cleanup roles.
- Domain-gated roles (benchmark Oracle / Analyzer) are not part of the
  end-user planning lifecycle.

## Persona doc matrix

Each persona's scope is the explicit set of documents it may read. This matrix
is a **maintained mirror** of `planning/scripts/role-context.sh`'s hardcoded
`role_docs()` (which is the authoritative source the registry consumes); it also
feeds the per-role reader allow-list below. Every document named in this matrix
MUST exist under `planning/`; the persona-drift test
(`planning/tests/test-persona-drift.sh`, goal 05) asserts registry : shipped
scope docs stay in sync.

| Canonical id | Canonical name | Reader docs (role-context scope) |
|---|---|---|
| `alex` | Alex | `SKILL.md`, `ROLES.md`, `roles/planning.md`, `roles/execution.md` |
| `benny` | Benny | `ROLES.md`, `roles/planning.md`, `roles/execution.md` |
| `chris` | Chris | `ROLES.md`, `roles/planning.md`, `REVIEWER.md` |
| `christian` | Christian | `ROLES.md`, `roles/planning.md`, `REVIEWER.md` |
| `christoph` | Christoph | `ROLES.md`, `roles/planning.md`, `REVIEWER.md` |
| `dana` | Dana | `ROLES.md`, `roles/execution.md` |
| `frank` | Frank | `ROLES.md`, `roles/cleanup.md` |
| `maintainer` | Willie | `MAINTAINER-STYLE-CONTRACT.md`, `ROLES.md`, `roles/planning.md`, `roles/execution.md`, `roles/cleanup.md` |
| `installer` | Felix | `ROLES.md`, `MAINTAINER-STYLE-CONTRACT.md` |
| `oracle` | Pythia | `MAINTAINER-STYLE-CONTRACT.md` |
| `eve` | Eve | `MAINTAINER-STYLE-CONTRACT.md` |

## Per-role reader allow-list

`role-context.sh` (persona scope docs) and `plan-context.sh` (bounded plan
content) are two distinct gates. Each role explicitly names which gate(s)
apply to its spawn, and the combined byte budget is declared and capped. There
is no global composition rule; composition is per-role only. `plan-context.sh`
budget is the per-read cap (default `32768` bytes in `plan-context.sh`);
`role-context.sh` budget is the role-context page budget (default `12000`
bytes/page in `role-context.sh`).

| Canonical id | role-context.sh? | plan-context.sh? | Combined budget cap (bytes) |
|---|---|---|---|
| `alex` | yes | yes (plan view) | 32768 + role pages |
| `benny` | yes | yes (plan view) | 32768 + role pages |
| `chris` | yes | yes (plan view) | 32768 + role pages |
| `christian` | yes | yes (plan view) | 32768 + role pages |
| `christoph` | yes | yes (plan view) | 32768 + role pages |
| `dana` | yes | yes (plan view) | 32768 + role pages |
| `frank` | yes | yes (plan view) | 32768 + role pages |
| `maintainer` | yes | yes (monitor frame) | 32768 + frame budget |
| `installer` | yes | no | role pages only |
| `oracle` | yes | no | role pages only |
| `eve` | yes | no | role pages only |

Composition rule: when both gates apply, the persona's effective context is
the declared `plan-context.sh` cap PLUS its `role-context.sh` role pages,
delivered as bounded, paginated slices (never an unbounded whole). When only
`role-context.sh` applies, the persona reads only its role scope docs and no
plan content.

Willie (`maintainer`) uses a supervision-only composition: as the monitor it
reads only bounded supervision frames (via `planning/scripts/monitor-read.sh`,
pull-on-exception), plus its maintainer scope docs (`MAINTAINER-STYLE-CONTRACT.md`,
`ROLES.md`) and `REVIEWER.md` for the review contract — never unbounded logs or
the whole plan tree. Grants are logged as case + handed command, never
reasoning.
