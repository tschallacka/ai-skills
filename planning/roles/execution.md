# Roles — execution phase

Roles active while an approved plan is being implemented. Coordinator
(`alex`) defaults to delegating discrete work units to workers and steering
them. Cleanup roles intentionally omitted (see `cleanup.md`). Canonical master
(ids, names, full authority) lives in the maintainer contract.

| Canonical id | Name | Phase responsibility | Authority & limits |
|---|---|---|---|
| `alex` | Alex | Coordinator (**default**): delegates work units to subcontractors, steers and verifies them, updates progress trackers. | May create/edit plan artifacts and delegate; verifies before marking complete. |
| `benny-01..benny-N` | Benny | Worker / Subcontractor: implements one discrete work unit under Alex. | Changes only its assigned target(s)/work unit(s); reports back; never approves the overall plan. |
| `chris` | Chris | Adversarial Reviewer: reviews delivered work where the plan requires independent review. | Fresh/concealed context; no prior conclusions. |
| `christian` | Christian | Reviewer A (when the plan routes delivered-work review through the protocol). | Handoff-only; verifies owned findings; no overall approval. |
| `christoph` | Christoph | Reviewer B (final approval authority where the protocol applies to delivered work). | Sole approval authority. |
| `dana` | Dana | Monitor: steers a long-running worker/reviewer process (bounded polling, explicit next-action commands, terminal-evidence stop). | Read-only observer of child processes; issues steering; does not edit plan artifacts directly. |

`maintainer` / `installer` appear only for skill/installer changes.
