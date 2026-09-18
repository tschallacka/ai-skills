# Testing story: ci-failures

## Task given to the agent (verbatim)

The CI run for my current branch just went red. Can you tell me what
actually broke?

## What "done" looks like

- The agent identifies which forge to use (GitHub vs GitLab) from the
  repository's own git remote, and says which one it's using rather than
  silently picking one.
- It resolves "my current branch" to the newest run/pipeline for that
  branch, without needing a run id or PR number spelled out for it.
- Its final answer names the actual failing job(s) and the specific lines
  that identify the failure (a panic, a `Failed:`/`test result:` summary, a
  `##[error]` annotation) — not just "the build failed," and not a raw,
  unfiltered log dump.
- If nothing in a job's log matches the tool's known failure patterns, the
  agent says so plainly and offers the full log (`--raw`) rather than
  staying silent or guessing at a cause.
- The agent does not attempt to trigger a new CI run, and does not confuse
  this task with writing a fix — the task is diagnosis only.

## Why this story

"What broke in the CI run for my branch" with a bare git remote and no run
id or PR number given is the single most common real-world shape of this
request, and exercises the skill's own disambiguation logic (which forge,
which run) with the least amount of hand-holding a user would ever give.

## Known risk areas to watch for in the transcript

- **This story needs one more piece of setup than any other story here**,
  worth stating plainly: `ci-failures` needs `gh`/`glab` (now installed in
  every `testing-stories/docker/Dockerfile.*` image), an authenticated
  session against a real forge (`run-story.sh` now forwards `GH_TOKEN`/
  `GITHUB_TOKEN`/`GITLAB_TOKEN`/`GLAB_TOKEN` from the host environment when
  set), and a git remote pointing at a repository with actual CI history —
  which no `.fixture.sh` can fabricate, since it requires a real GitHub
  Actions/GitLab CI run to have actually happened somewhere. The
  straightforward way to run this one for real: point the workspace's git
  remote at this very repository (`ai-skills` itself has real GitHub Actions
  history, almost certainly including a failed run somewhere in its
  history) with a `GH_TOKEN` that can read it, rather than trying to seed a
  synthetic failing run locally.
- Once runnable for real: does the agent correctly discover
  `scripts/ci-failures.sh` and invoke it plainly, or does it try `gh run
  view`/`glab pipeline` directly by itself instead of the skill's own
  wrapper — a real signal about whether "run scripts/ci-failures.sh instead
  of reading a CI run by hand" is prominent enough in the doc.
- Whether the agent correctly treats a bare number it might construct
  (e.g. if it tries to look up a PR number itself) with the documented
  9-digit heuristic, or gets confused between a run id and a PR/MR number —
  SKILL.md calls this out explicitly as a known ambiguity, so a fresh agent
  stumbling on it would be a real, expected-and-covered case, not a new gap.
- Whether the agent reports which forge it used on the FIRST line of its
  own answer (mirroring what the script itself does), or buries/omits that
  — SKILL.md treats "never a silent choice" as important enough to call out
  twice.
