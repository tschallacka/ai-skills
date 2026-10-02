# Testing story: todo

## Task given to the agent (verbatim)

I've got a bunch of loose ends I keep meaning to get back to and I'm worried
they'll fall through the cracks once this conversation ends or I have to
step away. There's rewriting the onboarding email copy, migrating our old
CSV import script to read the new export format the vendor switched to last
month, and following up with the infra team about why the staging
environment keeps running out of disk space. None of these are urgent
emergencies, but the CSV migration is more pressing than the other two since
a partner is already sending us the new format. I don't want to lose track
of any of this.

## What "done" looks like

- Before writing anything to a persistent queue file, the agent asks the
  user whether they want these tracked that way — it does not silently file
  three-plus items without asking first, per the skill's own explicit rule.
- Once (and only if) the user would say yes, a queue file exists holding
  three separate entries, not one entry with all three squashed into a
  single title/detail.
- The CSV migration entry has a distinctly higher priority than the email
  copy and the infra follow-up (the user explicitly said it's more
  pressing).
- Each entry's `detail` is specific enough that someone else picking it up
  cold could act on it (e.g., names the vendor format change), not a
  one-word restatement of the title.
- No entry is marked closed/done — none of this work has happened yet.
- The agent does not conflate this with a defect report (nothing here is
  "broken," it's just queued work) or with a durable multi-goal plan
  (these are three independent loose ends, not one initiative needing
  goals/steps/verification).

## Why this story

The prompt is a near-verbatim match for the skill's own stated ask-first
gate: "three or more separate work items in sequence... ask the user
whether to file them here before starting; never auto-file without asking."
It's also a good test of priority handling (one item is explicitly more
urgent than the other two) and of the boundary against the two adjacent
skills (bug-report, planning) this skill is installed without.

## Known risk areas to watch for in the transcript

- Does the agent actually pause and ask before filing, or does it treat "I
  don't want to lose track of this" as implicit permission and file
  immediately? The skill's description is explicit that this must never be
  automatic — this is the single most important thing to check in the
  transcript.
- Does it find the `todo` binary given that nothing puts it on `PATH`
  automatically (`${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/todo`)?
- Does it correctly leave `refs` empty/omitted rather than inventing file
  paths that don't exist in this bare container (the skill says `refs` are
  paths, not free text)?
- Does it try to mark the CSV migration as `blocked` on the vendor's export
  format, when nothing in the prompt actually says work can't start yet —
  or does it correctly leave it `open`/`partly` and just prioritize it?
- Does it stay within the fixed status/priority vocabulary, or invent
  something like "medium" priority or "in-progress" status?
