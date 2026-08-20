<!-- MODE: PROD -->
<!-- PACKAGE: PROD -->
# Comment discipline contract

Use this contract whenever produced code—implementation files, tests, config,
or generated artifacts—is written or reviewed under a planning initiative. It
applies to the **implementer** writing comments and to every **reviewer**
(implementer self-analysis, independent solutions agent, critical-feedback
agent) that reviews them. The goal is code that is self-documenting, with
comments reserved for genuinely useful, non-evident programming specifics.

Terms use RFC 2119 meaning: **MUST** = absolute requirement, **MUST NOT** =
absolute prohibition, **SHOULD** = recommendation.

## Scope

| Contract clause | Requirement |
|---|---|
| 1. Self-documenting by default | Produced code **MUST** be self-documenting: prefer clear names and structure over explanatory comments. Comments **MUST NOT** narrate what the code already states. |
| 2. Three-line limit | A comment **MUST NOT** exceed three lines. If it would take more, reduce it to the actionable specific or remove it. |
| 3. Genuine specifics only | A comment **MUST** keep only genuinely useful, non-evident programming-specific facts (a subtle invariant, a non-obvious constraint, a reason a reader cannot infer from the code). It **MUST NOT** contain tutorial/why prose, history, or restatement. |
| 4. Remove unneeded | Unneeded or bloated comments **MUST** be removed, not preserved. When in doubt, delete. |
| 5. No comment-driven discovery | Do **NOT** add comments whose only purpose is to explain cross-file relationships or call-graph links. Discovery **MUST** use repository-aware lookup (code graph, symbol search, language server, etc.) where available, not comments. |
| 6. Review findings | Reviewers **MUST** flag any comment that exceeds three lines or lacks genuine programming specifics as a review finding (proposed removal or whittling) in the post-implementation review. |

## Why

Verbose docblocks and tutorial comments duplicate what code expresses and
drift as code changes, so they become noise and misinfo. Restricting comments
to three lines of genuine specifics keeps them accurate, scannable, and rare.
Cross-file discovery belongs in the reader's tooling (code graph, symbol
search), not in comments that duplicate it.

## Enforcement

- Implementers follow clauses 1–5 when writing produced code under a plan.
- The post-implementation-review skill's implementer self-analysis and the
  independent/critical agents apply clause 6: comment-hygiene is a review
  finding category, and oversized/empty comments are proposed for removal or
  whittling (never silently kept).
