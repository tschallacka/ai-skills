# Atomic work-unit inventory

| ID | Work unit | Owner | Depends on | Acceptance evidence | Proof status |
|---|---|---|---|---|---|
| WU-01 | Create standalone document shell and one initial button | 01-button-chain | none | Initial browser count is exactly one | Planned; not executed |
| WU-02 | Track generated-button count independently of initial button | 01-button-chain | WU-01 | Count semantics identify the fourth generated button | Planned; not executed |
| WU-03 | Handle current-last-button activation with exactly one append below it | 01-button-chain | WU-01, WU-02 | Counts 2, 3, 4 after first three activations; DOM order remains correct | Planned; not executed |
| WU-04 | Implement terminal fourth-generated-button clear transition | 01-button-chain | WU-02, WU-03 | Fourth generated activation leaves no button | Planned; not executed |
| WU-05 | Render exact completion text and visible white border | 01-button-chain | WU-04 | Visible text is exactly `finished`; border is nonzero white | Planned; not executed |
| WU-06 | Run browser and static acceptance verification | 01-button-chain | WU-01..WU-05 | Testing companion pass record | Intentionally blocked by planning-only boundary |

The units are non-overlapping and collectively cover implementation, behavior, completion, and verification.
