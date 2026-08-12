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
