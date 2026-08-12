# Verification

Run `benchmark/planning/setup-and-run.sh pilot-142 --sequential --iterative --revisions v1.3.1,v1.4.1` and the matching `--fresh-review` command in a temporary fixture. Pass when both modes parse, malformed combinations fail before process launch, and the selected tags/limits are recorded in run metadata.

