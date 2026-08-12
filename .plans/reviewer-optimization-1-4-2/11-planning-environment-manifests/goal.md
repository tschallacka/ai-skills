# Goal: Create reusable planning environment manifests

## Current state and prior-goal handoffs

§ 2.1
Temporary monitor helpers currently repeat absolute paths and run-specific arguments. The planning skill has stable helper scripts and plan roots, but it does not create a concise environment contract that trusted temporary scripts can source.

## Outcome and definition of done

§ 3.1
Plan creation produces a global `~/.plans/.env` and a plan-local `.plans/<plan-name>/.env`. Both contain stable, clearly named, shell-safe path variables with deterministic refresh behavior and restrictive permissions. Trusted temporary scripts can source them through an explicit validation helper without repeating long paths in commands or conversation.

## Why this goal is needed

§ 4.1
Centralized environment variables reduce repeated command text and token use, prevent path drift between helpers, and make temporary monitoring/validation scripts easier to resume safely.

## Scope

§ 5.1
Include manifest creation, safe consumption, refresh behavior, permissions, quoting, path-root validation, and focused fixture tests. Exclude secrets, arbitrary environment inheritance, automatic sourcing from interactive shell startup, and publishing `.env` files into benchmark archives.

## Affected files, systems, data, and interfaces

§ 6.1
W67 owns plan creation integration. W68 owns the trusted manifest-consumption helper and the inventory/migration of applicable planning and temporary helper scripts. W69 owns fixture tests. Runtime files are `~/.plans/.env` and `.plans/<plan-name>/.env`; both are local environment metadata and must remain outside plan deliverable counts and published archives.

## Dependencies and handoffs

§ 7.1
Depends on the existing helper output and plan mutation contracts (W48 and W64) plus monitor continuation (W65). Hands off the variable contract and validation evidence to all plan helpers, temporary monitor scripts, and release workflows.

## Implementation approach, risks, and edge cases

§ 8.1
Emit only quoted `KEY=value` assignments with absolute canonical paths, stable variable names, and no command substitutions. Create parent directories as needed, write through a temporary file plus rename, set mode `600` for manifests, and refresh deterministically without deleting unrelated files. The consumer must verify ownership/permissions, required keys, path containment, and absence of unsafe shell constructs before sourcing.

## Owned work units

§ 9.1
`W67` — Create and refresh global and plan-local environment manifests.

§ 9.2
`W68` — Validate and safely consume the manifests from trusted temporary scripts.

§ 9.3
`W69` — Test the complete manifest lifecycle and safety boundary.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal creates executable shell metadata and a sourcing boundary; fixture tests must prove paths, permissions, isolation, refresh, and rejection behavior. |

## Goal-size exception

§ 11.1
Not applicable: this goal owns three distinct lifecycle outcomes with separate source/helper/test ownership.
