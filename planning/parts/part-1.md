<!-- MODE: PROD -->
> Generated from `skill-source.txt` by `scripts/generate-skill-docs.sh` — do not edit.
> Part 1 of 4 — setup, operating rules, gates, and establishing the plan boundary.
>
> Before treating this part as read: find the line below matching
> `<!-- SKILL-LOAD-PROOF part=part-1 token=... -->` — its position moves on every
> regeneration — and run `planning/scripts/verify-skill-load.sh --part part-1
> --token <the-token-you-found>` before continuing. Naming a token is not
> enough; the command must succeed. If it refuses, you have not finished
> reading this part.


# Planning

Use this skill to turn an initiative into a directory of Markdown files that
another agent can resume and execute without reconstructing missing context.

Do not use it for a small, self-contained change or a temporary in-chat plan.

## Setup / prerequisites

Every helper command in this skill is written as
`"$PLANNING_SKILL_DIR/scripts/<name>.sh" ...`, where `<installed-planning-skill-directory>`
and `<PLANNING_SKILL_DIR>` both mean **the directory containing this `SKILL.md`**
(the `planning/` directory of the installed skill). Before running any helper,
set it to that path, for example:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"   # e.g. the installed planning/ dir
export PLANNING_SKILL_DIR
```

Plans live under a **plans root**, resolved by `scripts/plan-root.sh` (see
section 2): `PLANS_ROOT` if set, else `<project>/.plans`, else `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/plans`. Set
`PLANS_ROOT` explicitly when automation must never prompt. Angle-bracket
placeholders such as `<plan-directory>`, `<planname>`, `<goal>`, `<step>`,
`<WNN>`, and `<text>` are literal tokens to be substituted with real values,
not literal text.

**Normative language.** Requirement keywords follow RFC 2119 / RFC 8174
meaning: **MUST / MUST NOT** = absolute requirement; **SHOULD / SHOULD NOT** =
recommendation that may be overridden only with a stated reason; **MAY / OPTIONAL**
= permitted. Bare imperatives in the Operating rules are MUSTs. When a rule says
"never", it is a MUST NOT. This convention is stated here once and applies to
this file and the three references.

**Numbered paragraphs.** Plan documents are organized into numbered paragraphs
labeled `§ N.N` (section.number, e.g. `§ 2.1`, `§ 9.2`), maintained by the
helpers. Address one via `-p N.N: <text>`; the helper flags map to sections
(`--description-paragraph … 6.1`, `--goal-paragraph … 5.1`, etc.). The **change
target** of a work unit is its one file plus primary symbol (or file scope),
from its inventory row and step header. The goal **roster** is the goal's
`§9.1` owned-work-units list naming every unit the inventory assigns to it.

When an initiative creates, changes, repairs, or validates any UI component,
page, interaction, visual state, or user-facing flow, read
[`references/ui-user-story-validation.md`](references/ui-user-story-validation.md)
in full before establishing the plan boundary. Its workflow is mandatory for
that plan; it adds browser-driven discovery, a user-story artifact, and a
bug-priority feedback loop.

**Plan reads go through the gated reader, never whole-file `cat`/`Read`.**
Before reading any plan artifact (plan docs, goal/step files, progress,
work-unit inventory, adversarial-review), read
[`references/plan-read-contract.md`](references/plan-read-contract.md) and
follow it. It applies to the main agent and to every subagent: plan content
MUST be read via `plan-context.sh read --plan-dir <PLAN_DIR> --document ID`
(or `--unit WNN`), never by `cat`/`Read`/whole-directory load of a plan file or
the `.plans/` tree. A wholesale plan read is a context-overflow violation; if
the gate cannot serve something, report it as a limitation rather than
bypassing it. Each read returns one **page**: while a page reports
`next_token`, records are still withheld — pass it back as `--token` until a
page returns without one, and never treat a single page as the whole document.

## Operating rules

- Keep the plan factual, concise, and executable.
- Separate the initiative into non-overlapping goals. Each goal owns one
  meaningful outcome or area of change.
- Make every goal and step self-contained. Include the context needed to act;
  do not require the agent to infer details from unrelated files. A claimed
  property is self-contained only when its condition travels with the claim: a
  cache needs its key (absolute vs relative path, exact spelling), idempotence
  needs what makes a repeat safe, ordering needs the sort key, validation
  needs what was validated. "Already memoized" without the key shape cost one
  plan a re-read on every call — the instruction achieved the opposite of what
  it said.
- One module over one piece of state owns every export that state needs. When
  decomposition splits a module so unit B must reach into unit A's file to
  invalidate or extend A's export, the split is wrong: give A both the export
  and its invalidator, make B depend on A, and assert the dependency in B's
  acceptance criteria. This is the rule the classify seams, synthesis and
  attribution findings each re-derived at decomposition cost; apply it there
  instead of discovering it in cycle 12.
- Record confirmed facts separately from assumptions. Ask the user only when
  an unresolved choice could materially change scope, implementation, risk, or
  verification. Otherwise make a reasonable assumption and record it.
- Do not invent details to make a plan appear complete. Mark unknowns as open
  questions or risks. A weaker true claim beats a stronger false one: when a
  unit's criterion cannot be met by construction, narrow the claim and record
  the blocking mechanism as a non-goal instead of promising the unreachable.
- Keep progress accurate. A completed status requires the implementation and
  all applicable verification to have passed.
- Treat decomposition as a design activity, not a formatting activity. Do not
  create goal or step files until the work-unit inventory and ownership map in
  section 2.2 pass their checks.
- Do not hide multiple edits behind a broad verb such as "implement",
  "update", "integrate", or "wire up". Name the concrete file and symbol (or
  file-level scope) that changes.
- Only invoke this skill when the task explicitly requests a durable plan or
  resumable plan files. The presence of `.plans/`, `brainstorm.md`, or
  plan-shaped files does not by itself authorize loading this skill; a
  subagent must not self-load it just because it recognizes those paths.
- **Subagent skill policy.** When you hand work to a fresh secondary agent
  (adversarial review, reviewer, or any subagent), its starting prompt must
  explicitly say: "Do not load any skill on your own. Use only the skills
  explicitly named in this starting prompt; do not infer a skill from file
  names, directories, or paths (for example `.plans/`, `.brainstorm/`, or
  skill files). If you believe another skill is needed, state it and stop —
  do not load it." Do not spawn a subagent that could read this skill and
  then reload it autonomously.
- **Naming a class does not schedule it.** Mentioning a new class, file, or
  method in a unit's instructions does not create an inventory row for it, and
  that unit's own atomicity check forbids touching another file. If a unit's
  instructions **instruct an edit** to a symbol that is not its own change
  target, an inventory row must own it before any plan depends on it.
  `validate-plan.sh --propagation` checks this (on by default): an instruction
  to edit a well-formed `Class::method` that no inventory row owns is surfaced
  as a WARN (never a blocking FAIL), because the rule cannot tell an edit
  instruction from a seam description in short form. A mere mention of a
  vendor/core seam is the point of naming it and is not flagged.
- **When a helper refuses a call, re-issue the call — never patch the script
  that produced it.** If a guard correctly refuses a malformed invocation,
  fix the invocation and re-run it. Editing the invoking script with `sed` or
  a one-off rewrite to force the call through is how literal shell commands
  end up inside plan prose and paragraphs get truncated. The guard worked; the
  workaround is the defect.
- **Record the reason for a decision as carefully as the decision.** A false
  recorded reason propagates into downstream fixes exactly as a false fact
  does. Verify a claim before writing it into a plan as a fact; when a decision
  is right but the reason is unverified, mark the reason as an assumption and
  verify it.
- **State what a correction replaced and why.** A corrected paragraph should
  say what the earlier version said and why it was wrong (for example: "an
  earlier version of this criterion took that direction from a configured
  parameter; it is not an open question"). This lets the next reviewer verify
  the fix landed instead of re-deriving it, prevents the same question being
  reopened, and makes a stale-wording sweep self-documenting — a `find` hit on
  old wording is instantly classifiable as live text or a deliberate
  corrective reference. The cost is verbosity; it is worth it.

## Tool discipline and context limits

Planning must stay within the agent's available context budget. When the
environment provides context-limiting tools—such as bounded reads, result
limits, pagination, summarization, compaction, or scoped subagent contexts—use
them while researching and reviewing a plan. Request only the files, symbols,
logs, and output needed for the current work unit; do not load an entire
repository or an unbounded command result when a scoped query is sufficient.

Use the planning skill's bundled shell scripts for creating, reading, and
mutating plan documents, trackers, inventories, reviews, and UI artifacts.
Do not reconstruct their behavior with ad-hoc patches or one-off text
rewrites. If a required helper is missing, add it to the skill before using
the workflow and keep its output validator-compatible.

Use the repository's available code-lookup tools before broad text searches
when discovering implementation files, symbols, callers, dependencies, or
blast radius. This may be an indexed code graph, symbol search, language
server query, IDE index, or another repository-aware lookup facility. Scope
the lookup to the named work unit and follow its pagination or result limits.
Use text search only for literals, non-code documents, configuration values,
or when the repository-aware lookup cannot answer the question.

**Comment discipline for produced code** (see
[`references/comment-discipline-contract.md`](references/comment-discipline-contract.md)).
Code produced under a plan MUST be self-documenting; comments MUST NOT exceed
three lines, MUST keep only genuinely useful, non-evident programming specifics,
and MUST NOT narrate/duplicate what the code already says. Unneeded comments
MUST be removed. Cross-file discovery MUST use repository-aware lookup (code
graph, symbol search), not comments. The post-implementation-review skill
flags violating comments as review findings.

These rules are agent-generic: use the strongest context limiter, shell
workflow, and code-lookup facility available in the current environment, and
record any unavailable facility as a plan constraint when it affects
discovery or verification.

## Hard planning gates

These rules are mandatory for a new or materially revised plan. A plan is not
ready to execute until it passes the decomposition and validation gates below.

### Atomic work-unit limit

A **work unit** is one independently reviewable change target:

- **Source code:** one file and one function, method, class, component, or
  other named primary symbol in it. A top-level static class, constant,
  initializer, or declaration is also valid when no executable symbol exists.
  Name one optional nested loop, callback, branch, or anonymous function as a
  subscope when that is the actual change target; otherwise use `N/A`.
- **Markup:** one file and one named DOM subtree or template block (for
  example, `#checkout-summary`).
