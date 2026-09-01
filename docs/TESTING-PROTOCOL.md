# Interactive-shell testing protocol

This protocol verifies that an AI agent can start a fresh subworker and have
that subworker operate unfamiliar interactive terminals by observing the
current screen and choosing the next action. The subworker is the driver; no
shell script may perform the interactive workflow on its behalf.

The subworker must dynamically generate a random filename ending in `.txt`
under `/tmp`, create it with `nano`, later find that same file through `mc`,
and edit it to `hello universe`. The workflow is not a transcript replay: the
subworker must use the wrapper's observations, structured actions, and
discovered element labels.

## Scope

The agent-driven test covers one wrapper-owned interactive Bash session and
exercises:

1. The parent agent starts a fresh subworker with only the task and the
   interactive-shell tool instructions.
2. The subworker starts an interactive `bash` through the wrapper. From that
   Bash prompt, it dynamically generates a path such as
   `/tmp/agent-tty-$RANDOM.txt`, retains the generated path, and runs `nano` on
   it.
3. The subworker observes `nano`, types `Hello World`, and saves and exits
   `nano`.
4. From the same interactive Bash prompt, the subworker runs `mc` without
   supplying `/tmp` as a startup argument.
5. The subworker uses `mc`'s visible file-pane UI to navigate a panel from its
   current directory to `/tmp`, observing each directory and focus change.
   It must use the file pane's normal directory entries and parent entry; it
   must not use a quick-directory command, shell escape, or startup argument to
   jump directly to `/tmp`.
6. The subworker observes the `/tmp` listing, finds the dynamically named file,
   selects it, and discovers how to open it in the configured editor. It should
   use visible UI elements where possible and ordinary keyboard navigation when
   that is more appropriate. This protocol supplies no application-specific
   controls.
7. `mc`'s configured internal `mcedit` editor subprocess replaces the content with
   `hello universe`, saves it, handles
   any overwrite prompt from the observed screen, and exits back to `mc`.
8. The subworker exits `mc` back to the same Bash prompt, runs `less` on the
   dynamically named file, observes `hello universe`, discovers how to exit
   `less` from its visible UI, built-in help, or manpage, and returns to Bash.
9. The subworker reports the final file contents and cleans up the dynamically
   named file.

The wrapper's socket and event log use a private temporary directory. The
agent scenario intentionally uses one dynamically generated `/tmp/*.txt`
file and must remove it during cleanup. It must not write to the repository or
to a user's real home directory.

## Preconditions

Run from the repository root in the Nix development environment. The isolated
crate must have been built with the current Rust toolchain, and `nano`, `mc`,
`less`, and `rjq` must be available. If any interactive dependency is missing,
the agent test cannot claim success.

## Procedure

Build the isolated tool and its protocol tests:

```sh
nix develop .#default --command cargo test --locked \
  --manifest-path src/interactive-shell/Cargo.toml
```

Then have the parent AI agent start a fresh subworker. The subworker prompt
must require one wrapper session running an interactive Bash, dynamic
generation of a `/tmp/*.txt` filename from that Bash, and the entire
`nano` → Bash → `mc` → editor workflow within that session. It must provide
access to the interactive-shell wrapper and input command and require the
subworker to inspect `observe` output before every consequential action. The
parent must collect the subworker's action log and observations as evaluation
evidence.

The subworker must independently launch interactive Bash, launch `nano` from
that Bash, and discover how to enter text, save, and exit `nano` from its
observations or documentation. It must use `mc` from that same Bash, move
focus between its panels using the UI's focus affordances or keyboard focus
navigation, and traverse directory entries in a file pane until it reaches
`/tmp`. It must locate the file by its observed label/elements, use `mc`'s
configured internal `mcedit` editor, edit the file, handle save and overwrite
prompts, and exit back to Bash. It must then discover how to exit `less` after
verifying the content. The parent must reject a run where a shell script sends
the workflow's keys, where separate wrapper sessions are used for the
applications, where a quick-directory command, shell escape, or startup path
is used to reach `/tmp`, where the subworker is given a fixed transcript or
application controls, or where the subworker cannot show the observations or
documentation that justified its actions.

The existing shell exploration script is only a deterministic fixture smoke
test for PTY integration. It is useful for regression checking, but it is not
evidence that an AI subworker can perform this protocol.

### Current evidence boundary

The latest successful run exercised the wrapper against real Linux `nano`,
`mc`, and `less` processes, with the wrapper's PTY, Unix socket, screen parser,
observation, element discovery, and input paths in the loop. Its driver was a
hard-coded Bash script, however, so it demonstrates wrapper integration and
the happy-path oracle only. It does not measure an AI model's planning,
observation use, recovery, or ability to select unknown controls. No live
subworker performance result should be reported until the parent-agent-driven
procedure above has been run.

For a resource-capped fixture validation run, use the repository's test wrapper:

```sh
nix develop .#default --command \
  resource-limited-testing/scripts/limited-run.sh 6G 400 -- \
  tests/test-interactive-shell-exploration.sh
```

The fixture starts each application through `interactive-shell`, waits for
observed screen text, and sends actions through `interactive-shell-input`.
It does not replace the required subworker evaluation.

## Pass criteria

The run passes only when all of the following are true:

- A fresh subworker, started by the parent agent, dynamically generates a
  `/tmp/*.txt` path, visibly reaches the `nano` save flow, and that file
  contains exactly `Hello World` after the first save.
- `mc` navigates a file pane through visible directory entries to `/tmp`, uses
  an observed file label, and opens that dynamically named file in the
  `mc`'s configured internal `mcedit` subprocess using a control discovered by
  the subworker.
- The second editor flow reaches and confirms the overwrite prompt.
- The final file content is exactly `hello universe`.
- `mc` exits back to the same interactive Bash session after the edit.
- `less` is launched from that same Bash session, visibly shows
  `hello universe`, and exits cleanly using a control discovered by the
  subworker.
- The parent records the subworker's observations, selected actions, process
  outcomes, and final file contents.
- The fixture smoke test may additionally print:

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
- discover application controls from visible labels, built-in help, or
  manpages and discover the interactive elements and controls itself;
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
The MC subprocess path is part of this validation: the parent agent must keep the wrapper socket usable while the editor child owns the terminal.
