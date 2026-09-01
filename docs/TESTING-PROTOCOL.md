# Interactive-shell testing protocol

This protocol verifies that an agent can operate an unfamiliar interactive
terminal by observing the current screen and choosing the next action. It is
not a transcript replay: the test must use the wrapper's observations,
structured actions, and discovered element labels.

## Scope

The isolated test covers one PTY wrapper session at a time and exercises:

1. `nano` creates a new file and saves `Hello World`.
2. `mc` discovers the file in its visible screen, selects it, and opens it
   through the editor action (`F4`).
3. The editor replaces the content with `hello universe` and handles the
   overwrite prompt.
4. `less` displays the resulting file and exits with `q`.

The test uses a private temporary directory for all files and sockets. It must
not write to the repository or to a user's real home directory.

## Preconditions

Run from the repository root in the Nix development environment. The isolated
crate must have been built with the current Rust toolchain, and `nano`, `mc`,
`less`, and `rjq` must be available. If any interactive dependency is missing,
the exploration test reports `SKIP` rather than claiming success.

## Procedure

Build and run the observation-driven test:

```sh
nix develop .#default --command cargo test --locked \
  --manifest-path src/interactive-shell/Cargo.toml
nix develop .#default --command tests/test-interactive-shell-exploration.sh
```

For a resource-capped validation run, use the repository's test wrapper:

```sh
nix develop .#default --command \
  resource-limited-testing/scripts/limited-run.sh 6G 400 -- \
  tests/test-interactive-shell-exploration.sh
```

The shell test starts each application through `interactive-shell`, waits for
observed screen text, and sends actions through `interactive-shell-input`.
For the `mc` step it reads an `observe` snapshot, finds `test.txt` by its
reported label in `elements`, and clicks the discovered element. It may page
down while searching, but it must not assume a fixed row or column.

## Pass criteria

The run passes only when all of the following are true:

- `nano` visibly reaches its save flow and `/tmp`-scoped test data contains
  exactly `Hello World` after the first save.
- `mc` is navigated using an observed file label and opens that file via the
  discovered screen state.
- The second editor flow reaches and confirms the overwrite prompt.
- The final file content is exactly `hello universe`.
- `less` visibly shows `hello universe` and exits cleanly after `q`.
- The test prints:

  `interactive-shell observation-driven nano/mc/less exploration passed`

The Rust protocol tests must also pass. They cover screen snapshots, delta
resynchronization, waits, structured input, arbitrary modifier combinations,
click targets, PTY lifecycle, and cleanup. A skipped dependency test is not a
pass for the interactive exploration itself.

## Agent-behavior review

When reviewing or extending this test, reject implementations that merely
replay a known keystroke transcript. An agent should be able to:

- call `observe` and reconstruct the currently visible state;
- use `wait` for a meaningful visible predicate rather than a fixed sleep;
- inspect `elements` and choose by label or id when semantic metadata exists;
- fall back to coordinates only after observing them;
- issue arbitrary `key`, `combo`, `text`, `paste`, `mouse`, `click`, `resize`,
  and `raw` actions as needed;
- recover with a fresh snapshot after missing screen events; and
- avoid acting on stale clickable elements after a redraw or erase.

The test is intentionally still an end-to-end scenario with known goals. Its
navigation decisions must remain observation-based so it measures whether a
fresh agent can use the helper on an unfamiliar TUI, not whether it memorized
the helper's internal escape sequences.

## Additional validation

Before handing off changes to the parent branch, run:

```sh
nix develop .#default --command cargo clippy --all-targets --locked \
  --manifest-path src/interactive-shell/Cargo.toml -- -D warnings
cargo fmt --all --manifest-path src/interactive-shell/Cargo.toml -- --check
git diff --check
bash -n tests/test-interactive-shell-exploration.sh
nix develop .#default --command shellcheck -s bash --severity=error \
  tests/test-interactive-shell-exploration.sh
```

The test's temporary directory is removed by its cleanup trap. If a run is
interrupted, inspect and remove only the specific test process and directory
left by that run; do not use broad recursive deletion targets.
