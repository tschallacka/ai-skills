# Atomic work-unit inventory

| ID | Owner/outcome | Depends on | Evidence required | Status |
|---|---|---|---|---|
| WU-01 | Define exact scope and terminal semantics | Task specification | plan-description.md | ✅ planned |
| WU-02 | Create one initial button and count state | WU-01 | implementation diff and DOM count | 💤 future |
| WU-03 | Gate clicks to current last button | WU-02 | non-last-button check | 💤 future |
| WU-04 | Append exactly one button below last | WU-03 | counts 2/3/4/5 | 💤 future |
| WU-05 | Trigger terminal action on generated button 4 | WU-04 | zero-button DOM observation | 💤 future |
| WU-06 | Render exact `finished` with visible white border | WU-05 | text/style observation | 💤 future |
| WU-07 | Run UI story and cache evidence | WU-02..WU-06 | ui-story-runs/button-chain.md | 💤 future |
| WU-08 | Review, resolve bugs, and hand off | WU-07 | adversarial-review.md and bug-register.md | ✅ planned |

No work unit authorizes HTML creation in this planning proof.
