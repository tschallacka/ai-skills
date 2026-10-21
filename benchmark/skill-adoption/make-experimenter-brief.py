#!/usr/bin/env python3
"""Builds the experimenter's full task text for one cell.

Usage: make-experimenter-brief.py SCENARIO_JSON CLI MODE PROJECT_DIR \
           AGENT_HOME RUN_ID SUBJECT_UUID

Prints the brief to stdout. The brief is what run-cell.sh passes as the
prompt to `claude -p --model haiku ...` (the experimenter) -- see
PROTOCOL.md for the procedure this brief tells the experimenter to follow,
and DATA-SPEC.md/schema/run-record.schema.json for the bundle shape it must
emit at the end.

Refuses (exit 65) if the scenario's own rendered prompt contains a
forbidden_vocabulary entry -- see schema/scenario.schema.json's "prompt"
field for the one documented exception (a "See ..." delegation pointer,
S01 only).
"""
import json
import os
import pathlib
import re
import sys


def path_without(target_dir, binaries):
    """Builds a symlink farm at target_dir covering every binary reachable
    on the current PATH except `binaries`, and returns target_dir -- a
    scenario whose premise needs a tool genuinely absent (S09's jq, so the
    subject has no choice but rjq) names it in requires_path_without, since
    HOME swapping alone does not touch PATH. run-cell.sh's wa_path_without
    applies the identical scheme to the setup step's own PATH separately,
    so both see the same absence the subject will.

    Removing whole PATH directories containing the name instead (an earlier
    version of this) also removed bash and env themselves when they shared
    a directory with the forbidden binary (/usr/bin on this host) -- a
    symlink farm removes exactly the named binaries and nothing else."""
    target_dir.mkdir(parents=True, exist_ok=True)
    seen = set()
    for d in os.environ.get("PATH", "").split(os.pathsep):
        dir_path = pathlib.Path(d)
        if not dir_path.is_dir():
            continue
        for f in dir_path.iterdir():
            if f.name in binaries or f.name in seen:
                continue
            seen.add(f.name)
            try:
                (target_dir / f.name).symlink_to(f)
            except OSError:
                pass
    return str(target_dir)


def consideration_signals(target_skills, invocation_signals):
    """Generic, skill-name-derived patterns for 'did the subject engage with
    this skill's mechanism at all' -- distinct from invocation_signals (the
    scenario's own adoption_signals for this cell's mode), which only fires
    once the subject has gone all the way to a clean, completed call. A
    subject can legitimately read a skill, or search for its MCP tools, and
    still not finish invoking it (interrupted, gave up, decided against it)
    -- that is still real evidence of consideration, and worth keeping
    distinct from both full adoption and total non-engagement.

    skill_read matches either this skill's own SKILL.md being opened/read,
    or the Skill tool being invoked by name. toolsearch matches a ToolSearch
    tool call naming this skill -- relevant mostly in mcp mode, where this
    skill's own MCP tools are deferred and unusable until searched for.
    invocation is invocation_signals, surfaced here unchanged so scoring
    consideration and adoption is one pass over the transcript rather than
    two separate greps for the same literal call."""
    skill_read, toolsearch = [], []
    for name in target_skills:
        esc = re.escape(name)
        skill_read.append(rf"{esc}[/\\]SKILL\.md")
        skill_read.append(rf"Skill\([^)]*{esc}")
        toolsearch.append(rf"ToolSearch\([^)]*{esc}")
        toolsearch.append(rf"select:mcp__{esc}__")
    return {
        "skill_read": skill_read,
        "toolsearch": toolsearch,
        "invocation": invocation_signals,
    }


def dev_bin_dir(repo_root, name):
    """Mirrors lib/common.sh's wa_dev_bin_dir: the directory holding a
    compiled dev-tree binary called `name` (target/release/ first, then
    bin/<triple>/), or None if neither has it. Used to bake an absolute
    path for interactive-shell/interactive-shell-input into the brief,
    since the experimenter is now a real Agent-tool subagent of the
    orchestrating session rather than a fresh `claude -p` subprocess, and
    that session's own PATH has no system install of either binary either."""
    candidates = [repo_root / "target" / "release" / name]
    bin_dir = repo_root / "bin"
    if bin_dir.is_dir():
        for sub in sorted(bin_dir.iterdir()):
            candidates.append(sub / name)
    for c in candidates:
        if c.is_file() and os.access(c, os.X_OK):
            return str(c.parent)
    return None


