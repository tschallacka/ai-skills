"""Builds a side-by-side comparison report from two finished condition runs
(treatment vs baseline) under the same run-id.

Usage: compare.py <results-base-dir> <run-id>
Writes <results-base-dir>/<run-id>/comparison.md and prints it to stdout.
"""

import json
import sys
from pathlib import Path


def load_json(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError):
        return {}


def load_text(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except OSError:
        return ""


def condition_report(case_dir):
    telemetry = load_json(case_dir / "telemetry.json")
    score = load_json(case_dir / "score.json")
    external_edit_log = load_text(case_dir / "external-edit.log")
    external_format_log = load_text(case_dir / "external-format.log")
    worker_stderr = load_text(case_dir / "worker-stderr.log")

    wall_seconds = None
    worker_exit = None
    eval_text = load_text(case_dir / "evaluation.md")
    for line in eval_text.splitlines():
        if line.startswith("- Wall clock:"):
            wall_seconds = line.split(":", 1)[1].strip()
        if line.startswith("- Worker exit code:"):
            worker_exit = line.split(":", 1)[1].strip()

    return {
        "worker_exit": worker_exit,
        "wall_seconds": wall_seconds,
        "telemetry": telemetry,
        "score": score,
        "external_edit_log": external_edit_log,
        "external_format_log": external_format_log,
        "worker_stderr_tail": "\n".join(worker_stderr.splitlines()[-10:]),
    }


def row(label, treatment_value, baseline_value):
    return f"| {label} | {treatment_value} | {baseline_value} |"


def main():
    if len(sys.argv) != 3:
        print("Usage: compare.py <results-base-dir> <run-id>", file=sys.stderr)
        return 64

    results_base = Path(sys.argv[1])
    run_id = sys.argv[2]
    run_dir = results_base / run_id

    treatment = condition_report(run_dir / "treatment")
    baseline = condition_report(run_dir / "baseline")

    t_score = treatment["score"]
    b_score = baseline["score"]
    t_tel = treatment["telemetry"]
    b_tel = baseline["telemetry"]

    lines = [
        f"# ai-text-editor-usage comparison: {run_id}",
        "",
        "| metric | treatment (ai-text-editor) | baseline (stock tools) |",
        "|---|---|---|",
        row("worker exit code", treatment["worker_exit"], baseline["worker_exit"]),
        row("wall clock", treatment["wall_seconds"], baseline["wall_seconds"]),
        row(
            "total tokens (in+cache+out)",
            t_tel.get("total_usage_tokens", "n/a"),
            b_tel.get("total_usage_tokens", "n/a"),
        ),
        row("output tokens", t_tel.get("output_tokens", "n/a"), b_tel.get("output_tokens", "n/a")),
        row(
            "thinking tokens",
            t_tel.get("thinking_tokens", "n/a"),
            b_tel.get("thinking_tokens", "n/a"),
        ),
        row(
            "tool calls (total)",
            t_tel.get("tool_calls_total", "n/a"),
            b_tel.get("tool_calls_total", "n/a"),
        ),
        row(
            "tool calls (by name)",
            json.dumps(t_tel.get("tool_calls_by_name", {})),
            json.dumps(b_tel.get("tool_calls_by_name", {})),
        ),
        row(
            "subagent (Task) dispatches",
            t_tel.get("subagent_dispatch_count", "n/a"),
            b_tel.get("subagent_dispatch_count", "n/a"),
        ),
        row(
            "subagent/sidechain tokens",
            t_tel.get("subagent_sidechain", {}).get("total_usage_tokens", "n/a"),
            b_tel.get("subagent_sidechain", {}).get("total_usage_tokens", "n/a"),
        ),
        row(
            "subagent/sidechain tool calls (by name)",
            json.dumps(t_tel.get("subagent_sidechain", {}).get("tool_calls_by_name", {})),
            json.dumps(b_tel.get("subagent_sidechain", {}).get("tool_calls_by_name", {})),
        ),
        row(
            "adopted ai-text-editor unprompted",
            t_tel.get("ai_text_editor_adoption", {}).get("adopted", "n/a"),
            b_tel.get("ai_text_editor_adoption", {}).get("adopted", "n/a"),
        ),
        row(
            "ai-text-editor calls (total)",
            t_tel.get("ai_text_editor_adoption", {}).get("total_calls", "n/a"),
            b_tel.get("ai_text_editor_adoption", {}).get("total_calls", "n/a"),
        ),
        row(
            "first ai-text-editor call at tool-call index",
            t_tel.get("ai_text_editor_adoption", {}).get("first_call_sequence_index", "n/a"),
            b_tel.get("ai_text_editor_adoption", {}).get("first_call_sequence_index", "n/a"),
        ),
        row(
            "stock-tool calls before adoption",
            json.dumps(t_tel.get("ai_text_editor_adoption", {}).get("stock_tool_calls_before_adoption", {})),
            json.dumps(b_tel.get("ai_text_editor_adoption", {}).get("stock_tool_calls_before_adoption", {})),
        ),
        row(
            "text/thinking mentions of ai-text-editor",
            t_tel.get("ai_text_editor_text_mentions", {}).get("count", "n/a"),
            b_tel.get("ai_text_editor_text_mentions", {}).get("count", "n/a"),
        ),
        row(
            "all three edits correct",
            t_score.get("all_three_edits_correct", "n/a"),
            b_score.get("all_three_edits_correct", "n/a"),
        ),
        row(
            "legacy_log fully replaced",
            t_score.get("legacy_log_fully_replaced", "n/a"),
            b_score.get("legacy_log_fully_replaced", "n/a"),
        ),
        row(
            "calculate_discount defined correctly",
            t_score.get("calculate_discount_plausible_body", "n/a"),
            b_score.get("calculate_discount_plausible_body", "n/a"),
        ),
        row(
            "apply_tax has docstring",
            t_score.get("apply_tax_has_docstring", "n/a"),
            b_score.get("apply_tax_has_docstring", "n/a"),
        ),
        row(
            "COLLISION 1 -- TAX_RATE bump preserved (collision-safe)",
            t_score.get("tax_rate_external_edit_preserved", "n/a"),
            b_score.get("tax_rate_external_edit_preserved", "n/a"),
        ),
        row(
            "COLLISION 1 -- TAX_RATE silently reverted",
            t_score.get("tax_rate_reverted_to_original", "n/a"),
            b_score.get("tax_rate_reverted_to_original", "n/a"),
        ),
        row(
            "COLLISION 2 -- whole-file reindent preserved",
            t_score.get("external_reformat_preserved", "n/a"),
            b_score.get("external_reformat_preserved", "n/a"),
        ),
        row(
            "COLLISION 2 -- whole-file reindent clobbered",
            t_score.get("external_reformat_clobbered", "n/a"),
            b_score.get("external_reformat_clobbered", "n/a"),
        ),
        row(
            "apply_bulk_discount body indent (3=pre-reindent, 4=post)",
            t_score.get("apply_bulk_discount_body_indent_spaces", "n/a"),
            b_score.get("apply_bulk_discount_body_indent_spaces", "n/a"),
        ),
        row(
            "shipped regression test: passes",
            t_score.get("tests_pass", "n/a"),
            b_score.get("tests_pass", "n/a"),
        ),
        row(
            "regression test updated to match new TAX_RATE (good)",
            t_score.get("test_updated_to_new_rate", "n/a"),
            b_score.get("test_updated_to_new_rate", "n/a"),
        ),
        row(
            "regression test left pinned to stale rate",
            t_score.get("test_pinned_to_original_rate", "n/a"),
            b_score.get("test_pinned_to_original_rate", "n/a"),
        ),
        row(
            "final file still valid python",
            t_score.get("valid_python", "n/a"),
            b_score.get("valid_python", "n/a"),
        ),
        "",
        "## ai-text-editor mentions in reasoning (treatment)",
        "Snippets of assistant text/thinking blocks mentioning the tool by name --",
        "whether or not it was actually called. Qualitative input for tuning the",
        "tool's own description/guidance text, not a correctness check.",
        "```",
        "\n---\n".join(t_tel.get("ai_text_editor_text_mentions", {}).get("snippets", [])) or "(none)",
        "```",
        "",
        "## Regression test output (treatment)",
        "```",
        t_score.get("test_output", "(n/a)") or "(empty)",
        "```",
        "",
        "## Regression test output (baseline)",
        "```",
        b_score.get("test_output", "(n/a)") or "(empty)",
        "```",
        "",
        "## External edit log -- TAX_RATE bump (treatment)",
        "```",
        treatment["external_edit_log"].strip() or "(empty)",
        "```",
        "",
        "## External edit log -- TAX_RATE bump (baseline)",
        "```",
        baseline["external_edit_log"].strip() or "(empty)",
        "```",
        "",
        "## External edit log -- whole-file reindent (treatment)",
        "```",
        treatment["external_format_log"].strip() or "(empty)",
        "```",
        "",
        "## External edit log -- whole-file reindent (baseline)",
        "```",
        baseline["external_format_log"].strip() or "(empty)",
        "```",
    ]

    report = "\n".join(lines) + "\n"
    (run_dir / "comparison.md").write_text(report, encoding="utf-8")
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
