# Progress: 03-untrack-reviewer-md

**Progress:** `100%  ####################  100%` ✅

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 03-untrack-reviewer-md | 01-step-ignore-reviewer | Add planning/REVIEWER.md to .gitignore with a comment naming generate-reviewer.sh and the SKILL.md S... | ✅ completed |
| 03-untrack-reviewer-md | 02-step-reviewer-test-build-to-temp | Rewrite from compare-against-committed to build-and-verify: run generate-reviewer.sh to a temp outpu... | ✅ completed |
| 03-untrack-reviewer-md | 03-step-role-context-refusal | When the skill tree has no REVIEWER.md, refuse with an actionable message naming generate-reviewer.s... | ✅ completed |
| 03-untrack-reviewer-md | 04-step-context-listing-guard | When REVIEWER.md is absent from the source root, omit it from the context source listing with a stat... | ✅ completed |
| 03-untrack-reviewer-md | 05-step-release-generate-reviewer | Run planning/scripts/generate-reviewer.sh before collect when REVIEWER.md is missing from the tree b... | ✅ completed |
| 03-untrack-reviewer-md | 06-step-capsule-generate-reviewer | Generate REVIEWER.md into the capsule when absent instead of copying only if present, so every capsu... | ✅ completed |
| 03-untrack-reviewer-md | 07-step-maintainer-reviewer-doc | Align the artifact-map REVIEWER.md row with the untracked reality: generated on demand by generate-r... | ✅ completed |
| 03-untrack-reviewer-md | 08-step-verify-reviewer | Remove REVIEWER.md from a scratch checkout: role-context.sh fails with the named fix; test-reviewer-... | ✅ completed |
