<!-- MODE: DEV -->
# Tests that share a machine-wide resource fail only when something else runs

A test that passes alone and fails while another run is active is not flaky, it
is colliding on something the whole machine shares. Two different collisions
were measured on 2026-09-21, and both surfaced the same way: a bare
`FAIL cargo test: <crate>` from the pre-push gate that vanished when the same
tree was pushed again. The rule that follows is
`.agents/MAINTAINER.md` section 1.15.

Environment for both: Linux 7.0.0-30-generic x86_64, cargo 1.98.0 (the repo's
pinned toolchain, entered with `nix develop`), git 2.53.0. Section 1 was
measured on commit `9e45b3b3`, one commit before its fix `e22172b4`; section 2
on `48a09027`, one commit before its fix `a3a5df90`.

## 1. Literal UDP beacon ports (`chat-client-rs`, `chat-server-rs`)

**Answer:** the chat discovery tests hard-coded the UDP beacon ports 47995,
47996 (`src/chat-client-rs/tests/resolution.rs`) and 47997
(`src/chat-server-rs/tests/end_to_end.rs`). Two overlapping runs of the same
suite share those ports, so each hears the other's beacon or fails to bind.

**How it was measured.** Build once, then run
`cargo test -q -p chat-client-rs --test resolution` (3 tests, about 4.4 s):

| trial | result |
|---|---|
| alone, twice in a row | 3 passed, 3 passed |
| two copies started together, trial 1 | copy A 3 passed; copy B failed `the_beacon_carries_a_connectable_host_never_bare_localhost` and `the_client_ladder_heals_a_dead_session_via_discovery` |
| two copies started together, trial 2 | the same result, the same two tests failing in copy B |

After the ports were taken from the OS (`free_udp_port()` in
`src/chat-server-rs/tests/support/mod.rs`), four concurrent trials (two rounds
of two copies) all passed.

**What it means.** A discovery test takes its port from the OS and passes it
down; it never writes a number. `src/chat-mcp/tests/mcp_flow.rs` already
derived its port from a free TCP one.

## 2. Git's detached auto-maintenance in a plan directory (`create-plan`)

**Answer:** git runs its automatic maintenance detached after a commit, so it
outlives the tool that committed and creates and removes files under `.git`
while the plan directory is read, copied or removed. Measured on
`planning-server`'s
`update_step_with_a_stale_revision_is_refused_and_changes_nothing`, which
snapshots the plan directory before and after a refused update.

**How it was measured.** Build the test binary once (`cargo test -p
planning-server --lib --no-run --message-format=json` prints its path, of the
form `target/debug/deps/planning_server-<hash>`) and run that executable
directly with the test's name as its only argument. Each worker is a background
subshell that does this 300 times in a row and counts the runs that exit
non-zero, keeping a failing run's output; N workers are started together and
awaited. The two "before" and "after" plan-directory snapshots the test prints
on failure were compared by their file-name sets.

| workers in parallel | runs | failures |
|---|---|---|
| 1 | 40 | 0 |
| 8 (2400 runs) | 2400 | 14 |
| 8 (2400 runs), after `create-plan` set foreground maintenance | 2400 | 0 |

Every one of the 14 failures had the same single difference between the
"before" and "after" snapshots: `.git/objects/maintenance.lock`, present in the
first and gone from the second. Two other failures of the same test family
were seen and not reproduced: the macOS bash 3.2 CI leg failed the same test
with `NotFound` from `fs::read` in the snapshot walk (the missing file was not
named), and a sibling copy test once failed locally with
`cp: cannot stat '.../.git/objects/bitmap-ref-tips_*'`. Both fit the same
mechanism; only the `maintenance.lock` case was measured.

A tiny repository left nothing behind in a 3 s watch after one commit
(`ps` and `find .git` both empty), so the lock is short-lived: the window is
narrow and only parallel load widens it.

**What it means.** `create-plan` sets `gc.autoDetach` and
`maintenance.autoDetach` to `false` in a repository it creates itself, and only
there: an existing project's own repository keeps whatever the user set.
Maintenance still runs, in the foreground, so it finishes before the commit
returns. A test that snapshots, copies or deletes a directory a tool has just
committed into is exposed to the same race unless the tool's git runs in the
foreground.

## Re-checking

Section 1 reproduces on `9e45b3b3` and section 2 on `48a09027`, using only the
commands above; on `a3a5df90` and later neither should. If either fails again
on a current tree, something else is now shared: look at what the failing
runs have in common before adding a retry.