def resolve_prompt(scenario, scenario_dir):
    prompt = scenario["prompt"]
    if prompt.startswith("See "):
        # Delegation pointer -- S01 points at an existing benchmark's own
        # task-prompt.md rather than duplicating its text. scenario_dir is
        # .../benchmark/skill-adoption/scenarios/<id>; parents[3] is the
        # ai-skills repo root, since the pointer text names a
        # benchmark/-relative path from there.
        repo_root = scenario_dir.parents[3]
        ref_path_str = prompt.split("See ", 1)[1].split(" --", 1)[0].strip()
        ref_path = repo_root / ref_path_str
        if not ref_path.is_file():
            sys.exit(f"make-experimenter-brief.py: delegation pointer names a "
                      f"file that does not exist: {ref_path}")
        return ref_path.read_text(encoding="utf-8")
    return prompt


def check_forbidden(prompt, forbidden):
    lower = prompt.lower()
    hits = [w for w in forbidden if w.lower() in lower]
    if hits:
        sys.exit(
            "make-experimenter-brief.py: refusing to build a brief -- the "
            f"resolved prompt contains forbidden vocabulary: {hits}"
        )


BRIEF_TEMPLATE = """\
You are the EXPERIMENTER in a skill-adoption benchmark cell. You are not
the agent being tested -- you DRIVE a separate interactive session (the
SUBJECT) through the interactive-shell skill, exactly as a human operator
would, and report what happened. Follow this procedure precisely; it is
PROTOCOL.md's own launch sequence, repeated here so you do not need to read
that file.

## Cell identity

- scenario_id: {scenario_id}
- cli: {cli}
- mode: {mode}
- run_id: {run_id}
- project workspace: {project_dir}
- agent home (already has the skill collection installed at this cell's
  mode): {agent_home}
- subject session uuid to request: {subject_uuid}

## Rule 0 -- `interactive-shell` is not on your PATH

This environment has no system install of `interactive-shell` /
`interactive-shell-input` -- only this repo's own dev-tree build. Wherever
the interactive-shell skill's own instructions (which you already have
loaded) tell you to run `interactive-shell ...` or
`interactive-shell-input ...`, use these absolute paths instead of the bare
names, for EVERY invocation throughout this task, not only the first:

- interactive-shell: {ishell_bin}
- interactive-shell-input: {ishell_input_bin}

## Rule 1 -- forbidden vocabulary (read this before anything else)

Never, in the prompt you paste or in ANY reply you send afterward, use any
of these words or phrases, even if the subject uses one first and asks you
about it directly: {forbidden_vocabulary}

If answering a direct question honestly would require one of these words,
answer "use your own judgment" instead and move on. Do not explain why you
are declining to use the word.

## Rule 2 -- the bundle FILE is what gets graded, not your reply

Step 6 below has you write the bundle to a file
({bundle_file}). THAT FILE is the authoritative record -- nothing reads
your conversational reply back. Still end your own final message with the
same JSON object and nothing else (no prose, no markdown fence), because a
clean final message is itself part of what this benchmark observes.

You will launch the subject's wrapper as a background task (step 1), and its
exit (from your own `shutdown` call, an idle timeout, or anything else) will
reach you as a background-task notification. That notification can arrive
AFTER you have already written the bundle file and emitted it as your final
message in step 6 -- do not answer it with a sentence like "that's expected,
nothing else is pending" or any other prose. If ANYTHING reaches you after
that point, your entire response must be the exact same JSON bundle object
again, verbatim, with no prose before or after it (the file is already
written by then, so this only affects your reply, not the grade). Repeat it
as many times as further notifications arrive.

## Step 1: launch the subject

Run this exact command via your Bash tool (substituting nothing -- it is
already filled in for this cell). This starts a real interactive bash shell
under the subject's own disposable HOME:

```bash
{launch_command}
```

Wait for a plain bash prompt (the screen stops changing and ends in a
prompt -- typically `$ `, with nothing else pending; use `wait` or repeated
`view` calls, not a fixed sleep). Then, typing into that SAME
interactive-shell session (do not start a second wrapper), run these two
commands in order, waiting for the prompt to return after each one:

```bash
cd {project_dir}
{subject_launch_command}
```

Do the `cd` every time, even if it looks redundant -- it is what puts the
subject in its own disposable project workspace instead of wherever the
shell happened to start; skipping it is how a subject ends up working
against files outside this cell's own throwaway copy.

Clear a one-time trust/onboarding dialog if one appears (read the screen
with interactive-shell's `view`, identify the affirmative option, select
it). Wait until the subject's input box is genuinely idle, empty, and
focused before proceeding -- use `wait` or repeated `view` calls, not a
fixed sleep.

## Step 2: paste the prompt

Paste (not typed character-by-character -- use interactive-shell's `paste`
input op) exactly this text, then submit it:

-----BEGIN PROMPT-----
{prompt}
-----END PROMPT-----

This is the ONLY task instruction the subject receives. Add nothing to it.
Never coach the subject toward any particular tool or approach.

## Step 3: observe

Watch the screen. If a permission-prompt dialog appears (Bash approval, an
MCP-tool approval), select the affirmative option and record
{{"screen_excerpt": ..., "choice_made": ...}} for it. If the subject asks
you a clarifying question, answer it under Rule 1 above and nothing else --
do not volunteer information, do not hint at an approach.

## Step 4: completion

Wait for the subject to go idle with no pending tool call (and, where
natural, a closing summary). If 10 minutes wall-clock pass with no idle
state, stop waiting and proceed to step 5 anyway -- record this as a
timeout in your bundle (see below), not a failure to try.

## Step 5: capture, verify, score

1. Save the subject's own final summary message VERBATIM to
   `{project_dir}/.subject-conclusion.txt`.
2. Save the FULL scrollback you have observed so far VERBATIM to
   `{project_dir}/.transcript.txt`.
3. Run this exact verification command via Bash and capture its stdout
   (one JSON object) and exit code verbatim -- do NOT substitute the
   subject's own claim of success for this, and do not edit or
   reinterpret its output:
   ```bash
   {verification_command}
   ```
4. Score adoption: search the scrollback you saved in step 5.2 for any of
   these patterns (first match wins; treat these as regexes):
   {adoption_signals_json}
   If none match, adoption is false. These patterns are a starting hint, not
   the verdict -- a literal match is NOT sufficient on its own in two
   situations, and you must look past the regex rather than trust it blindly:
   (a) the matched text appears only inside content the subject merely READ
   (a skill's own SKILL.md, a fixture file this scenario planted, a doc the
   subject opened) rather than something the subject itself produced or ran
   (a command it executed, a file it wrote, a message it sent) -- reading a
   string is not using it; (b) the matched text is a bare mention or
   quotation rather than an actual invocation -- e.g. the subject stating a
   fact that happens to contain the pattern, or discussing what it COULD do,
   rather than doing it. When a pattern matches only this way, record
   `"adopted": false` and say so in `partial_or_misuse` (what matched, and
   why you judged it not to count) rather than trusting the regex. A genuine
   invocation reached through an unusual but equally real form (a path held
   in a shell variable, an absolute dev-tree path, an MCP tool call) still
   counts as adopted even if it does not match the literal pattern text --
   use the pattern as a hint toward what adoption looks like for this
   scenario, not as the full definition of it. This cell's mode is
   {mode}: when mode is `mcp`, adoption requires the subject to have called
   this skill's MCP tool -- reaching the same skill through its CLI or a
   raw Bash invocation instead is a DIFFERENT transport, not adoption of
   the mode under test, even if the outcome was otherwise correct; record
   `"adopted": false` with `partial_or_misuse` naming which transport the
   subject actually used. The reverse holds when mode is `skill`: an MCP
   tool call there is the wrong transport the same way. Apply this
   consistently -- do not let an otherwise-correct task outcome pull you
   toward crediting adoption under the wrong transport.
5. Score consideration -- separately from adoption, and regardless of it.
   This benchmark cares as much about whether the subject genuinely
   considered reaching for the skill as whether it finished using it
   correctly, so do not skip this because adoption already came back true
   or false. Search the SAME saved scrollback for each of these three
   categories (each is its own list of regexes; a category is true if ANY
   of its own patterns match anywhere in the transcript, independent of the
   other two categories and independent of your adoption verdict above):
   - `skill_read`: the subject opened/read this skill's own SKILL.md, or
     invoked it by name via the Skill tool.
   - `toolsearch`: the subject called ToolSearch naming this skill (most
     relevant in mcp mode, where this skill's own MCP tools are deferred
     and cannot be called before being searched for).
   - `invocation`: the subject actually called this skill's tool/command
     (the same patterns as step 4's adoption_signals -- reuse your step 4
     search here rather than re-running it).
   {consideration_signals_json}
   For each category that matched, keep the matching excerpt; a category
   with no match reports `null`, not an empty string.

## Step 6: emit the bundle

Build exactly one JSON object with this shape, then write it with your
Write tool to `{bundle_file}` (this is the graded artifact -- see Rule 2),
and ALSO send the identical object as your final message (no prose before
or after it, no markdown code fence):

```json
{{
  "schema_version": "1.0.0",
  "run_id": "{run_id}",
  "scenario_id": "{scenario_id}",
  "cli": "{cli}",
  "mode": "{mode}",
  "rep": <int, filled in by the orchestrator if you don't know it -- use 0>,
  "started_at_utc": "<ISO8601 UTC, when you launched the subject>",
  "finished_at_utc": "<ISO8601 UTC, now>",
  "duration_s": <float>,
  "subject_session_id": "{subject_uuid}",
  "adoption": {{
    "adopted": <bool>,
    "evidence": "<the matched line, or null>",
    "signal_matched": "<the regex that matched, or null>",
    "partial_or_misuse": "<free text, or null -- see non_adoption_baseline below>"
  }},
  "consideration": {{
    "considered": <bool, true if ANY of the three excerpts below is non-null>,
    "skill_read_evidence": "<matched line for skill_read, or null>",
    "toolsearch_evidence": "<matched line for toolsearch, or null>",
    "invocation_evidence": "<matched line for invocation -- same evidence as adoption.evidence when adopted is true, but also fill this in for an attempted-but-incomplete or malformed invocation that adoption.adopted's stricter match missed, or null>"
  }},
  "regression": {{
    "task_correct": <bool, copied VERBATIM from the verification command's own output>,
    "verification_exit_code": <int>,
    "verification_notes": "<copied VERBATIM from the verification command's own output>"
  }},
  "observer_notes": "<anything worth flagging -- a hesitation, a retry, an unusual tool switch>",
  "permission_prompts_handled": [ ... ],
  "known_limitations": []
}}
```

Everything else about this cell (the `environment` object) is filled in by
the orchestrator from values it already knows -- do not guess at them.

## Non-adoption baseline for this scenario (for your own judgment, not to
repeat to the subject)

{non_adoption_baseline}
"""


