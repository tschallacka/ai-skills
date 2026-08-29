<!-- MODE: PROD -->
# Project-specific deviations

**The quirks map, so nobody rediscovers them the hard way.**

Every project lies a little: the script that only fails on NFS, the test that
needs `LC_ALL=C`, the tool that isn't where its docs say. This skill keeps
those confirmed deviations in per-project notes that future agents load
instead of re-debugging.

## What you get

- **A deviations note per project.** `<project>-deviations.md` records
  behavior that deviates from the normal defaults: environment quirks,
  non-obvious conventions, tooling gotchas — each one *confirmed*, not
  suspected.
- **Loaded when it matters.** When implementation, debugging, testing, or
  tooling touches the project, the matching note is pulled in before the
  mistake is repeated.
- **Written once, by evidence.** A deviation is recorded after it is
  confirmed against reality — a hunch with no reproduction stays out.

## Quick start

> Note a deviation: this repo's `make test` writes into `/tmp` and fails if
> TMPDIR points at a mounted volume.

Next session, the note is there before the failure is.

## Good to know

This is for what *deviates*. General documentation, changelogs, and behavior
that matches the project's normal defaults belong elsewhere.
