---
name: nuxtpolymarket-deviations
description: Project-specific Nuxtpolymarket behavior to load before running tests or making framework-default assumptions.
---
<!-- MODE: DEV -->
<!-- PACKAGE: DEV -->

# Test environment

The Vitest global setup calls Node's `process.loadEnvFile`. Run Vitest through the installed entry with Node (`node node_modules/vitest/vitest.mjs run ...`); forcing the Vitest process under Bun fails in setup before test modules load because Bun does not provide that API.

Database-backed Vitest suites require PostgreSQL listening on localhost:5432; without it, balance and exchange cleanup/setup fail with `ECONNREFUSED` before their assertions run.

# Database schema

The project applies Drizzle schema changes with `bun run db:push` and does not
keep generated migration history in the repository. Do not commit a generated
initial migration when changing `server/database/schema.ts`.

# Visual QA artifacts

Temporary screenshots, Playwright captures, traces, videos, and diagnostic
images belong under `/tmp`, not in the repository. Load the project-local
`temporary-screenshot-output` skill whenever choosing visual-QA output paths.