- **Style:** one file and one CSS selector or named style token (for example,
  `.completion-message`).
- **Configuration:** one file and one precisely named key, route, declaration,
  or section.
- **Test:** one test file and one test class or test function.
- **Documentation, migration, fixture, or asset:** one file and one named
  section, migration, fixture, or asset.
- **Verification:** one named command or one bounded browser/API flow. It has
  no implementation file, but it is still its own work unit and step.

One implementation step owns exactly one work unit. It MUST NOT include a
second source file, second symbol, second test target, or a catch-all such as
"related callers." Make those separate, ordered steps even when the changes
are mechanically small. Do not use globs, directory names, or "all affected
files" as a target.

An exception is allowed only for an inseparable generated-file update. Record
the generator command and every generated file in the step, set its type to
`generated`, and explain why individual review is impossible. Never use this
exception for ordinary source, configuration, test, or documentation edits.

### Goal size limit

A goal owns one coherent, independently demonstrable outcome and contains
**2–10 work units** (MUST). A single-work-unit goal is allowed only for a
genuinely standalone documentation, configuration, discovery, or verification
outcome; state the reason in its `goal.md`. A goal with more than 10 work
units is invalid and MUST be split at the next stable product, contract,
deployability, or ownership boundary. Do not split merely by file type.

Every goal needs its own definition of done that can be demonstrated without
claiming completion of later goals. If it cannot be demonstrated independently,
it is a segment of another goal, not a goal.

