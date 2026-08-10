# Bug register and risks

| ID | Risk/bug | Impact | Detection | Disposition |
|---|---|---|---|---|
| B-001 | Initial button counted as generated | Terminal transition occurs one press early/late | Record counts and generated counter semantics | Mitigated in plan; verify during implementation |
| B-002 | Multiple listeners or handlers append more than once | Violates exactly-one append contract | Compare count delta after each activation | Open until browser verification |
| B-003 | New button is not the current last target | Chain can skip or target an older button | Check DOM order and target after each press | Open until browser verification |
| B-004 | Completion string has wrong case or extra text | Exact-text acceptance fails | Inspect visible text equality | Open until browser/static verification |
| B-005 | White border is absent, transparent, or zero width | Completion styling is not visible | Browser visual check and static declaration check | Open until implementation |
| B-006 | Validator unavailable in tagged revision | Cannot obtain tagged validator exit 0 | Final command result recorded in validation.md | Environment limitation; not fabricated |
