# Step: Add the /health test (W02)

## Change target
- File: `test/app.test.js`
- Symbol: `/health` test

## Objective
Assert the `/health` endpoint returns HTTP 200 with the expected body.

## Acceptance criteria
- A test hits `GET /health` and asserts 200.
- The test asserts the body equals `{"status":"ok"}`.
