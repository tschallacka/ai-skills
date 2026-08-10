# UI story run/cache: button-chain

## Run metadata

- Run status: `not executed — planning-only proof`
- Intended target: future root-level `button-chain.html`
- Required environment: clean browser page, no server required
- Evidence source: future browser observation recorded by the implementation agent

## Expected cached sequence

| Event | Generated count | Total buttons | Expected result |
|---|---:|---:|---|
| Load | 0 | 1 | One initial button |
| Click current last | 1 | 2 | One button appended below |
| Click current last | 2 | 3 | One button appended below |
| Click current last | 3 | 4 | One button appended below |
| Click current last | 4 | 5 before action | Document clears; render exact `finished` with visible white border |

No actual UI run, browser, screenshot, DOM inspection, or HTML artifact exists
from this proof. A future run must replace this expected cache with observed
pass/fail evidence and preserve the same sequence.
