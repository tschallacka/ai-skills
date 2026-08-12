# Goal: Ship and validate the new regression coverage

## Current state and prior-goal handoffs

The repository contains focused tests for manifest, integrity, and reviewer
behavior, but the finite V27 package manifest does not include the new
manifest regression test.

## Outcome and definition of done

The package inventory explicitly includes the regression coverage required by
the implemented planning contracts, and installer tests prove it is emitted
and resolvable.

## Why this goal is needed

An installed skill without its new safety tests cannot reproduce or verify the
contract that the source checkout implements.

## Scope

Include package manifest/map, installer emission, and package regression tests.
Exclude unrelated benchmark result artifacts.

## Affected files, systems, data, and interfaces

W82 updates `planning/V27-PACKAGE-MANIFEST.txt`, `planning/V27-PACKAGE-MAP.tsv`,
and installer file emission. W83 extends installer/package tests.

## Dependencies and handoffs

Depends on W72-W81 and hands off a package-complete safety contract to release
validation.

## Implementation approach, risks, and edge cases

Add only the intended regression files, preserve source/destination ownership,
verify every manifest row resolves, and keep source-only files excluded.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Package omission would make the installed protocol unverifiable. |

## Owned work units

- `W82` — Add required regression files to the package inventory.
- `W83` — Test package emission and source resolution.
