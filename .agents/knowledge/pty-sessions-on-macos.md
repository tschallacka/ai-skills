<!-- MODE: DEV -->
# PTY sessions on macOS: exit waits for its output, and cargo under nix hides the race

Measured 2026-09-24 on macOS 26 aarch64 while fixing B377 (`interactive-shell-mcp`'s
`mcp_flow` failing every run).

**On macOS a dying child whose terminal still holds unread output does not
finish exiting until the PTY master reads it, SIGKILL included.** A parent that
sends SIGKILL and then does a blocking `waitpid` while holding the master unread
waits forever. `ps` shows the child as `?Es` with no arguments (its name only,
e.g. `(coreutils)`), and `sample` on the parent shows it parked in `__wait4`.

## How it was measured

- `interactive-shell`'s `posix::stop` sent SIGTERM, polled 300 ms, sent
  SIGKILL, then called a blocking `waitpid`. A `shutdown` that arrived before
  the run loop had read the child's `printf hello` left the wrapper in
  `__wait4` for the full 20 s the test waited. The child was `?Es` the whole
  time, and the session socket was never removed.
- Unit test `stop_returns_when_the_childs_output_was_never_read`
  (`src/interactive-shell/src/posix.rs`): spawn `sh -c 'printf hello; exec
  sleep 600'`, read nothing for 500 ms, call `stop()`. Before the fix it hit
  its 10 s bound (10.51 s). After the fix it passes, and `stop()` drains the
  master on every poll turn instead of blocking.

## Why it only failed under cargo: `DYLD_LIBRARY_PATH` from the nix shell

The race above needs the child's output to arrive after the loop's last read.
On an idle machine it arrived first, so everything passed by hand. What changed
under `cargo test` inside `nix develop`:

- nix's cargo exports `DYLD_LIBRARY_PATH=/nix/store/…-curl-8.21.0/lib` (and
  `DYLD_FALLBACK_LIBRARY_PATH` into `target/debug`). Every test binary, and every
  process it spawns, inherits both.
- With only that variable toggled, the same `mcp_flow` binary goes from pass
  (0.10 s) to fail (0.18 s), whether run directly or under cargo. With it set, a
  2 s pause before `view` passes again, so the child is slower to draw, not
  broken. The mechanism behind the slowdown was not measured.
- **A test runner can hide it.** Any `CARGO_TARGET_*_RUNNER` that starts
  through a SIP-protected binary (`#!/bin/bash`, `#!/usr/bin/env`) has every
  `DYLD_*` variable stripped by the OS, and the failure disappeared. A runner
  with a nix-store shebang kept the variables and kept the failure. Bisect with
  a non-SIP runner or you will measure the wrong environment.

## What it means here

- Anything that stops a PTY child must keep reading the master until it has
  reaped the child. This belongs in the stop path itself, not in each caller.
- `start`'s `ready` means the socket exists, not that the program has drawn
  anything. A test waits for the text it needs (`wait` with `contains`) and
  never calls `view` straight after `start`. That is the readiness rule from
  `github-ci-runners.md`, applied to a local cause of slowness.
- A capture that throws away the wrapper's stderr (`Stdio::null()` in the
  adapter) makes this look like a socket problem. It is the same trap
  `unix-sockets-across-platforms.md` records. Shim the sibling binary with a
  script that logs, then `exec`s the real one, before guessing.
