# AI Skills Bug Report

**Reported:** 2026-08-20
**Scope:** Planning skill, gated plan tooling, planning mutation helpers, and AI-agent workflow defects
**Excluded:** Application bugs, Gradle/VPS issues, CI configuration, Git/network failures, and product/legal decisions

## Summary

The planning skill and its helper tools have multiple defects that make resumable, gated plan execution unreliable. Several helpers reject valid workflow inputs, hide useful diagnostics, report successful mutations that are not visible through canonical readers, or generate work units whose targets do not exist. These behaviors encourage agents to bypass the gated reader and inspect implementation details directly, which undermines the skill's safety and traceability model.

## Findings

### PLAN-001: Explicit plan creation probes an unintended global root

**Severity:** High

When a plan root was explicitly supplied, plan creation still probed the default/global plans location unless `PLANS_ROOT` was exported. This caused confusing permission failures and made explicit-root behavior non-deterministic.

**Expected:** An explicit plan directory must be authoritative and must not access an unrelated global root.

### PLAN-002: Gated reader lacks a planning-bugs document identifier

**Severity:** High

`plan-context.sh read --document planning-bugs` rejected an existing `planning-bugs.json` artifact as an unsupported entry. The bug-report workflow therefore could not use the gated reader for its own canonical bug document.

**Expected:** Every canonical plan artifact, including planning bug reports, must have a documented reader identifier and supported format.

### PLAN-003: Unsupported reader views are not discoverable

**Severity:** Medium

`plan-context.sh read --unit W11 --view full` returned only `unsupported view: full`, without listing supported views or providing a bounded alternative.

**Expected:** Invalid views must print valid values and recovery guidance.

### PLAN-004: Public helpers handle `--help` inconsistently

**Severity:** Medium

`update-step.sh --help` printed usage, while `update-progress.sh --help` treated `--help` as a path and reported a missing progress file.

**Expected:** All public planning helpers must support consistent, side-effect-free `--help` behavior.

### PLAN-005: Gated reader diagnostics are insufficient for execution

**Severity:** High

Bounded work-unit summaries omitted enough acceptance, dependency, verification, and target detail that execution required shell inspection or guesswork.

**Expected:** A bounded unit read must include owner, target, acceptance criteria, verification command, dependencies, status, and pagination metadata.

### PLAN-006: No helper maintains UI-story work-unit links

**Severity:** Medium

UI stories could be added, but no safe mutation helper updated or validated their related work-unit references atomically.

**Expected:** UI-story mutations must maintain traceability links through a validated helper.

### PLAN-007: Coverage mutation reports success without canonical visibility

**Severity:** High

`add-coverage` reported success, but later coverage rows were not visible through the canonical bounded inventory output.

**Expected:** A successful mutation must be immediately observable through the canonical reader, including returned/total row counts and the new record ID.

### PLAN-008: Summaries truncate without an explicit completeness signal

**Severity:** High

Plan-context summaries were bounded and truncated without reliably communicating that later records were omitted. This can produce false completion conclusions.

**Expected:** Every bounded response must report total records, returned records, truncation state, and a next-page mechanism.

### PLAN-009: Fix-key workflow is not supported by plan-context

**Severity:** High

The documented fix-key verification workflow expected a document entry that `plan-context` did not support. Formal claims verification was therefore blocked.

**Expected:** All documents used by approval, validation, and fix-key verification must be registered in the reader schema.

### PLAN-010: Invalid helper arguments produce misleading state errors

**Severity:** Medium

Argument mistakes were reported as missing plan files or progress state rather than syntax errors. This makes operators investigate the wrong layer.

**Expected:** Parse and validate arguments before resolving plan paths or reading state.

### PLAN-011: Generated work-unit targets can name nonexistent files

**Severity:** High

W31 targeted `app/src/main/java/com/healthapp/features/FeatureCompatibilityMode.kt`, but the implementation was in `PrivacyModels.kt`. W35 targeted `PrivacyRepositoryRuntimeTest.kt`, but that file did not exist.

**Expected:** Plan generation must resolve target paths and symbols against the repository, flag missing targets, and distinguish moved symbols from absent implementations.

### PLAN-012: Plan generation does not reliably preserve repository reality

**Severity:** High

Generated plans can describe source/test boundaries that do not match the current repository layout, creating false ownership and verification assumptions.

**Expected:** Plan generation should snapshot repository paths and symbols, validate them before approval, and invalidate or revise stale targets when implementation moves.

## Recommended Fix Order

1. Add one shared argument parser and help contract for all planning helpers.
2. Register every canonical document, including bug reports and fix-key artifacts, in the gated reader.
3. Add pagination and explicit truncation metadata to every bounded reader response.
4. Make mutation helpers return canonical record IDs and immediately verify reader visibility.
5. Add repository path/symbol validation to work-unit generation and plan validation.
6. Add regression tests covering invalid arguments, unsupported documents/views, stale targets, and post-mutation visibility.
