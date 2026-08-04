---
name: resource-limited-testing
description: Use when you are about to run a test, build, analyzer, linter, browser-automation command, or similar tool that may consume substantial CPU or memory. It runs the command under a resource cap. Do not use for lightweight commands, ordinary application commands, or discussions about testing without running a command.
---

# Resource-limited testing

## Why this exists

Some test and quality tools can exhaust a machine's memory or create large
process trees. Without a process-level cap, the operating system may kill
unrelated processes. The wrapper limits the command and its descendants so a
runaway job cannot take down the rest of the system.

Run resource-intensive commands through the wrapper instead of invoking them
directly.

## How it works

On Linux, `scripts/limited-run.sh` runs a command inside a transient
`systemd --user` scope with a hard memory ceiling, no swap, and a CPU quota.
If the wrapped process tree exceeds the memory ceiling, the operating system
kills only the processes in that scope.

On macOS, the same script applies a virtual-memory limit with `ulimit`. macOS
does not provide a portable built-in equivalent to the Linux CPU cgroup quota,
so the CPU argument is accepted for a consistent interface but cannot be
enforced by the fallback. Use a separate platform tool when a CPU cap is
required on macOS.

If neither supported mechanism is available, stop and explain the limitation
before running the command without a cap.

## Ask before installing or changing configuration

Before installing a tool, changing a shell profile, creating a service or
agent, or changing system-wide resource limits:

1. Tell the user why the additional control is useful.
2. Recommend the smallest suitable option and explain its tradeoffs.
3. Ask the user whether they want the tool installed or the configuration
   changed.
4. Do not install packages or make persistent changes without approval.

If a built-in, temporary command is sufficient, use it without asking for
package-installation approval. If the command is likely to threaten system
stability and no adequate limit is available, ask before running it.

## Platform recommendations

### Linux

- Prefer `systemd-run --user --scope` with `MemoryMax`, `MemorySwapMax`, and
  `CPUQuota`. Ubuntu, Debian, Fedora, and many other desktop distributions
  normally provide systemd and cgroup v2.
- Use `nice -n 10` or a higher value as an optional responsiveness aid. `nice`
  changes scheduling priority; it is not a CPU usage limit.
- Use `taskset` when the goal is to restrict a job to specific cores. It limits
  CPU affinity, not the percentage of CPU consumed by those cores.
- If cgroup control is unavailable, recommend `cpulimit` as an optional,
  user-approved fallback. It can throttle a percentage of CPU but uses process
  suspension and resumption, so it is less robust than cgroups for complex or
  interactive process trees. Install it with the distribution's package
  manager only after approval, then invoke it as
  `cpulimit --limit=<percent> -- <command> ...`.
- Do not install another service manager merely to obtain resource limits.
  First check whether the existing user systemd session or cgroup delegation
  can be used.

### macOS

- Use the built-in `nice` command for lower scheduling priority and the
  wrapper's `ulimit` memory protection for temporary limits. Neither is a
  reliable percentage-based CPU cap.
- If a percentage-based CPU cap is important, recommend the optional Homebrew
  `cpulimit` formula (`brew install cpulimit`) and ask for approval before
  installing it. Explain that it uses process suspension/resumption and may
  behave poorly with interactive jobs or detached child processes. Use it as
  `cpulimit --limit=<percent> -- <command> ...` after installation.
- Do not recommend Linux-only tools such as `systemd-run`, cgroups, or
  `taskset` for macOS.
- Avoid changing global `launchctl` limits or login configuration unless the
  user explicitly requests persistent system-wide behavior. Prefer a
  per-command wrapper.

When presenting a recommendation, state whether it provides a hard limit, a
best-effort throttle, a priority change, or only CPU affinity. Do not describe
`nice` as a CPU cap.

```text
scripts/limited-run.sh <memory-max> <cpu-quota-percent> -- <command> [args...]
```

- `<memory-max>`: a size such as `2G`, `6G`, or `512M`.
- `<cpu-quota-percent>`: a percentage of one core; `400` means up to four
  cores on Linux. It is accepted but not enforced by the macOS fallback.

## Starting presets

Pick the smallest preset that realistically fits the task. These values are
starting points for a machine with several CPU cores and roughly 30 GB of RAM;
tune them for the host and command.

| Workload | Command shape | Memory | CPU |
|---|---|---|---|
| Small or unit test suite | `.../scripts/limited-run.sh 2G 400 -- <test-command> ...` | 2G | 400% |
| Large or integration test suite | `.../scripts/limited-run.sh 6G 400 -- <test-command> ...` | 6G | 400% |
| Static analysis or build | `.../scripts/limited-run.sh 4G 400 -- <analysis-or-build-command> ...` | 4G | 400% |
| Formatter or lightweight linter | `.../scripts/limited-run.sh 1G 200 -- <lint-command> ...` | 1G | 200% |
| Browser automation | `.../scripts/limited-run.sh 6G 400 -- <browser-test-command> ...` | 6G | 400% |
| Standalone browser driver | `.../scripts/limited-run.sh 6G 400 -- <browser-driver> ...` | 6G | 400% |

Adjust the path to `limited-run.sh` to wherever this skill directory resolves
on disk.

## Wrap child processes too

If the command starts a separate service, driver, worker, or browser process,
wrapping only the parent command may not limit the child. Start standalone
child processes through the wrapper as well:

```bash
scripts/limited-run.sh 6G 400 -- <child-process-command> ... &
```

Use the tool's own low-resource or headless options when available. Those
options reduce baseline usage but do not replace the process-level cap.

## After a run

Background processes can keep running after the command finishes. Check for
and stop them explicitly rather than leaving them to accumulate:

```bash
ps aux | grep -iE "<driver>|<browser>|<worker>" | grep -v grep
```
