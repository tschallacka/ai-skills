# Goal: Exclude local manifests from published archives

## Current state and prior-goal handoffs

Plan-local `.env` files are hidden files under benchmark workspaces, while
publication copies the workspace with `cp -R .../.`; no explicit exclusion or
test protects the archive boundary.

## Outcome and definition of done

Published result archives never contain plan-local or global `.env` manifests,
while ordinary plan evidence remains complete.

## Why this goal is needed

Environment files are local execution metadata and must not become benchmark
artifacts or leak paths into published evidence.

## Scope

Include publication filtering and archive-integrity tests. Exclude deleting
manifests from active workspaces.

## Affected files, systems, data, and interfaces

W74 updates `benchmark/planning/setup-benchmark.sh`. W75 extends archive tests.

## Dependencies and handoffs

Depends on W67-W71 and hands off a tested publication boundary to release work.

## Implementation approach, risks, and edge cases

Exclude only `.env` manifests and temporary manifest files during staging;
retain all other hidden evidence and make the test assert both absence of
manifests and presence of the plan.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Publication filtering is observable and security-sensitive. |

## Owned work units

- `W74` — Filter local manifests from publication staging.
- `W75` — Test archive exclusion and retained plan evidence.
