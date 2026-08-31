# Goal: The removal's declared surface

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 01 to 03 for a binary that does the work the removed files did, and on goal 16 for the regenerated installer: W81's cross-check reads the skill_files list out of install.sh, so it cannot agree with this goal's manifest until W113 has regenerated it. Confirmed by the adversarial review, finding AR-04: the template, the serve wrapper, requires.tsv, both package tables, SKILL.md and the reference documentation all name the old renderer, and no inventory row owned any of them. W15 and W16 instructed those edits inside their own instructions while their atomicity checks certified that no other file changes, which is the contradiction this goal resolves. The dependency on goal 16 was added by AR-54 and went unrecorded here until AR-61.

## Outcome and definition of done

§ 3.1
Every file that declares, ships or documents the old renderer is corrected, so nothing points at something that no longer exists. Demonstrated by installing the planning skill from a clean checkout and finding no reference to the removed renderer, its template, its serve wrapper or its runtime requirements in any declaration, manifest or document.

## Why this goal is needed

§ 4.1
A deletion is not finished when the file is gone. Every declaration that still names it either fails a freshness gate or, worse, tells the next reader to run something that no longer exists. This goal is what makes the removal true rather than merely started.

## Scope

§ 5.1
In scope: the token template, the serve wrapper, the runtime requirement rows, both package tables, the skill contract and the reference documentation. Out of scope: the tests, which goal 14 owns, and the binary itself, which goals 01 to 03 own.

## Affected files, systems, data, and interfaces

§ 6.1
planning/templates/plan-overview.html.tmpl and planning/scripts/overview-serve.sh are deleted; planning/requires.tsv loses the overview-server-runtimes any-of group; planning/PACKAGE-MANIFEST.tsv and planning/PACKAGE-MAP.tsv lose the removed rows and gain both the artifact rows and the row and entry for planning/binaries.tsv, which AR-43 assigned here; planning/SKILL.md and planning/docs/README.md are corrected.

## Dependencies and handoffs

§ 7.1
Depends on goals 01 to 03 for the replacement binary and on goal 16 for W113 and W81 ordering. The local order is W80 removes the runtime requirement rows; W113 regenerates install.sh from the changed declarations; W81 and W82 reconcile the package manifest and map; W83 and W84 update the contracts. Goal 14 then rewrites the tests and goal 09 consumes the completed removal branch through W48.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: work outward from the files themselves to the declarations that name them, so no step deletes a file another step still expects. The order is recorded in the dependencies rather than left to chance. Risk: a manifest and a map that disagree pass separately and fail together, so both are corrected in adjacent steps and one verification reads them as a pair. Edge case: removing the runtime requirement rows lowers the skill's declared requirements, which is the opposite of the usual direction and will look like a mistake to a reviewer who has not read section 7.1 of the plan description.

## Owned work units

§ 9.1
`W78` — Delete the at-underscore token template. It exists only for the substitution pass the binary replaces, and leaving it behind invites a future reader to wire it back up.

§ 9.2
`W79` — Delete the serve wrapper that chose a runtime rung and passed the plan directory to it. The binary serves the artifact itself, so the wrapper has nothing left to choose between.

§ 9.3
`W80` — Remove the four-row any-of group that declared python3, node, perl and socat for the overview server. A shipped binary asks nothing of the box, so the requirement is not merely satisfied differently, it is gone.

§ 9.4
`W81` — Remove the rows naming the renderer, its template, the serve wrapper and the runtime directory, and add the rows for the prebuilt artifacts. A manifest that lists a deleted file fails its own freshness check.

§ 9.5
`W82` — Remove the map entries for the same deleted files and record the artifacts in their place, so the map and the manifest agree on what ships.

§ 9.6
`W83` — Correct the instructions that tell an agent to run render-plan-overview.sh or overview-serve.sh, naming the binary and its subcommands instead. This is the contract agents act on, so a stale instruction here is followed rather than noticed.

§ 9.7
`W84` — Correct the reference documentation for the overview: how it is rendered, how it is served, and what a platform without a prebuilt artifact is told.

## Testing requirement

| Test required | Rationale |
|---|---|
| no | No new test belongs to this goal: every file it touches is already covered by a standing repository gate, and the gates are named correctly here after an earlier version named the wrong one. tests/test-skill-files-manifest.sh and planning/tests/test-installer-manifest.sh are what cross-check the manifest, the package map and skill_files() against the tree, so a row naming a deleted file fails there; tests/test-mode-markers.sh checks that a file's MODE marker agrees with skill_files() and does not perform that cross-check, though an earlier version of this rationale credited it with doing so, which adversarial finding AR-38 recorded. W113 regenerates install.sh so a stale runtime row shows as a diff its own freshness gate catches, and the documentation corrections are proven by this goal's definition of done, an install from a clean checkout that names the removed renderer nowhere. Goal 14 owns every change to the suite itself. |

## Goal-size exception
