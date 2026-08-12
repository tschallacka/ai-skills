# Step: Add the /health handler (W01)

## Change target
- File: `app.js`
- Symbol: `addHealthHandler`

## Objective
Return `{"status":"ok"}` with HTTP 200 from `GET /health`.

## Acceptance criteria
- `app.js` registers a `/health` route.
- The response body is exactly `{"status":"ok"}`.
