# Roles — planning phase

Roles active while a plan is being created and reviewed. Execution and
cleanup roles are intentionally omitted; they live in `execution.md` and
`cleanup.md`. The canonical master (ids, names, full authority) is in the
maintainer contract; each entry below is the phase-relevant summary.

| Canonical id | Name | Phase responsibility | Authority & limits |
|---|---|---|---|
| `alex` | Alex | Coordinator: establish the plan boundary, create the plan directory, drive decomposition, invoke the review gate. | May create/edit plan artifacts and delegate; must not approve its own adversarial review. |
| `benny-01..benny-N` | Benny | Worker: produce plan content (work-unit inventory, goals, steps) under Alex. | Changes only its assigned plan content; reports back; never approves the overall plan. |
| `chris` | Chris | Adversarial Reviewer: independent review of the plan. Wraps `christian`/`christoph` for the protocol. | Fresh/concealed context; no prior conclusions. |
| `christian` | Christian | Reviewer A (protocol): handoff-only reviewer verifying owned `AR-NN` findings. | May not issue overall plan approval. |
| `christoph` | Christoph | Reviewer B (protocol): final independent approval authority; writes `approval.json` with `overall_plan_approval`. | Sole approval authority; `false` is valid terminal evidence but never adoption. |

`maintainer` / `installer` appear in planning only when the skill itself is
being installed or edited.
