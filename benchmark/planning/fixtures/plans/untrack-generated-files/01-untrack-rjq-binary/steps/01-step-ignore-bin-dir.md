# Step: 01-step-ignore-bin-dir

## Ownership

- Goal: `01-untrack-rjq-binary`
- Work unit: `W01`
- Type: `config`

## Change target

- File: `.gitignore`
- Primary symbol or file scope: `planning/bin section`
- Subscope: `N/A`

## Objective

§ 4.1
Correct the stale tracked-artifact comment and ignore the triple-shaped per-target artifact directories (planning/bin/*/), then git rm --cached planning/bin/x86_64-unknown-linux-musl/rjq so the blob leaves the index while binaries.tsv rows stay declared-but-unbuilt.

## Instructions

§ 5.1
In .gitignore, replace the stale comment block that claims shipped per-target plan-crypt artifacts are tracked with a comment stating binaries are CI-delivered and never tracked (planning/MAINTAINER.md section 2.15). Add planning/bin/*/ to ignore the triple-shaped artifact directories. Run git rm --cached planning/bin/x86_64-unknown-linux-musl/rjq so the e8bdaef blob leaves the index; the file may remain on disk locally. Leave planning/binaries.tsv and every binaries.tsv row untouched: declared-but-unbuilt stays the legal resting state.

## Acceptance criteria

§ 6.1
git ls-files planning/bin prints nothing; git check-ignore -v names the new rule for planning/bin/x86_64-unknown-linux-musl/rjq; the stale tracked-artifact comment is gone (grep for 'are tracked' under the planning/bin section finds nothing); the index no longer contains the blob while the working-tree file is simply untracked.

## Handoff

§ 7.1
No downstream reliance beyond the state this step creates: the index no longer carries the blob and the ignore rule is in place for every later goal's gitignore sections to follow.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
