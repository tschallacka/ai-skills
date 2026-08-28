<!-- MODE: PROD -->
# Merge request etiquette

**Descriptions a reviewer can act on, in your voice, without the process noise.**

How to write the description on a merge request or pull request: whose voice it is
in, the one-paragraph TLDR that opens it, what the body says about the fix, and the
single case where a collapsible block earns its place.

## What you get

- **Your voice, not an assistant's.** The description reads as written by the person
  whose name is on the request, because that is who authored the change. No mention
  of tooling, no "as requested", no process narration.
- **A TLDR that actually is one.** One short paragraph at the top saying what the
  change does and why. A reviewer who reads nothing else knows what they are
  approving.
- **A body about the fix.** The defect, the cause, the change. No headings for their
  own sake, no restating the diff the reviewer is already looking at.
- **Derived from the commits.** The reasoning is usually already in `git log` for the
  branch, written when the work was fresh. If it is not, that is worth fixing where
  the commits are still yours to rewrite.
- **No chat transcripts, anywhere.** Not in the body, not hidden in a collapsible
  block. A fact worth keeping gets stated plainly; the conversation around it does not
  travel.
- **One collapsible block, conditionally.** For evidence a reviewer needs that the
  commits cannot carry — a captured upgrade log, an external tool's diff summary.
  Three conditions, all required, and everything else stays out.

## When to use it

Reach for it when opening a merge request or pull request, or when revising a
description that has grown into a narrative. It does not cover commit messages,
review comments, or release notes.