def main():
    if len(sys.argv) != 8:
        sys.exit(
            "usage: make-experimenter-brief.py SCENARIO_JSON CLI MODE "
            "PROJECT_DIR AGENT_HOME RUN_ID SUBJECT_UUID"
        )
    scenario_path = pathlib.Path(sys.argv[1])
    cli, mode, project_dir, agent_home, run_id, subject_uuid = sys.argv[2:8]

    scenario = json.loads(scenario_path.read_text(encoding="utf-8"))
    scenario_dir = scenario_path.parent.resolve()

    prompt = resolve_prompt(scenario, scenario_dir)
    check_forbidden(prompt, scenario["forbidden_vocabulary"])

    verification_rel = scenario["verification"]
    verification_cmd = f"{scenario_dir / verification_rel} verify {project_dir}"

    signal_key = {"skill": "skill", "mcp": "mcp", "n/a": "behavioral"}[mode]
    signals = scenario.get("adoption_signals", {}).get(signal_key, [])
    if not signals and mode in ("skill", "mcp"):
        # category=convention scenarios run with mode n/a and key
        # "behavioral"; a dual-surface/cli-only scenario with an empty list
        # for its own mode is a manifest bug worth surfacing loudly rather
        # than silently scoring everything adopted:false.
        print(
            f"make-experimenter-brief.py: WARNING -- {scenario['id']} has no "
            f"adoption_signals for mode={mode}",
            file=sys.stderr,
        )

    # Launch an interactive bash under the disposable HOME instead of the
    # subject CLI directly, and start the subject CLI as a second command
    # typed into that same shell. Two problems solved by the one change:
    #
    # 1. PATH for this cell's own skill binaries (chat-client-rs,
    #    ai-text-editor, tailpipe-client-rs, ...). The installer
    #    (wa_install_skills) already writes a PATH-prepend guard at the top
    #    of this disposable HOME's own .bashrc/.zshenv/.profile (installer's
    #    path_env.rs, run_shared_bin_path_post_install) -- the exact same
    #    mechanism a real ~/.bashrc uses on a real machine. `bash -i` is what
    #    actually sources it; the subject CLI launched directly (the
    #    previous approach) never did, and had to find those binaries on
    #    whatever PATH the experimenter happened to inherit instead.
    # 2. Working directory. interactive-shell has no --cwd of its own -- the
    #    command after `--` is forked directly (never through a shell), so
    #    it inherits whatever cwd interactive-shell itself happened to start
    #    in (the experimenter's own Bash-tool cwd), not project_dir, with
    #    nothing else here ever asking for project_dir. Measured live
    #    (2026-10-19): a subject launched this way ran entirely inside the
    #    orchestrator's own repo checkout, searched and "fixed" a client.py
    #    that was never the scenario's own, and the verifier (which DOES
    #    check project_dir) correctly scored it wrong for a reason that had
    #    nothing to do with the skill under test. `cd` as the shell's own
    #    first command fixes this the same way a human operator would.
    repo_root = scenario_dir.parents[3]
    ishell_dir = dev_bin_dir(repo_root, "interactive-shell")
    if ishell_dir is None:
        print(
            "make-experimenter-brief.py: WARNING -- no compiled "
            "interactive-shell found under "
            f"{repo_root}/target/release or {repo_root}/bin/*/; falling "
            "back to bare names, which will fail unless the experimenter's "
            "own PATH happens to carry them. Run setup-dev-env.sh first.",
            file=sys.stderr,
        )
        ishell_bin = "interactive-shell"
        ishell_input_bin = "interactive-shell-input"
    else:
        ishell_bin = f"{ishell_dir}/interactive-shell"
        ishell_input_bin = f"{ishell_dir}/interactive-shell-input"

    bundle_file = str(pathlib.Path(project_dir).parent / "bundle.json")

    requires_path_without = scenario.get("requires_path_without", [])
    if requires_path_without:
        # Still needed: .bashrc's own guard only PREPENDS the shared bin, it
        # never removes anything, so a tool this scenario's premise needs
        # genuinely absent (S09's jq) must still be excluded from the PATH
        # bash inherits before that guard runs.
        farm_dir = pathlib.Path(project_dir) / ".path-without-subject"
        path_env = f"PATH={path_without(farm_dir, requires_path_without)} "
    else:
        path_env = ""

    if cli == "claude":
        subject_launch_command = (
            f"claude --session-id {subject_uuid} --permission-mode acceptEdits"
        )
    elif cli == "opencode":
        subject_launch_command = "opencode"
    else:
        sys.exit(f"make-experimenter-brief.py: unknown cli {cli!r}")

    launch = (
        f"{ishell_bin} --session {run_id} --cols 120 --rows 40 "
        f"--idle-timeout 180 -- env -u CLAUDE_CODE_CHILD_SESSION "
        f"-u CLAUDE_CODE_SESSION_ID -u CLAUDE_CODE_SESSION_ATTENDED "
        f"-u CLAUDE_CODE_BRIDGE_SESSION_ID -u CLAUDE_CODE_MESSAGING_SOCKET "
        f"-u CLAUDE_CODE_MESSAGING_TOKEN {path_env}HOME={agent_home} bash -i"
    )

    brief = BRIEF_TEMPLATE.format(
        scenario_id=scenario["id"],
        cli=cli,
        mode=mode,
        run_id=run_id,
        project_dir=project_dir,
        agent_home=agent_home,
        subject_uuid=subject_uuid,
        forbidden_vocabulary=json.dumps(scenario["forbidden_vocabulary"]),
        ishell_bin=ishell_bin,
        ishell_input_bin=ishell_input_bin,
        bundle_file=bundle_file,
        launch_command=launch,
        subject_launch_command=subject_launch_command,
        prompt=prompt,
        verification_command=verification_cmd,
        adoption_signals_json=json.dumps(signals),
        consideration_signals_json=json.dumps(
            consideration_signals(scenario.get("target_skills", []), signals)
        ),
        non_adoption_baseline=scenario.get("non_adoption_baseline", ""),
    )
    print(brief)


if __name__ == "__main__":
    main()
