| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|
| W01 | source | `app.js` | `addHealthHandler` | handler | Add `GET /health` handler returning `{"status":"ok"}`. | — | 01-health-endpoint | 01-step-add-handler |
| W02 | test | `test/app.test.js` | `/health` test | test | Assert 200 and body on `/health`. | W01 | 01-health-endpoint | 02-step-add-test |
