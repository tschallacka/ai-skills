# Plan: Add a /health endpoint

## Current state
A tiny example service with a single `/` route.

## Desired outcome
A `/health` endpoint that returns `{"status":"ok"}` with HTTP 200.

## Approach
Add one route handler and one test.

## Scope
In scope: the handler and its test. Out of scope: auth, metrics, deployment.

## UI classification
- UI affected: no
