Analyze the completed planning benchmark batch `{{RUN_ID}}`.

You may inspect only:

- `{{ANALYSIS_ROOT}}/benchmark-test.md`
- `{{ANALYSIS_ROOT}}/harness-summary.tsv`
- result archives under `{{RESULTS_ROOT}}` whose run id is `{{RUN_ID}}`

Write the comparison report to:

```text
{{RESULTS_ROOT}}/comparison-{{RUN_ID}}.md
```

Include one row per revision and separate worker exit status, validation
result, HTML/HTM artifact audit, session UUID, telemetry records, token total
or unavailable status, accepted/tainted status, and taint reasons.

Do not repair worker artifacts and do not fill missing telemetry from other
sessions.
