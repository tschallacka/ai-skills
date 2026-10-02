<!-- MODE: DEV -->
# Why `grep` sometimes warns "stray \ before -" for no reason in this repo

Measured 2026-09-09. **`grep` is not `grep` inside a Claude Code Bash tool
session.** Claude Code installs a shell *function* named `grep` that shadows
the real binary and routes calls through `ugrep`, running as the `claude`
binary itself:

```
$ type grep
grep is a function
grep ()
{
    ...
    ( exec -a ugrep "$_cc_bin" -G --ignore-files --hidden -I \
        --exclude-dir=.git --exclude-dir=.svn ... ${1+"$@"} )
}
```

A shell function always wins over a same-named binary on `$PATH`, so this
shadow survives `nix develop --command bash ...` too — a devshell's `$PATH`
cannot override it. Any script this session runs through `bash -c` inherits
the function, since it is exported.

**What it does that matters:** `ugrep` is not byte-for-byte GNU grep. At least
two patterns already in this repo's own test suite — an escaped literal
hyphen inside an ERE, `\-` (`tests/test-register-schemas.sh:34`,
`planning/tests/test-atomicity-flow.sh:54`) — can make it print an advisory
`warning: stray \ before -` to stderr. Confirmed harmless: `command grep`
(bypassing the function, real GNU grep 3.12) runs the identical pattern with
the identical result — same match count, same exit status, no warning.

```
$ printf '1.2.3-beta\n' | command grep -Ec '^[0-9]+\.[0-9]+\.[0-9]+(\-[0-9A-Za-z.-]*)?$'
1
$ printf '1.2.3-beta\n' | grep -Ec '^[0-9]+\.[0-9]+\.[0-9]+(\-[0-9A-Za-z.-]*)?$'
1
warning: stray \ before -   # only sometimes -- see below
```

## What it means here

A `grep: warning: stray \ before -` line seen while running this repo's tests
or `pre-push-check.sh` **from inside a Claude Code session** is not evidence
of a bug in the script. Verify with `command grep` (or check `type grep`
first) before concluding a regex needs fixing or filing a register entry.
Real CI runners never see this: they run a genuine `grep`, not a Claude Code
Bash-tool session, so the warning is invisible to anyone but an agent working
in one interactively.

The `\-` escapes themselves are still slightly non-idiomatic (redundant
outside a bracket expression) but not wrong anywhere the suite actually runs
— nothing here calls for changing them.

## Caveat: the warning was not reproducible on every retry

Re-running the identical pattern through the shadowed `grep` did not print
the warning every time in a quick spot-check — the exact trigger condition
(pipeline shape? pre-existing invocation in the same shell? something ugrep
caches) was not isolated. Treat "no warning this run" as inconclusive, not as
proof the shim stopped applying; `type grep` is the reliable check for
whether the shadow is present at all.

## Re-checking it

`type grep` in the session in question. If it prints a function body
mentioning `ugrep` and `CLAUDE_CODE_EXECPATH`, the shim is active. Compare
`command grep` against plain `grep` on the pattern in question before trusting
either one's stderr.
