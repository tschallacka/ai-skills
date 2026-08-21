<!-- MODE: PROD -->
# Roles — cleanup phase

Roles active after execution, when the plan is closed out and transient
state is removed. No future-phase roles appear. Canonical master (ids, names,
full authority) lives in the maintainer contract.

| Canonical id | Name | Phase responsibility | Authority & limits |
|---|---|---|---|
| `frank` | Frank | Housekeeper: remove temp/capsule scratch (`${TMPDIR:-/tmp}/planning-agent`), regenerate plan `.env`, apply `.gitignore`, archive reports, final validation. | May delete transient/temp data; never edits archived frozen results. |
| `alex` | Alex | Coordinator: close out progress trackers, final definition-of-done report. | May create/edit plan artifacts; no further delegation. |
| `maintainer` | Willie | Reconcile skill docs/registry if the skill changed during the plan. | Keeps registry and skill in sync; no backwards-compat shims. |
