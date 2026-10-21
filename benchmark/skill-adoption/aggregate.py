#!/usr/bin/env python3
"""runs.jsonl -> aggregate.json + AGGREGATE.md.

Usage: aggregate.py RUNS_JSONL [OUT_DIR]

Pure function of RUNS_JSONL and the scenario manifests under scenarios/ --
see DATA-SPEC.md's "Design principle". OUT_DIR defaults to RUNS_JSONL's own
directory. Writes aggregate.json (schema/aggregate.schema.json) and a
human-readable AGGREGATE.md twin.
"""
import hashlib
import json
import pathlib
import sys
from datetime import datetime, timezone

HERE = pathlib.Path(__file__).parent.resolve()


def wilson_interval(k, n, z=1.96):
    """95% Wilson score interval for k successes out of n trials. Chosen
    over the normal (Wald) approximation because this benchmark's default
    rep count is tiny (2 per cell) -- see DATA-SPEC.md's Metrics section."""
    if n == 0:
        return (0.0, 0.0)
    p = k / n
    denom = 1 + z * z / n
    center = p + z * z / (2 * n)
    spread = z * ((p * (1 - p) / n + z * z / (4 * n * n)) ** 0.5)
    lo = (center - spread) / denom
    hi = (center + spread) / denom
    return (max(0.0, lo), min(1.0, hi))


def load_scenario_targets():
    """scenario_id -> list of target_skills, for the by_skill rollup."""
    out = {}
    for f in (HERE / "scenarios").glob("*/scenario.json"):
        doc = json.loads(f.read_text())
        out[doc["id"]] = doc.get("target_skills", [])
    return out


def compute_cell(key, runs):
    n = len(runs)
    adopted = [r for r in runs if r["adoption"]["adopted"]]
    # consideration is a newer, required-as-of-schema-1.0.0 field; .get()
    # keeps this readable against an older run record someone re-aggregates
    # without first re-running it (considered_n is then just 0 for it).
    considered = [r for r in runs if r.get("consideration", {}).get("considered")]
    correct = [r for r in runs if r["regression"]["task_correct"]]
    adopted_n = len(adopted)
    considered_n = len(considered)
    correct_n = len(correct)

    adoption_rate = adopted_n / n if n else 0.0
    consideration_rate = considered_n / n if n else 0.0
    regression_rate = (n - correct_n) / n if n else 0.0

    adopted_incorrect = len([r for r in adopted if not r["regression"]["task_correct"]])
    unadopted = [r for r in runs if not r["adoption"]["adopted"]]
    unadopted_incorrect = len([r for r in unadopted if not r["regression"]["task_correct"]])

    regression_rate_adopted = (adopted_incorrect / adopted_n) if adopted_n else None
    regression_rate_unadopted = (unadopted_incorrect / len(unadopted)) if unadopted else None

    dirty = len([r for r in runs if r.get("environment", {}).get("worktree_dirty")])

    return {
        "key": key,
        "n": n,
        "adopted_n": adopted_n,
        "adoption_rate": adoption_rate,
        "adoption_ci95": list(wilson_interval(adopted_n, n)),
        "considered_n": considered_n,
        "consideration_rate": consideration_rate,
        "consideration_ci95": list(wilson_interval(considered_n, n)),
        "correct_n": correct_n,
        "regression_rate": regression_rate,
        "regression_rate_ci95": list(wilson_interval(n - correct_n, n)),
        "regression_rate_adopted": regression_rate_adopted,
        "regression_rate_unadopted": regression_rate_unadopted,
        "worktree_dirty_fraction": (dirty / n) if n else 0.0,
    }


