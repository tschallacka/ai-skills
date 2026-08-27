<!-- MODE: PROD -->
# Resource-limited testing

**Big commands, capped.**

Some commands eat a machine: full test suites, builds, analyzers, headless
browsers. This skill runs them under a resource cap so "run the tests" never
means "freeze my laptop" or "get the CI job OOM-killed".

## What you get

- **One wrapper, right cap.** `limited-run.sh <memory> <cpu> -- <command>`
  picks the platform-appropriate mechanism: cgroups on Linux, `memlimit` +
  `cpulimit` on macOS, and a graceful `nice`-only degradation when neither is
  available — it tells you exactly which cap you got.
- **Failures that mean something.** A run that dies at the cap reports it in
  plain words instead of masquerading as a flaky test.
- **Honest degradation.** Where a platform has no cap mechanism, the wrapper
  says so instead of pretending.

## Quick start

> Run the full suite under a cap.

Behind the scenes:

```sh
limited-run.sh 6G 400 -- ./run-tests.sh
```

## Good to know

Lightweight commands don't need this — `ls` does not require supervision.
Use it when the command could plausibly take the machine down with it.
