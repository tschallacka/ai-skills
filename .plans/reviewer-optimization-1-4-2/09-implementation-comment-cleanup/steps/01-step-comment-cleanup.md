# Step: 01-step-comment-cleanup

## Ownership

- Goal: `09-implementation-comment-cleanup`
- Work unit: `W63`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `final implementation comment cleanup`
- Subscope: `N/A`

## Objective

Remove developer-journey narration from finished implementation code while retaining concise comments that explain non-obvious behavior.

## Instructions

1. Build the final changed-file list from the completed work-unit handoffs and inspect comments in each implementation file.
2. Remove comments that merely narrate work performed, repeat the code, record temporary debugging thoughts, or explain behavior that is obvious from clear names and structure.
3. Refactor remaining useful comments into concise explanations of why the code exists, including non-obvious constraints, invariants, compatibility requirements, security considerations, or deliberate trade-offs.
4. Do not remove required plan, benchmark, audit, or analysis evidence; this step applies to implementation code only.
5. Review the final diff for accidental behavior changes and run the relevant focused tests or validation commands.

## Acceptance criteria

- No developer-journey narration remains in the finished implementation files unless it documents genuinely non-obvious intent or a constraint.
- Remaining implementation comments are concise, accurate, and do not duplicate self-documenting code.
- The final diff contains only intended comment cleanup or necessary concise clarification.
- Relevant tests and validation pass after the cleanup.

## Handoff

Hand off the final changed-file list, removed/refactored comment summary, test results, and any comments deliberately retained with their rationale to the release gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