def main():
    if len(sys.argv) not in (2, 3):
        sys.exit("usage: aggregate.py RUNS_JSONL [OUT_DIR]")
    runs_path = pathlib.Path(sys.argv[1])
    out_dir = pathlib.Path(sys.argv[2]) if len(sys.argv) == 3 else runs_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)

    raw = runs_path.read_bytes()
    runs = [json.loads(line) for line in raw.decode("utf-8").splitlines() if line.strip()]

    by_cell = {}
    for r in runs:
        key = f"{r['scenario_id']}/{r['cli']}/{r['mode']}"
        by_cell.setdefault(key, []).append(r)
    cells = [compute_cell(k, v) for k, v in sorted(by_cell.items())]

    targets = load_scenario_targets()
    by_skill_runs = {}
    for r in runs:
        for skill in targets.get(r["scenario_id"], []):
            by_skill_runs.setdefault(skill, []).append(r)
    by_skill = [compute_cell(k, v) for k, v in sorted(by_skill_runs.items())]

    aggregate = {
        "schema_version": "1.0.0",
        "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source_runs_file": str(runs_path),
        "source_runs_sha256": hashlib.sha256(raw).hexdigest(),
        "n_runs": len(runs),
        "cells": cells,
        "by_skill": by_skill,
    }

    (out_dir / "aggregate.json").write_text(json.dumps(aggregate, indent=2) + "\n", encoding="utf-8")
    write_markdown(aggregate, out_dir / "AGGREGATE.md")
    print(f"aggregate.py: wrote {out_dir / 'aggregate.json'} ({len(runs)} runs, {len(cells)} cells)")


def fmt_rate(x):
    return "n/a" if x is None else f"{x:.2f}"


def write_markdown(aggregate, path):
    lines = [
        "<!-- MODE: DEV -->",
        "# Skill-adoption aggregate",
        "",
        f"Generated {aggregate['generated_at_utc']} from `{aggregate['source_runs_file']}` "
        f"(sha256 `{aggregate['source_runs_sha256'][:16]}...`), {aggregate['n_runs']} runs.",
        "",
        "Regenerate with `python3 aggregate.py <runs.jsonl>` -- this file and "
        "`aggregate.json` are pure functions of that one input; see DATA-SPEC.md.",
        "",
        "A rate from `n < 5` is a pilot number, not a finding -- see DATA-SPEC.md's "
        "Metrics section before citing one as settled.",
        "",
        "Adoption and consideration are the headline numbers this pass cares "
        "about; regression/correctness is reported alongside them but is "
        "not the point of this benchmark -- see DATA-SPEC.md's Metrics "
        "section on `consideration`.",
        "",
        "## By skill",
        "",
        "| skill | n | adoption_rate (95% CI) | consideration_rate (95% CI) | regression_rate | regression\\|adopted | regression\\|unadopted | dirty tree |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for c in aggregate["by_skill"]:
        ci = c["adoption_ci95"]
        cci = c["consideration_ci95"]
        lines.append(
            f"| {c['key']} | {c['n']} | {c['adoption_rate']:.2f} "
            f"[{ci[0]:.2f}, {ci[1]:.2f}] | {c['consideration_rate']:.2f} "
            f"[{cci[0]:.2f}, {cci[1]:.2f}] | {c['regression_rate']:.2f} | "
            f"{fmt_rate(c['regression_rate_adopted'])} | "
            f"{fmt_rate(c['regression_rate_unadopted'])} | "
            f"{c['worktree_dirty_fraction']:.0%} |"
        )
    lines += ["", "## By cell (scenario / cli / mode)", "",
              "| cell | n | adoption_rate (95% CI) | consideration_rate (95% CI) | regression_rate | regression\\|adopted | regression\\|unadopted |",
              "|---|---|---|---|---|---|---|"]
    for c in aggregate["cells"]:
        ci = c["adoption_ci95"]
        cci = c["consideration_ci95"]
        lines.append(
            f"| {c['key']} | {c['n']} | {c['adoption_rate']:.2f} "
            f"[{ci[0]:.2f}, {ci[1]:.2f}] | {c['consideration_rate']:.2f} "
            f"[{cci[0]:.2f}, {cci[1]:.2f}] | {c['regression_rate']:.2f} | "
            f"{fmt_rate(c['regression_rate_adopted'])} | "
            f"{fmt_rate(c['regression_rate_unadopted'])} |"
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
