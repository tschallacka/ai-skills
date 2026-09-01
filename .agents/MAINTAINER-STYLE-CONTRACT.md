<!-- MODE: PROD -->
# Style & evidence contract

**Audience: agents and maintainers.** The repo-wide contract for how documents
are written, how generated formats are changed, and how evidence is treated.
The planning skill's specific formats (canonical plan documents, table schemas,
status values, placeholders) live in `planning/MAINTAINER-STYLE-CONTRACT.md`;
this file holds the parts that apply to the repository as a whole.

## Source of truth

- Creation/generator scripts define canonical document order, headings, tables,
  labels, and initial values. The validator is the readiness and completion
  gate; **do not weaken it** to accommodate a malformed fixture — update the
  fixture through the helpers.
- Every hard rule needs a regression assertion in the tests directory.
- Hash drift is a suspect external edit. Helpers report it for human
  intervention and must not silently overwrite or repair the document.
- Flagged, deterministic mutation helpers (paragraph/section flags, inserts)
  own document edits; never add a second numbering scheme or hand-reflow labels.

## Historical evidence and protocol boundaries

- Benchmark reports and archives are immutable evidence of the code and
  protocol that produced them.
- Do not rerun an older version or retrofit an older report to satisfy a newer
  protocol. This repository does not add backward compatibility for that
  purpose.
- Compare archived data as-is only when its task, revision, metadata, and
  evidence boundaries are compatible. Otherwise record the comparison as
  unavailable or contextual, and run only the current protocol when new
  evidence is genuinely required.

## Shared conventions

- Directory/file names use lowercase kebab case where the skill's contract
  does not specify otherwise; follow the skill's own naming rule when it does.
- A table paragraph is the controlled multiline exception to single-line
  paragraph rules: CSV rows become a header row, separator row, and data rows
  under one paragraph label. Table values cannot contain `|` or newlines.
- Do not add decorative columns, omit empty cells, or put an unescaped `|` in a
  cell. Use the dedicated helper for structured table mutations (do not add a
  second numbering scheme or hand-reflow labels).

## Change checklist (generated formats)

When changing a generated format:

1. Update the creating helper and any parser/validator that consumes it.
2. Update the flagged mutation helper if the document is mutable.
3. Add or update a regression fixture covering the positive and malformed
   forms.
4. Run shell syntax checks, `git diff --check`, and all bounded tests. When a
   registry, reader scope, voice, or reader-composition changes, re-run the
   corresponding drift tests.
5. Keep the contract precise and keep agent-facing prose limited to workflow
   decisions rather than repeating implementation details.