# Step: 01-step-read-plan-tree

## Ownership

- Goal: `01-engine-core`
- Work unit: `W01`
- Type: `source`

## Change target

- File: `src/plan-overview/src/plan/tree.rs`
- Primary symbol or file scope: `read_plan_tree()`
- Subscope: `N/A`

## Objective

§ 4.1
Read a plan directory into one owned in-memory structure, so every later stage works from a parsed tree rather than from paths and process arguments.

## Instructions

§ 5.1
Open the plan description, every goal document, every step and testing companion, the work-unit inventory, the adversarial review, the coverage table and the review history, reading each as a file. Return one owned structure holding their contents and paths. No content is passed through a process argument at any point, which is the limit that breaks the current renderer.

## Acceptance criteria

§ 6.1
Reading the 337 KB size fixture returns a populated structure with every document accounted for, and the same read on a plan with a missing optional document reports which document was absent rather than failing. No code path builds a command line containing document content.

## Handoff

§ 7.1
Later units consume this structure; none of them reopens plan files. The document set and its accessors are the contract W02 parses against.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
