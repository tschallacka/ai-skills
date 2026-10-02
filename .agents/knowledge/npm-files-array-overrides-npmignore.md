<!-- MODE: DEV -->
# `package.json`'s `files` array ships a listed directory wholesale, `.npmignore` notwithstanding

Measured 2026-09-25, T70's real 2.0.0-alpha release rehearsal, on `master`
after PR #168 merged.

## The fact

When `package.json`'s `files` array lists a directory by name (as it does here
for `agent-identity-plugin`, `chat-interrupt-plugin`, `editor-gate-plugin`,
`tui-hint-plugin`, `planning`, `todo`, `bug-report`, `chat`, and others), `npm
pack`/`npm publish` includes **every file under that directory**, even one
`.npmignore` explicitly lists for exclusion. `.npmignore` is not ignored
everywhere — it is what governs anything the `files` array does not already
cover by a directory-shaped entry — but it does not filter *inside* a
directory `files` already claims wholesale.

Measured directly: `.npmignore` (regenerated fresh via `installer/
build-release.sh --npmignore`, which derives it from the same MODE:PROD
declarations `--list` uses) explicitly lists `agent-identity-plugin/
README.md` and `agent-identity-plugin/tests/test-agent-identity-injection.sh`
for exclusion. A real `npm pack` still includes both. `package.json`'s
`files` array lists `"agent-identity-plugin"` with no matching `!` negation
for either path, and that inclusion wins.

## What this means here

`installer/build-release.sh --list` (and the `.npmignore` derived from it)
define the **GitHub release tarball's** contents — the strict, MODE:PROD-only
set `test-release-package.sh` enforces byte-for-byte. They are not the same
definition as **the npm package's** contents, which `package.json`'s own
`files` array (plus its hand-maintained `!` negations) controls instead, with
`planning/tests/test-npm-package.sh`'s frozen `npm-package-baseline.tsv` as
the actual, tested authority for what that should be.

So a path appearing in `npm pack --dry-run` but not in `--list` is not
automatically a packaging bug -- it can be an intentional, already-baselined
choice for a `files`-listed directory (several plugin READMEs and test
scripts ship this way today, along with `installer/bootstrap.sh`, needed
there for npm's own `bin` mechanism, and `planning/skill-source.txt` and
`planning/scripts/generate-skill-docs.sh`, none of which `--list` carries).
RELEASE.md's own step 6 language ("`npm pack --dry-run` ... should match
`--list`") is an approximation for that reason: treat a divergence as a
prompt to check `test-npm-package.sh`'s baseline before assuming it is new
drift. A path that is genuinely unwanted needs an explicit `!` negation
added to `package.json`'s `files` array itself -- editing `.npmignore` alone
cannot remove it from a `files`-listed directory.
