# Bug register

| ID | Risk/bug | Severity | Detection/impact | Disposition |
|---|---|---|---|---|
| B-001 | Interpret fourth button overall instead of fourth generated | High | Terminal fires one click early | Resolved in plan contract and UI cache |
| B-002 | Handler appends more than one button | High | Count jumps by more than one | Guarded by exact count scenarios |
| B-003 | Non-last button can append | Medium | Chain branches or violates linear behavior | Guarded by non-last scenario |
| B-004 | Completion text case or border is wrong | High | Acceptance mismatch | Guarded by exact text/style assertion |
| B-005 | Tagged validator unavailable at requested path | Medium | Cannot truthfully report tagged script exit 0 | Open environmental limitation; equivalent check recorded |

No implementation defects were observed because implementation was prohibited.
