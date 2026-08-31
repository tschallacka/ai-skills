# Progress: 01-engine-core

**Progress:** `0%  --------------------  100%` 💤

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 01-engine-core | 01-step-read-plan-tree | Read a plan directory into one owned in-memory structure, so every later stage works from a parsed t... | 💤 incomplete |
| 01-engine-core | 02-step-parse-state | Turn the state document into typed values so pages read fields rather than re-deriving them from tex... | 💤 incomplete |
| 01-engine-core | 03-step-derive-counts | Compute every count and percentage the pages present, in one place, so two surfaces cannot disagree ... | 💤 incomplete |
| 01-engine-core | 04-step-derive-geometry | Derive the ring and donut geometry from the counts so the circumference and offsets exist once rathe... | 💤 incomplete |
| 01-engine-core | 05-step-test-parse-and-derive | Pin the parse and derive contract against a fixture so a later change cannot quietly alter a number ... | 💤 incomplete |
| 01-engine-core | 06-step-verify-size-fixture | Prove on the plan that currently cannot render at all that the new path has no argument-length limit... | 💤 incomplete |
| 01-engine-core | 07-step-crate-manifest | Create the crate manifest that owns the renderer: package name, the release profile, and no dependen... | 💤 incomplete |
| 01-engine-core | 08-step-crate-root | The crate root: the module declarations that make plan, render, pages, serve and watch reachable, an... | 💤 incomplete |
| 01-engine-core | 09-step-cli-surface | The command-line surface the binary exposes: --plan-dir, --out, --refresh, --watch, --serve and --po... | 💤 incomplete |
| 01-engine-core | 10-step-state-extraction | Produce the state document's values inside the crate rather than by invoking overview-state.sh, so t... | 💤 incomplete |
