---
name: merge-request-etiquette
description: Use when writing or revising the description of a merge request or pull request. Covers the voice it is written in, the TLDR that opens it, what belongs in the body, and the one case where a collapsible section earns its place. Do not use for commit messages, code review comments, or release notes.
---
<!-- MODE: PROD -->

# Merge request and pull request etiquette

A merge request description is read by a reviewer deciding whether to approve, and
later by whoever is bisecting a regression. Both want the same thing: what changed,
what it fixes, and nothing else.

## The description is written in the driving human's voice

The person whose name is on the merge request is the author. Write as them.

- First person singular is theirs: "I moved the retry into the handler", not "the
  assistant moved" and not "we moved" unless more than one person actually worked on it.
- Never mention the agent, the session, the model, or the fact that any of this was
  assisted. A reviewer is reviewing a change, not a process.
- Never write "as requested", "per your instructions", or anything else that implies
  the author was taking direction. They are the author.
- Never link, quote, paste, or summarise a chat transcript. Not in the body, not in a
  collapsible section, not as a reference. The reasoning that matters belongs in the
  description in the author's own words; the rest is not evidence a reviewer can use.

If a fact only exists in a conversation, either state the fact plainly on its own or
leave it out. "The retry limit is three because the upstream times out at four" is a
fact. "We discussed the retry limit and settled on three" is a transcript.

## Open with a TLDR

Every description starts with a `TLDR` heading and **one short paragraph**. Not a
list, not two paragraphs, not a sentence per change. It answers: what does this change
do, and why.

```markdown
## TLDR

Orders placed with a gift card no longer lose the card balance when payment fails.
The balance was released before the payment result was known; it now releases only
after the gateway confirms, and a failed payment leaves the card untouched.
```

A reviewer who reads only the TLDR should be able to say what the merge request is
for. If it takes more than a paragraph, the merge request is probably too big.

## The body says what was fixed

After the TLDR, keep it short and concrete. Name the defect, the cause, and the change.
Prose, not ceremony:

- No "Changes", "Motivation", "Background", "Testing" headings unless the project's
  template demands them. Headings for their own sake pad a description that should
  fit on one screen.
- No restating the diff. The reviewer has the diff. Say what it does that the diff
  does not make obvious.
- Name the reproduction if there is one, and say what now happens instead.
- One line per genuinely separate fix. If the list runs past five, the merge request
  is doing too much.

Link the issue or ticket if the project uses one. That is a reference a reviewer can
act on; a chat log is not.

## Derive it from the commits first

Before writing anything, read `git log` for the branch. Almost everything a description
needs is already there, written when the work was fresh:

```bash
git log --oneline origin/<target>..HEAD
git log origin/<target>..HEAD          # full messages: the why is usually here
git diff --stat origin/<target>..HEAD  # the shape of the change
```

If the commit messages do not carry the reasoning, that is worth fixing at the source
where the branch is still yours to rewrite. A description that explains what a commit
message should have said leaves the explanation somewhere `git blame` will never find.

Read them from the working branch, before you squash. The squashed commit is written
from what you find here, so the order is: read the route, write the description, then
cut the request branch and squash into it.

## Open the request from its own branch, squashed

A merge request gets a branch of its own, cut for the request and holding one commit.
Do not open it from the working branch.

```bash
git checkout -b mr/<short-subject> origin/<target>
git merge --squash <working-branch>
git commit                       # one message: the TLDR, then what was fixed
git push -u origin mr/<short-subject>
```

The reason is what a reviewer is asked to do. A working branch carries the route the
work took — a fix, a correction to the fix, a test that failed, a rename, a revert of
something tried and abandoned. That history is worth having while the work is live and
is noise to someone deciding whether to approve the result. Squashing to one commit
puts the change on the reviewer's screen as the thing it is, not as the sequence that
produced it.

Cutting a separate branch rather than squashing the working branch in place keeps the
route intact where it is still useful: the working branch stays as it was, so the
commits and their reasoning are still reachable if a question comes up mid-review, and
you have not rewritten a branch a colleague may have pulled.

The squashed commit's message is the description: the TLDR paragraph first, then what
was fixed. Write it once and reuse it as the request body — if they disagree, one of
them is wrong, and it is usually the one written second.

Two cases where this changes:

- **The project asks for something else.** A repository that rebases, or that wants
  one commit per logical change, has decided already. Follow it.
- **The commits are genuinely separate changes.** If squashing would fuse two unrelated
  fixes into one commit, that is a sign the request should be two requests, not that
  the squash is wrong.

## The collapsible section, and when it is allowed

Some information genuinely cannot be derived from the commits: a captured upgrade log,
a long diff summary from an external tool, a list of warnings a dependency emitted
during a version bump. It belongs in the merge request because a reviewer needs it, and
it would drown the description if inlined.

Put that, and only that, in a collapsible block. GitLab and GitHub both render HTML
`<details>`:

```markdown
<details>
<summary>View Magento update log</summary>

```
composer require magento/product-community-edition:2.4.7-p10 -W
DIFF WARNING: app/design/frontend/.../attribute.phtml
DB CHANGE WARNING: ! lib/internal/Magento/Framework/Setup/...
```

</details>
```

Three conditions, all of them:

1. The content cannot be derived from the commits or the diff.
2. A reviewer would actually want it — it is evidence, not padding.
3. The platform renders `<details>`. If it does not, and the content still qualifies,
   attach it or link the build artifact rather than inlining it.

Everything else stays out. A collapsible section is not a place to hide a chat log, a
narrative of how the work went, or a description that should have been shorter.

## Before you post

Read the description as the reviewer. It should answer, in order: what does this do
(TLDR), what was broken and how is it fixed (body), and where do I look if I need more
(issue link, or the one collapsible block). If a sentence serves none of those, cut it.