### Target reachability gate

Before a work unit may target a template, block, or layout, the plan must
record evidence that the target actually renders on the surface in question. A
file existing is not evidence. Record per target, in order:

1. the file exists, and whether it is core's, a module's, or a theme's;
2. no layout in `app/code` or `vendor` removes the block that renders it
   (`<referenceBlock ... remove="true"/>`);
3. no layout re-points it to another template (`<action method="setTemplate">`
   or a re-registered `<block ... template>`);
4. the theme actually in use for that area resolves to it, including every
   intermediate theme in the inheritance chain;
5. for a module template, whether any theme overrides it.

A goal whose units target templates, blocks, or layouts must own a discovery
unit that records this evidence per target, and that unit's acceptance
criteria must require the recorded evidence — not merely that a search was
performed. Steps 2 and 3 are the ones repeatedly missed: a theme-override
search finds neither.

**Marker pre-check (required first step for any unit whose change target is a
template, and for any template-ambiguous block/layout target).** Static
reachability evidence is necessary but not sufficient: it cannot tell which
route actually renders a target that several themes or a re-pointed layout
could serve. Before building anything on an assumption about which file
renders, add a visible literal marker to the candidate template, confirm which
route and block render it (browser or live block tree), then remove the marker
and record the confirmed surface. This is the only mitigation that reliably
catches a "wrong target surface" defect — the class no validator can detect
because it requires reading live block trees and theme chains.

## 1. Establish the plan boundary

Before creating plan files, establish enough information to write an
executable plan:

- Desired outcome and definition of done
- In-scope and explicitly out-of-scope behavior
- Affected files, modules, layouts, services, data, or systems
- Constraints, ownership boundaries, conventions, and compatibility concerns
- User-visible behavior and required browser verification, if any
- Backend behavior and required unit or integration verification, if any
- Dependencies on other goals or external systems

<!-- SKILL-LOAD-PROOF part=part-1 token=64647e1270fa28f0 -->


Ask focused follow-up questions for material gaps. Do not ask for details that
can be discovered safely from the repository or environment. If the user does
not need to choose between materially different approaches, choose one,
explain the assumption in the plan, and continue.

Do not create implementation files until each goal has a clear owner, boundary,
outcome, and definition of done.
